# claude-usage

A command line tool for tracking and analyzing Claude API usage.

## Description

This tool helps you monitor and analyze your Claude API usage patterns, including:
- Token consumption
- Cost tracking
- Usage statistics
- Historical trends

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Show usage statistics
claude-usage stats

# Track API calls
claude-usage track

# Generate reports
claude-usage report --format json
```

## Configuration

The tool stores configuration in `~/.config/claude-usage/config.toml`.

## Development

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the tool in development
cargo run -- [arguments]
```