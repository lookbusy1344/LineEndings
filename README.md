# Line Endings Analyzer

[![Build and test](https://github.com/lookbusy1344/LineEndings/actions/workflows/ci.yml/badge.svg)](https://github.com/lookbusy1344/LineEndings/actions/workflows/ci.yml)

A fast, safe, and efficient Rust command-line tool for analyzing and fixing line ending issues in text files. Detect line ending types (LF/CRLF), check for Byte Order Marks (BOM), and optionally fix these issues with consistent line endings.

## Features

- **Line Ending Detection**: Identify LF (Unix/Linux) vs CRLF (Windows) line endings
- **BOM Detection**: Check for Byte Order Marks in text files (UTF-8, UTF-16, UTF-32)
- **Binary File Detection**: Automatically skips binary files to prevent corruption
- **Batch Processing**: Process multiple files using glob patterns
- **Parallel Processing**: Fast analysis using multi-threaded processing
- **Memory Efficient**: Streams files line-by-line without loading entire contents into memory
- **File Fixing**: Rewrite files with consistent line endings or remove BOMs
- **Recursive Search**: Optionally search subdirectories
- **Safe Backup System**: Creates `.bak` backups before modifying files
- **Trash Integration**: Optionally move backup files to system trash/recycle bin
- **Statistics & Timing**: Comprehensive summary with execution times

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

# Clean up backup files after conversion
./line-endings --linux-line-endings --delete-backups "*.txt"
```

### Advanced Options

```bash
# Search in specific folder with case-sensitive matching
./line-endings --folder /path/to/files --case-sensitive "*.TXT"

# Combine operations: convert to LF, remove BOM, and clean up backups
./line-endings --linux-line-endings --remove-bom --delete-backups "*.txt"
```

## Safety Features

The tool includes several safety features to protect your files:

- **Automatic Backups**: Creates `.bak` backup files before any modifications
- **Binary File Detection**: Automatically skips binary files (executables, images, etc.)
- **Trash Integration**: Backup cleanup moves files to trash/recycle bin (recoverable), not permanent deletion
- **Memory Efficiency**: Streams large files without loading them entirely into memory
- **Error Handling**: Stops on errors and reports issues clearly

**Important**: Original files are NEVER permanently deleted. They are always backed up before modification.

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
| `--delete-backups` | `-d` | Move .bak backup files to trash after operations |

**Note**: The `--windows-line-endings` and `--linux-line-endings` options are mutually exclusive.

## Examples

### Analysis Only

```bash
# Check line endings in all text files
./line-endings "*.txt"

# Output example:
# "test_windows.txt"    CRLF 15
# "test_linux.txt"      LF 25
# "test_lines.txt"      Mixed LF 10, CRLF 10
# 
# --- Summary ---
# Total files processed: 3
# Files with mixed line endings: 1
# Total LF line endings: 35
# Total CRLF line endings: 25
# Analysis time: 0.001s
# Total time: 0.002s
```

### Fixing Files

```bash
# Standardize all source files to LF endings
./line-endings --linux-line-endings --recursive "**/*.{rs,js,py}"

# Clean up text files (LF + remove BOM + delete backups)
./line-endings --linux-line-endings --remove-bom --delete-backups "*.txt"
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
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt
```

## Performance

The tool is optimized for performance and efficiency:

- **Parallel Processing**: Uses Rayon for multi-threaded file analysis
- **Memory Efficient**: Streams files line-by-line (no full file loading)
- **Fast I/O**: Uses buffered readers with 4KB buffers
- **Release Optimizations**: LTO and single codegen unit for smaller, faster binaries

Typical performance: Processes thousands of files in seconds, with minimal memory overhead.

## Dependencies

- **anyhow**: Error handling with context
- **pico-args**: Lightweight command-line argument parsing
- **rayon**: Parallel processing for performance
- **glob**: File pattern matching
- **trash**: Cross-platform trash/recycle bin support

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
