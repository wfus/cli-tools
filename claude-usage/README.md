# claude-usage

A command line tool for tracking and analyzing Claude API usage.

## Description

This tool helps you monitor and analyze your Claude API usage patterns, including:
- Token consumption
- Cost tracking
- Usage statistics
- Historical trends

## Installation

### Option 1: Install with Cargo (Recommended)

```bash
# From the claude-usage directory
cargo install --path .

# This installs the binary to ~/.cargo/bin/claude-usage
# Make sure ~/.cargo/bin is in your PATH
```

### Option 2: Build and Copy Manually

```bash
# Build the release binary
cargo build --release

# Copy to a directory in your PATH (e.g., /usr/local/bin)
# You may need sudo for system directories
cp target/release/claude-usage /usr/local/bin/

# Or copy to a user directory (no sudo needed)
mkdir -p ~/bin
cp target/release/claude-usage ~/bin/
# Then add ~/bin to your PATH in ~/.bashrc or ~/.zshrc:
# export PATH="$HOME/bin:$PATH"
```

### Verify Installation

```bash
# Check if claude-usage is available
which claude-usage

# Test the installation
claude-usage --help
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