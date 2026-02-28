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
cargo build
cargo build --release
cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code
cargo fmt
cargo test
```

## Build Configuration

The project uses Rust 2024 edition with aggressive release optimizations (LTO, single codegen unit, stripped debug symbols, panic abort). See `Cargo.toml` for details.

**Before every commit, all of the following must pass:**

```bash
cargo build --all-targets
cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code
cargo fmt
cargo test
```

**Security:**
- Run `cargo audit` once a day when working on this project to check for security vulnerabilities in dependencies
