# Project guidelines

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Version control (read first)

Before the first VCS command, run `jj --ignore-working-copy root`. This may be a jj repo on one machine and plain Git on another.

**IMPORTANT:** if it succeeds, use `jj` for all VCS commands, including `log`, `show`, `status` and `diff`. Do not run `git` on jj repos. The `gitStatus` snapshot in the session context is not a reason to use git.

jj has no commit hooks. Run the pre-commit checks before `jj commit`, `jj describe` (when finalising a change) and `jj squash`.
If a change touches only non-code files (`*.md`), skip the cargo steps.

Before `jj git push` or moving a shared bookmark, run `scripts/pre-push.sh`. By default it formats the tip (newest non-empty mutable revision) with `jj fix`, then runs fmt check, build, clippy and nextest on it. `--full` formats every mutable revision in `::@` and checks each in its own checkout via `jj run`. In a plain git checkout it checks HEAD (`--full`: each commit in `@{upstream}..HEAD`) and does not reformat. `scripts/agent-pre-push-hook.sh` runs the tip check on every push command and blocks the push on failure. Claude Code (`.claude/settings.json`) and Codex (`.codex/hooks.json`) call it as a `PreToolUse` hook. The hook checks the tip of `@`, not the bookmark being pushed, so push only the bookmark you are working on: the one on `@-`, with an empty `@` above it.

Push only on explicit request. "Push this" means: if `@` is non-empty, `jj commit` it (after the pre-commit checks). Move the bookmark to `@-` with `jj bookmark set <name> -r @-`, then `jj git push --bookmark <name>`. Use the bookmark already on the stack; otherwise `main`. Never push any other bookmark. Do not use `jj git push -c`.

## Personal information

Exclude PII from every commit, commit message and bookmark name: real names, email addresses, usernames, machine paths such as `/Users/<name>/`, hostnames, tokens and credentials. Check the diff before `jj commit`, `jj describe` (finalising) and `git commit`.

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
cargo clippy --all-targets --all-features
cargo fmt
cargo nextest run
```

## Build Configuration

The project uses Rust 2024 edition with aggressive release optimizations (LTO, single codegen unit, stripped debug symbols, panic abort). See `Cargo.toml` for details.

**Before every commit, `scripts/pre-commit.sh` must pass** (the `pre-commit` skill). It runs fmt check, build, clippy and nextest, and skips them when no `.rs`, `Cargo.toml` or `Cargo.lock` file changed. Lint levels (`clippy::all`, `clippy::pedantic` deny; `unsafe_code` forbid) live in `Cargo.toml` `[lints]`.

**Security:**
- Run `cargo audit` once a day when working on this project to check for security vulnerabilities in dependencies
