# aws-billing-info

A minimal CLI tool that fetches and displays AWS billing data for the last N days using the AWS CLI. Outputs color-coded daily breakdowns with configurable amount thresholds.

## Prerequisites

- [AWS CLI](https://aws.amazon.com/cli/) installed and configured
- AWS Cost Explorer permissions enabled (`aws ce get-cost-and-usage`)

## Installation

```bash
curl -LsSf https://raw.githubusercontent.com/muhammad-towfique-imam/aws-billing-info/main/install.sh | sh
```

Custom install directory:
```bash
curl -LsSf https://raw.githubusercontent.com/muhammad-towfique-imam/aws-billing-info/main/install.sh | INSTALL_DIR=/usr/local/bin sh
```

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
