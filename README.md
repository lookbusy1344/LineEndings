# Line Endings Analyzer

[![Build and test](https://github.com/lookbusy1344/LineEndings/actions/workflows/ci.yml/badge.svg)](https://github.com/lookbusy1344/LineEndings/actions/workflows/ci.yml)

A fast Rust command-line tool for analyzing and fixing line ending issues in text files. Detect line ending types (LF/CRLF), check for Byte Order Marks (BOM), and optionally fix these issues with consistent line endings.

## Features

- **Line Ending Detection**: Identify LF (Unix/Linux) vs CRLF (Windows) line endings
- **BOM Detection**: Check for Byte Order Marks in text files
- **Batch Processing**: Process multiple files using glob patterns
- **Parallel Processing**: Fast analysis using multi-threaded processing
- **File Fixing**: Rewrite files with consistent line endings or remove BOMs
- **Recursive Search**: Optionally search subdirectories

## Installation

### From Source

```bash
git clone <repository-url>
cd LineEndings
cargo build --release
```

The binary will be available at `target/release/line-endings`.

## Usage

### Basic Analysis

```bash
# Analyze all .txt files in current directory
./line-endings "*.txt"

# Analyze a specific file
./line-endings test_file.txt

# Check for BOMs in files
./line-endings --bom "*.txt"

# Recursive search in subdirectories
./line-endings --recursive "**/*.txt"
```

### Fixing Files

```bash
# Convert to Linux line endings (LF)
./line-endings --linux-line-endings "*.txt"

# Convert to Windows line endings (CRLF)
./line-endings --windows-line-endings "*.txt"

# Remove BOM from files
./line-endings --remove-bom "*.txt"
```

### Advanced Options

```bash
# Search in specific folder with case-sensitive matching
./line-endings --folder /path/to/files --case-sensitive "*.TXT"

# Combine operations: convert to LF and remove BOM
./line-endings --linux-line-endings --remove-bom "*.txt"
```

## Command Line Options

| Option | Short | Description |
|--------|-------|-------------|
| `--help` | `-h` | Show help information |
| `--folder <FOLDER>` | `-f` | Specify search directory (default: current) |
| `--case-sensitive` | `-c` | Enable case-sensitive glob matching |
| `--bom` | `-b` | Check for Byte Order Mark (BOM) |
| `--recursive` | `-r` | Search subdirectories recursively |
| `--windows-line-endings` | `-w` | Convert to Windows line endings (CRLF) |
| `--linux-line-endings` | `-l` | Convert to Linux line endings (LF) |
| `--remove-bom` | `-m` | Remove BOM from files |

**Note**: The `--windows-line-endings` and `--linux-line-endings` options are mutually exclusive.

## Examples

### Analysis Only

```bash
# Check line endings in all text files
./line-endings "*.txt"

# Output example:
# test_windows.txt: CRLF line endings
# test_linux.txt: LF line endings
# has_bom.txt: LF line endings, BOM detected
```

### Fixing Files

```bash
# Standardize all source files to LF endings
./line-endings --linux-line-endings --recursive "**/*.{rs,js,py}"

# Clean up text files (LF + remove BOM)
./line-endings --linux-line-endings --remove-bom "*.txt"
```

## Development

### Building

```bash
# Debug build
cargo build

# Optimized release build
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Run clippy for linting
cargo clippy --color=always -- -D clippy::all -D clippy::pedantic

# Format code
cargo fmt
```

## Dependencies

- **anyhow**: Error handling with context
- **pico-args**: Lightweight command-line argument parsing
- **rayon**: Parallel processing for performance
- **glob**: File pattern matching

## License

[Add your license information here]

## Contributing

[Add contribution guidelines here]
