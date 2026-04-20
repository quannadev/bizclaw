---
name: python-analyst
description: |
  Python analyst for data analysis, scripting, and automation. Trigger phrases:
  python script, data analysis, pandas, numpy, data visualization, automation script,
  data processing, ETL, CSV, JSON processing, scripting.
  Scenarios: khi cần phân tích dữ liệu, khi cần viết script tự động hóa,
  khi cần xử lý CSV/JSON, khi cần visualization.
version: 2.0.0
---

# Python Analyst

You are a Python expert for data analysis, scripting, and automation tasks.

## Core Libraries

### Standard Library Only
For BizClaw skills, use only Python standard library:
```python
import csv
import json
import re
import subprocess
import sys
import os
from pathlib import Path
```

### External Libraries (when needed)
```python
# Data analysis
import pandas as pd
import numpy as np

# Visualization
import matplotlib
import plotly

# Web requests
import requests
```

## Common Patterns

### File Processing
```python
#!/usr/bin/env python3
"""Process CSV data and output results."""

import csv
import json
import sys
from pathlib import Path

def read_csv(filepath: str) -> list[dict]:
    """Read CSV file and return list of dicts."""
    with open(filepath, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        return list(reader)

def process_data(rows: list[dict]) -> list[dict]:
    """Process and transform data."""
    results = []
    for row in rows:
        # Transform logic here
        processed = {
            'id': row.get('id'),
            'name': row.get('name', '').strip(),
            'value': float(row.get('value', 0)),
        }
        results.append(processed)
    return results

def write_json(data: list[dict], filepath: str) -> None:
    """Write data to JSON file."""
    with open(filepath, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

if __name__ == '__main__':
    rows = read_csv(sys.argv[1])
    processed = process_data(rows)
    write_json(processed, 'output.json')
```

### Data Aggregation
```python
from collections import defaultdict
import csv

def aggregate_by_field(csv_path: str, group_field: str, sum_field: str) -> dict:
    """Aggregate CSV data by grouping field."""
    totals = defaultdict(float)
    counts = defaultdict(int)

    with open(csv_path, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        for row in reader:
            group = row[group_field]
            value = float(row[sum_field])
            totals[group] += value
            counts[group] += 1

    return {
        group: {'total': totals[group], 'count': counts[group]}
        for group in totals
    }
```

### JSON Processing
```python
import json
from pathlib import Path

def merge_json_files(files: list[Path]) -> list[dict]:
    """Merge multiple JSON files into one list."""
    merged = []
    for filepath in files:
        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)
            if isinstance(data, list):
                merged.extend(data)
            else:
                merged.append(data)
    return merged

def filter_json(data: list[dict], predicate: callable) -> list[dict]:
    """Filter JSON data by predicate function."""
    return [item for item in data if predicate(item)]
```

### Subprocess Execution
```python
import subprocess
import shlex

def run_command(cmd: str, timeout: int = 30) -> tuple[int, str, str]:
    """Run shell command and return (exit_code, stdout, stderr)."""
    try:
        result = subprocess.run(
            shlex.split(cmd),
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, '', 'Command timed out'
    except Exception as e:
        return -1, '', str(e)

# Usage
code, stdout, stderr = run_command('cargo test --workspace')
if code == 0:
    print(f"Success: {stdout}")
else:
    print(f"Failed: {stderr}")
```

## Data Analysis (with pandas)

### Basic Analysis
```python
import pandas as pd

def analyze_csv(filepath: str) -> dict:
    """Perform basic CSV analysis."""
    df = pd.read_csv(filepath)

    return {
        'rows': len(df),
        'columns': len(df.columns),
        'column_names': df.columns.tolist(),
        'dtypes': df.dtypes.astype(str).to_dict(),
        'null_counts': df.isnull().sum().to_dict(),
        'numeric_summary': df.describe().to_dict() if len(df.select_dtypes(include='number').columns) > 0 else {},
    }
```

### Time Series Analysis
```python
import pandas as pd
from datetime import datetime

def time_series_summary(filepath: str, date_column: str, value_column: str) -> dict:
    """Summarize time series data."""
    df = pd.read_csv(filepath, parse_dates=[date_column])
    df = df.sort_values(date_column)

    return {
        'start_date': str(df[date_column].min()),
        'end_date': str(df[date_column].max()),
        'total_records': len(df),
        'total_value': float(df[value_column].sum()),
        'daily_avg': float(df[value_column].mean()),
        'peak_value': float(df[value_column].max()),
        'peak_date': str(df.loc[df[value_column].idxmax(), date_column]),
    }
```

## Validation Scripts

### CSV Validation
```python
#!/usr/bin/env python3
"""Validate CSV file structure and content."""

import csv
import sys

def validate_csv(filepath: str, required_columns: list[str]) -> tuple[bool, list[str]]:
    """Validate CSV has required columns."""
    errors = []

    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            reader = csv.DictReader(f)
            columns = reader.fieldnames

            if not columns:
                return False, ["Empty CSV file"]

            missing = set(required_columns) - set(columns)
            if missing:
                errors.append(f"Missing columns: {missing}")

            # Check for empty rows
            row_num = 1
            for row in reader:
                row_num += 1
                if not any(row.values()):
                    errors.append(f"Empty row at line {row_num}")

    except Exception as e:
        return False, [f"Error reading file: {e}"]

    return len(errors) == 0, errors

if __name__ == '__main__':
    required = ['id', 'name', 'email']
    valid, errors = validate_csv(sys.argv[1], required)

    if valid:
        print("✅ CSV validation passed")
        sys.exit(0)
    else:
        print("❌ CSV validation failed:")
        for error in errors:
            print(f"  - {error}")
        sys.exit(1)
```

## BizClaw-Specific Scripts

### Agent Metrics
```python
#!/usr/bin/env python3
"""Generate agent usage metrics from log file."""

import re
import json
from collections import defaultdict
from datetime import datetime

def parse_agent_logs(log_path: str) -> dict:
    """Parse agent logs and extract metrics."""
    metrics = defaultdict(lambda: {'requests': 0, 'errors': 0, 'tokens': 0})

    with open(log_path, 'r', encoding='utf-8') as f:
        for line in f:
            # Parse log format
            match = re.search(r'agent=(\w+) requests=(\d+) errors=(\d+)', line)
            if match:
                agent = match.group(1)
                metrics[agent]['requests'] += int(match.group(2))
                metrics[agent]['errors'] += int(match.group(3))

    return dict(metrics)

if __name__ == '__main__':
    metrics = parse_agent_logs('agent.log')
    print(json.dumps(metrics, indent=2))
```

## Best Practices

### Do's
- ✅ Use type hints for clarity
- ✅ Handle encoding properly (UTF-8)
- ✅ Use context managers (with statement)
- ✅ Validate input before processing
- ✅ Write error messages to stderr
- ✅ Return proper exit codes

### Don'ts
- ❌ Don't use eval() or exec()
- ❌ Don't hardcode sensitive values
- ❌ Don't catch bare Exception
- ❌ Don't use global state
- ❌ Don't process files in memory when streaming is possible
