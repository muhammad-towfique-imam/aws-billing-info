# aws-billing-info

A minimal CLI tool that fetches and displays AWS billing data for the last N days using the AWS CLI. Outputs color-coded daily breakdowns with configurable amount thresholds.

## Prerequisites

1. **AWS CLI** — install and configure it:
   ```bash
   # macOS
   brew install awscli && aws configure

   # Linux
   curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o awscliv2.zip
   unzip awscliv2.zip && sudo ./aws/install && aws configure
   ```
   See [official docs](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) for other platforms.

2. **AWS Cost Explorer** — enable it in the [AWS Console](https://console.aws.amazon.com/cost-management/home#/ce) if you haven't already. It may take 24 hours after first enabling to show data.

3. **IAM permissions** — your AWS user/role needs `ce:GetCostAndUsage` (and optionally `ce:GetDimensionValues`). Example policy:
   ```json
   {
     "Version": "2012-10-17",
     "Statement": [
       {
         "Effect": "Allow",
         "Action": ["ce:GetCostAndUsage", "ce:GetDimensionValues"],
         "Resource": "*"
       }
     ]
   }
   ```

## Installation

```bash
curl -LsSf https://raw.githubusercontent.com/muhammad-towfique-imam/aws-billing-info/main/install.sh | sh
```

The installer asks if it must show the billing summary when a shell starts (see [Show a summary when a shell starts](#show-a-summary-when-a-shell-starts)). To answer without the question, set `AWS_BILLING_SUMMARY`:

```bash
curl -LsSf https://raw.githubusercontent.com/muhammad-towfique-imam/aws-billing-info/main/install.sh | AWS_BILLING_SUMMARY=yes sh
```

## Uninstall

```bash
curl -LsSf https://raw.githubusercontent.com/muhammad-towfique-imam/aws-billing-info/main/uninstall.sh | sh
```

The uninstaller removes the binary, the shell summary line from your shell rc files, and the summary cache. It does not change your `PATH` line.

## Usage

```bash
# Default: show only total
aws-billing-info

# Show detailed daily breakdown
aws-billing-info --detailed

# Custom days with details
aws-billing-info --days 14 --detailed
aws-billing-info -d 30 --detailed
```

## Show a summary when a shell starts

`--summary` prints one line for the last 7 days (or `--days N`). The daily totals come after the average, newest day first:

```
AWS billing (last 7 days): $6.32 total · avg $0.90/day · $0.10 $0.20 $0.13 $0.40 $0.23 $0.22 $0.11
```

If you answer `y` to the installer question, the installer adds the line for your shell. It adds the line to `~/.bashrc` (`~/.bash_profile` on macOS), `~/.zshrc`, or `~/.config/fish/config.fish`.

To add the line yourself, add this line to `~/.bashrc` or `~/.zshrc`:

```bash
[[ $- == *i* ]] && command -v aws-billing-info >/dev/null && aws-billing-info --summary 2>/dev/null
```

For fish, add this to `~/.config/fish/config.fish`:

```fish
status is-interactive; and type -q aws-billing-info; and aws-billing-info --summary 2>/dev/null
```

AWS charges $0.01 for each Cost Explorer API request. Thus `--summary` keeps the response in `~/.cache/aws-billing-info/` for 6 hours. A new shell reads the cache and starts with no delay. Only the first shell after the cache expires waits for the AWS call (approximately 2 seconds).

## Output

### Default (daily totals)

```
Fetching AWS billing data for last 7 days...

12 July 2026
 - Total: $1.02

13 July 2026
 - Total: $1.02

14 July 2026
 - Total: $1.02
```

### Detailed (`--detailed`)

```
Fetching AWS billing data for last 7 days...

12 July 2026
 - AWS Glue: $0.00
 - AWS Key Management Service: $0.00
 - EC2 - Other: $0.02
 - Amazon Elastic Compute Cloud - Compute: $0.00
 - Amazon Lightsail: $0.23
 - Amazon Relational Database Service: $0.51
 - Amazon Simple Storage Service: $0.02
 - Amazon Virtual Private Cloud: $0.24
 - AmazonCloudWatch: $0.00
 - Total: $1.02

13 July 2026
 - AWS Glue: $0.00
 - EC2 - Other: $0.02
 - Amazon Elastic Compute Cloud - Compute: $0.00
 - Amazon Lightsail: $0.23
 - Amazon Relational Database Service: $0.51
 - Amazon Simple Storage Service: $0.02
 - Amazon Virtual Private Cloud: $0.24
 - AmazonCloudWatch: $0.00
 - Total: $1.02

14 July 2026
 - AWS Glue: $0.00
 - EC2 - Other: $0.02
 - Amazon Elastic Compute Cloud - Compute: $0.00
 - Amazon Lightsail: $0.23
 - Amazon Relational Database Service: $0.51
 - Amazon Simple Storage Service: $0.02
 - Amazon Virtual Private Cloud: $0.24
 - AmazonCloudWatch: $0.00
 - Total: $1.02
```

## Color Thresholds

Amounts are color-coded based on configurable thresholds:

- **Green**: amount below threshold
- **Orange**: amount at or above green threshold, below orange threshold
- **Red**: amount at or above orange threshold

Default thresholds:
- **Line items**: green < $1.00, orange < $2.00
- **Totals**: green < $2.00, orange < $4.00

## Configuration

Create a `config.toml` file in the project root or at `~/.config/aws-billing-info/config.toml` to override thresholds:

```toml
[thresholds.total]
green = 2.0
orange = 4.0

[thresholds.line_items]
green = 1.0
orange = 2.0
```

## How It Works

The tool calls `aws ce get-cost-and-usage` with daily granularity, grouped by service. It parses the JSON output and displays each day's services and totals in a minimal, color-coded format.
