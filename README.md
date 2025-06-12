# CLI Tools Collection

A collection of custom command line tools, each organized as an independent Rust project.

## Repository Structure

This repository is organized as a collection of independent CLI tools, where each tool lives in its own subdirectory:

```
cli-tools/
├── README.md           # This file
├── .gitignore          # Shared gitignore for Rust projects
├── claude-usage/       # CLI tool for tracking Claude usage
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs
│   └── README.md
├── [tool-name]/        # Another CLI tool
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs
│   └── README.md
└── ...
```

## Tools

### claude-usage
A command line tool for tracking and analyzing Claude API usage.

## Building Tools

Each tool is an independent Rust project and can be built separately:

```bash
# Build a specific tool
cd tool-name
cargo build --release

# Run a specific tool
cargo run

# Install a tool globally
cargo install --path .
```

## Adding a New Tool

To add a new CLI tool to this collection:

1. Create a new directory for your tool:
   ```bash
   mkdir new-tool-name
   cd new-tool-name
   ```

2. Initialize a new Rust project:
   ```bash
   cargo init
   ```

3. Develop your tool in the `src/main.rs` file

4. Add a README.md specific to your tool explaining its purpose and usage

5. Build and test your tool:
   ```bash
   cargo build
   cargo run
   ```

## Development Guidelines

- Each tool should be self-contained and independent
- Include a README.md in each tool directory with:
  - Purpose and description
  - Installation instructions
  - Usage examples
  - Configuration options (if any)
- Follow Rust best practices and idioms
- Add appropriate error handling
- Include tests where applicable
- Use descriptive commit messages

## License

[Add your preferred license here]