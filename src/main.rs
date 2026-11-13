use std::process::Command;
use std::path::PathBuf;

use clap::Parser;
use dirs::config_dir;
use chrono::Datelike;
use serde_json::Value;
use toml::Value as TomlValue;

#[derive(Parser, Debug)]
#[command(author, version, about = "Minimal TUI for AWS billing", long_about = None)]
struct Args {
    /// Number of days to look back (default: 7)
    #[arg(short = 'd', long = "days", default_value = "7")]
    days: u32,
    /// Show detailed daily breakdown (default: show only total)
    #[arg(long = "detailed")]
    detailed: bool,
    /// Print one summary line (for shell startup). Uses a cache to limit AWS API calls.
    #[arg(long = "summary", conflicts_with = "detailed")]
    summary: bool,
}

/// Cost Explorer charges $0.01 per API request, so summary mode reuses a cached response.
const SUMMARY_CACHE_TTL_SECS: u64 = 6 * 60 * 60;

#[derive(Debug, Clone)]
struct DailyCost {
    formatted_date: String,
    services: Vec<(String, f64)>,
}

#[derive(Debug, Clone)]
struct BillingData {
    days: Vec<DailyCost>,
}

#[derive(Debug, Clone, Copy)]
struct Thresholds {
    line_item_green: f64,
    line_item_orange: f64,
    total_green: f64,
    total_orange: f64,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("AWS CLI error: {0}")]
    AwsCli(String),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            line_item_green: 1.0,
            line_item_orange: 2.0,
            total_green: 2.0,
            total_orange: 4.0,
        }
    }
}

fn load_thresholds() -> Thresholds {
    let mut thresholds = Thresholds::default();

    let candidates = vec![
        PathBuf::from("config.toml"),
        config_dir()
            .unwrap_or_default()
            .join("aws-billing-info")
            .join("config.toml"),
    ];

    for path in candidates {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(config) = content.parse::<TomlValue>() {
                if let Some(t) = config.get("thresholds").and_then(|v| v.as_table()) {
                    if let Some(total) = t.get("total").and_then(|v| v.as_table()) {
                        if let Some(v) = total.get("green").and_then(|v| v.as_float()) {
                            thresholds.total_green = v;
                        }
                        if let Some(v) = total.get("orange").and_then(|v| v.as_float()) {
                            thresholds.total_orange = v;
                        }
                    }
                    if let Some(line_items) = t.get("line_items").and_then(|v| v.as_table()) {
                        if let Some(v) = line_items.get("green").and_then(|v| v.as_float()) {
                            thresholds.line_item_green = v;
                        }
                        if let Some(v) = line_items.get("orange").and_then(|v| v.as_float()) {
                            thresholds.line_item_orange = v;
                        }
                    }
                }
            }
        }
    }

    thresholds
}

fn color_for_amount(amount: f64, green_threshold: f64, orange_threshold: f64) -> &'static str {
    if amount < green_threshold {
        "\x1b[1;32m"
    } else if amount < orange_threshold {
        "\x1b[1;33m"
    } else {
        "\x1b[1;31m"
    }
}

fn today_utc() -> chrono::NaiveDate {
    let now = chrono::Utc::now();
    now.date_naive()
}

fn time_period(days: u32) -> (String, String) {
    let end = today_utc() + chrono::Duration::days(1);
    let start = end - chrono::Duration::days(days as i64);
    (
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string(),
    )
}

