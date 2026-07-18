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
}

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

fn fetch_billing(days: u32) -> Result<BillingData, Error> {
    let end = today_utc() + chrono::Duration::days(1);
    let start = end - chrono::Duration::days(days as i64);

    let start_str = start.format("%Y-%m-%d").to_string();
    let end_str = end.format("%Y-%m-%d").to_string();

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

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(&stdout)?;

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

fn main() {
    let args = Args::parse();
    let thresholds = load_thresholds();

    println!("Fetching AWS billing data for last {} days...", args.days);
    match fetch_billing(args.days) {
        Ok(data) => {
            if data.days.is_empty() {
                eprintln!("No billing data returned. Check your AWS config and Cost Explorer permissions.");
                std::process::exit(1);
            }
            println!();
            print_billing(&data, thresholds);
        }
        Err(e) => {
            eprintln!("Failed to fetch billing data: {}", e);
            std::process::exit(1);
        }
    }
}
