# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust command-line tool for analyzing and fixing line ending issues in text files. The tool can detect line ending types (LF/CRLF), check for Byte Order Marks (BOM), and optionally fix these issues by rewriting files with consistent line endings or removing BOMs.

## Architecture

The codebase is organized into focused modules:

- **main.rs**: Entry point, argument parsing, and parallel file processing using Rayon
- **lib.rs**: Library interface and module exports
- **config.rs**: Command-line argument parsing using pico-args
- **analysis.rs**: Core file analysis logic for detecting line endings and BOMs
- **processing.rs**: File rewriting operations for fixing line endings and removing BOMs
- **types.rs**: Core data structures including `ConfigSettings`, `FileAnalysis`, `BomType`, `LineEnding`, `LineEndingTarget`, `RewriteResult`, and `BomRemovalResult`
- **utils.rs**: Utility functions for glob pattern expansion and file path handling
- **help.rs**: Help text definition
- **unit_tests.rs**: Comprehensive unit tests for the core functionality
- **tests/integration_tests.rs**: Integration tests for end-to-end functionality

The tool uses parallel processing via Rayon to analyze multiple files concurrently, with results collected and processed sequentially for output and error handling.

## Common Development Commands

```bash
# Build the project
cargo build

# Build optimized release version
cargo build --release

# Run with basic file pattern
cargo run -- "test*.txt"

# Run with specific test file
cargo run -- test_lines.txt

# Run Clippy with strict linting
cargo clippy --color=always -- -D clippy::all -D clippy::pedantic
cargo clippy --all-targets --all-features -- -D warnings

# Format code for consistent style (run after all changes)
cargo fmt

# Run tests (if any exist)
cargo test
```

## Command Line Options

The tool supports these flags:
- `-h, --help`: Prints help information
- `-f, --folder <FOLDER>`: Specify the folder to search in (default: current directory)
- `-c, --case-sensitive`: Case-sensitive glob matching
- `-b, --bom`: Check for Byte Order Mark (BOM) in files
- `-r, --recursive`: Recursively search subdirectories
- `-w, --windows-line-endings`: Rewrite with Windows line endings (CRLF)
- `-l, --linux-line-endings`: Rewrite with Linux line endings (LF)
- `-m, --remove-bom`: Remove BOM from files that have one
- `-d, --delete-backups`: Move .bak backup files to trash after operations

Note: The `-w` and `-l` options are mutually exclusive (enforced via `LineEndingTarget` enum).

## Dependencies

- **anyhow**: Error handling with context
- **pico-args**: Lightweight command-line argument parsing
- **rayon**: Data parallelism for concurrent file processing
- **glob**: File pattern matching and expansion
- **trash**: Cross-platform trash/recycle bin support for safe backup deletion
- **tempfile**: Temporary file creation and safe atomic file operations

## Build Configuration

The project uses Rust 2024 edition with aggressive release optimizations:
- LTO enabled for smaller binaries
- Single codegen unit
- Debug symbols stripped
- Panic abort strategy

**Security:**
- Run `cargo audit` once a day when working on this project to check for security vulnerabilities in dependencies