fn fetch_raw(start_str: &str, end_str: &str) -> Result<String, Error> {
    let output = Command::new("aws")
        .args([
            "ce",
            "get-cost-and-usage",
            "--time-period",
            &format!("Start={},End={}", start_str, end_str),
            "--granularity",
            "DAILY",
            "--metrics",
            "BlendedCost",
            "--group-by",
            "Type=DIMENSION,Key=SERVICE",
        ])
        .output()
        .map_err(|e| Error::AwsCli(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::AwsCli(stderr.into_owned()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Return the AWS response from the cache if it is fresh. Otherwise fetch it and update the cache.
fn fetch_raw_cached(start_str: &str, end_str: &str) -> Result<String, Error> {
    let cache_path = dirs::cache_dir().map(|d| {
        d.join("aws-billing-info")
            .join(format!("ce-{}-{}.json", start_str, end_str))
    });

    if let Some(path) = &cache_path {
        let fresh = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .map(|age| age.as_secs() < SUMMARY_CACHE_TTL_SECS)
            .unwrap_or(false);
        if fresh {
            if let Ok(content) = std::fs::read_to_string(path) {
                return Ok(content);
            }
        }
    }

    let raw = fetch_raw(start_str, end_str)?;

    if let Some(path) = &cache_path {
        if let Some(dir) = path.parent() {
            if std::fs::create_dir_all(dir).is_ok() {
                // Remove old cache files for other time periods.
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if entry.path() != *path {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
                let _ = std::fs::write(path, &raw);
            }
        }
    }

    Ok(raw)
}

fn fetch_billing(days: u32, use_cache: bool) -> Result<BillingData, Error> {
    let (start_str, end_str) = time_period(days);
    let raw = if use_cache {
        fetch_raw_cached(&start_str, &end_str)?
    } else {
        fetch_raw(&start_str, &end_str)?
    };
    let v: Value = serde_json::from_str(&raw)?;

    let mut days_map: std::collections::BTreeMap<String, Vec<(String, f64)>> =
        std::collections::BTreeMap::new();

    if let Some(results) = v.get("ResultsByTime").and_then(|r| r.as_array()) {
        for result in results {
            let date_str = result
                .get("TimePeriod")
                .and_then(|t| t.get("Start"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();

            if let Some(groups) = result.get("Groups").and_then(|g| g.as_array()) {
                for group in groups {
                    let service = group
                        .get("Keys")
                        .and_then(|k| k.as_array())
                        .and_then(|a| a.first())
                        .and_then(|s| s.as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let amount = group
                        .get("Metrics")
                        .and_then(|m| m.get("BlendedCost"))
                        .and_then(|b| b.get("Amount"))
                        .and_then(|a| a.as_str())
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0);

                    days_map.entry(date_str.clone()).or_default().push((service, amount));
                }
            }
        }
    }

    let mut days = Vec::new();

    for (date_str, services) in days_map {
        let date_obj =
            chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").expect("Invalid date format");
        let formatted_date = format!(
            "{} {} {}",
            date_obj.day(),
            date_obj.format("%B"),
            date_obj.year()
        );
        days.push(DailyCost {
            formatted_date,
            services,
        });
    }

    Ok(BillingData { days })
}

fn print_daily_totals(data: &BillingData, thresholds: Thresholds) {
    for (day_idx, day) in data.days.iter().enumerate() {
        if day_idx > 0 {
            println!();
        }
        println!("\x1b[1;36m{}\x1b[0m", day.formatted_date);
        let day_total: f64 = day.services.iter().map(|(_, amt)| *amt).sum();
        let color = color_for_amount(day_total, thresholds.total_green, thresholds.total_orange);
        println!(" - Total: {}{:.2}\x1b[0m", color, day_total);
    }
}

fn print_billing(data: &BillingData, thresholds: Thresholds) {
    for (day_idx, day) in data.days.iter().enumerate() {
        if day_idx > 0 {
            println!();
        }
        println!("\x1b[1;36m{}\x1b[0m", day.formatted_date);
        for (svc, amt) in &day.services {
            let color = color_for_amount(*amt, thresholds.line_item_green, thresholds.line_item_orange);
            println!(" - {}: {}{:.2}\x1b[0m", svc, color, amt);
        }
        if day.services.is_empty() {
            println!(" - No data: \x1b[1;32m$0.00\x1b[0m");
        }
        let day_total: f64 = day.services.iter().map(|(_, amt)| *amt).sum();
        let color = color_for_amount(day_total, thresholds.total_green, thresholds.total_orange);
        println!(" - Total: {}{:.2}\x1b[0m", color, day_total);
    }
}

fn print_summary(data: &BillingData, days: u32, thresholds: Thresholds) {
    let totals: Vec<f64> = data
        .days
        .iter()
        .map(|d| d.services.iter().map(|(_, amt)| *amt).sum())
        .collect();
    let total: f64 = totals.iter().sum();
    let avg = total / days.max(1) as f64;
    let avg_color = color_for_amount(avg, thresholds.total_green, thresholds.total_orange);
    // Newest day first, so today's partial amount comes first.
    let daily: Vec<String> = totals
        .iter()
        .rev()
        .map(|t| {
            let color = color_for_amount(*t, thresholds.total_green, thresholds.total_orange);
            format!("{}${:.2}\x1b[0m", color, t)
        })
        .collect();
    println!(
        "\x1b[1;36mAWS billing (last {} days):\x1b[0m {}${:.2}\x1b[0m total · avg {}${:.2}\x1b[0m/day · {}",
        days,
        avg_color,
        total,
        avg_color,
        avg,
        daily.join(" ")
    );
}

fn main() {
    let args = Args::parse();
    let thresholds = load_thresholds();

    if args.summary {
        match fetch_billing(args.days, true) {
            Ok(data) => print_summary(&data, args.days, thresholds),
            Err(e) => {
                eprintln!("aws-billing-info: {}", e.to_string().trim());
                std::process::exit(1);
            }
        }
        return;
    }

    println!("Fetching AWS billing data for last {} days...", args.days);
    match fetch_billing(args.days, false) {
        Ok(data) => {
            if data.days.is_empty() {
                eprintln!("No billing data returned. Check your AWS config and Cost Explorer permissions.");
                std::process::exit(1);
            }
            println!();
            if args.detailed {
                print_billing(&data, thresholds);
            } else {
                print_daily_totals(&data, thresholds);
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch billing data: {}", e);
            std::process::exit(1);
        }
    }
}
