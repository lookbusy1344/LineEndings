#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use pico_args::Arguments;
use rayon::prelude::*;
use std::time::Instant;

mod help;

use help::show_help;
use line_endings::analysis::analyze_file;
use line_endings::config::parse_args;
use line_endings::processing::{
    existing_backup_paths, remove_bom_from_files, rewrite_files, trash_backup_files,
    would_remove_bom, would_rewrite,
};
use line_endings::types::{self, FileAnalysis, LineEndingTarget};
use line_endings::utils::get_paths_matching_glob;

/// Formats and prints analysis results for a successfully analyzed file
fn print_file_analysis(result: &FileAnalysis) {
    let file_name = result.path.display();

    let mut parts = Vec::new();
    if result.lf_count > 0 {
        parts.push(format!("LF {}", result.lf_count));
    }
    if result.crlf_count > 0 {
        parts.push(format!("CRLF {}", result.crlf_count));
    }
    if result.cr_count > 0 {
        parts.push(format!("CR {}", result.cr_count));
    }
    let line_endings = match parts.len() {
        0 => String::from("None"),
        1 => parts.remove(0),
        _ => format!("Mixed {}", parts.join(", ")),
    };

    let bom_info = if result.bom_checked {
        match &result.bom_type {
            None => String::from(", BOM: none"),
            Some(bom) => format!(", BOM: {bom}"),
        }
    } else {
        String::new()
    };

    println!("\"{file_name}\"\t{line_endings}{bom_info}");
}

fn main() -> Result<()> {
    let mut p_args = Arguments::from_env();

    if p_args.contains(["-h", "--help"]) {
        show_help();
        return Ok(());
    }

    let config = parse_args(p_args)?;

    let start_time = Instant::now();

    // expand glob patterns and get file paths (symbolic links are excluded)
    let expanded_paths =
        get_paths_matching_glob(&config).with_context(|| "Failed to expand glob patterns")?;

    if expanded_paths.is_empty() {
        return Err(anyhow::anyhow!("No input files found"));
    }

    // Display configuration if there are any non-default options
    let config_parts = build_config_display(&config);
    if !config_parts.is_empty() {
        println!("{}", config_parts.join(", "));
    }

    // Process all files in parallel using rayon
    let analysis_start = Instant::now();
    let results: Vec<_> = expanded_paths
        .par_iter()
        .map(|path| analyze_file(path, &config))
        .collect();
    let analysis_duration = analysis_start.elapsed();

    // Print any errors and categorize them
    let mut has_errors = 0;
    let mut binary_files = 0;
    let mut analyzed_files = 0;
    let mut total_lf = 0usize;
    let mut total_crlf = 0usize;
    let mut mixed_files = 0usize;

    for result in &results {
        if result.is_binary {
            binary_files += 1;
        } else if let Some(error) = &result.error {
            let filename = result.path.display();
            println!("\nFile: {filename}\terror: {error}");
            has_errors += 1;
        } else {
            print_file_analysis(result);

            analyzed_files += 1;
            total_lf += result.lf_count;
            total_crlf += result.crlf_count;
            if result.has_mixed_line_endings() {
                mixed_files += 1;
            }
        }
    }

    // Report binary files separately
    if binary_files > 0 {
        println!("\nSkipped {binary_files} binary file(s)");
    }

    // bail if there are any real errors (not binary files)
    if has_errors > 0 {
        return Err(anyhow::anyhow!("  Files with errors: {has_errors}"));
    }

    // In dry-run mode, report what would change and stop before any mutation.
    if config.dry_run {
        print_dry_run(&config, &results);
        print_summary(
            analyzed_files,
            binary_files,
            mixed_files,
            total_lf,
            total_crlf,
            analysis_duration,
            start_time.elapsed(),
        );
        return Ok(());
    }

    let will_mutate = config.has_rewrite_option() || config.remove_bom;

    // Snapshot backups that already exist *before* any mutation. These were not
    // created by this run (stale leftovers or unrelated user files) and must
    // never be trashed.
    let preexisting_backups = if will_mutate {
        snapshot_and_warn_preexisting_backups(&results)
    } else {
        std::collections::HashSet::new()
    };

    // optionally rewrite files if requested
    if config.has_rewrite_option() {
        rewrite_files(&config, &results)?;
    }

    // Remove BOMs if requested (can happen alongside line ending changes)
    if config.remove_bom {
        remove_bom_from_files(&config, &results)?;
    }

    // Move backup files to trash unless --no-trash was specified
    if !config.no_trash && will_mutate {
        trash_backup_files(&results, &preexisting_backups)?;
    }

    // Print summary statistics
    let total_duration = start_time.elapsed();
    print_summary(
        analyzed_files,
        binary_files,
        mixed_files,
        total_lf,
        total_crlf,
        analysis_duration,
        total_duration,
    );

    Ok(())
}

/// Builds the list of non-default/active configuration options to display.
fn build_config_display(config: &types::ConfigSettings) -> Vec<String> {
    let mut config_parts = Vec::new();

    // Always show folder if not current directory
    if let Some(folder) = &config.folder
        && folder != "."
    {
        config_parts.push(format!("Folder: {folder}"));
    }

    // Only show boolean flags if they are true
    if config.case_sensitive {
        config_parts.push("Case sensitive: true".to_string());
    }
    if config.recursive {
        config_parts.push("Recursive: true".to_string());
    }
    if config.check_bom {
        config_parts.push("Check BOM: true".to_string());
    }
    if config.remove_bom {
        config_parts.push("Remove BOM: true".to_string());
    }
    if config.no_trash {
        config_parts.push("Trash backups: disabled".to_string());
    }

    // Only show line ending alteration if one is set
    match config.line_ending_target {
        LineEndingTarget::Linux => {
            config_parts.push("Line ending alteration: Linux (LF)".to_string());
        }
        LineEndingTarget::Windows => {
            config_parts.push("Line ending alteration: Windows (CRLF)".to_string());
        }
        LineEndingTarget::None => {} // Don't show anything for no alteration
    }

    config_parts
}

/// Reports what each file would have done in a real run, without modifying it.
fn print_dry_run(config: &types::ConfigSettings, results: &[FileAnalysis]) {
    println!("\n--- Dry run (no files will be modified) ---");

    let target_label = match config.line_ending_target {
        LineEndingTarget::Linux => Some("LF"),
        LineEndingTarget::Windows => Some("CRLF"),
        LineEndingTarget::None => None,
    };

    let mut would_change = 0usize;
    for result in results {
        if result.is_binary || result.error.is_some() {
            continue;
        }
        let path = result.path.display();
        if let Some(label) = target_label
            && would_rewrite(result, config.line_ending_target)
        {
            println!("\"{path}\"\twould rewrite to {label}");
            would_change += 1;
        }
        if config.remove_bom && would_remove_bom(result) {
            let bom = result.bom_type.expect("would_remove_bom implies a BOM");
            println!("\"{path}\"\twould remove BOM: {bom}");
            would_change += 1;
        }
    }

    if would_change == 0 {
        println!("No files would be modified");
    }
}

/// Snapshots backups that already exist before any mutation and warns that
/// each will neither be refreshed nor trashed by this run.
fn snapshot_and_warn_preexisting_backups(
    results: &[FileAnalysis],
) -> std::collections::HashSet<std::path::PathBuf> {
    let preexisting = existing_backup_paths(results);
    for backup_path in &preexisting {
        println!(
            "Warning: backup \"{}\" already exists; it will not be refreshed and the change will not be protected by it",
            backup_path.display()
        );
    }
    preexisting
}

fn print_summary(
    analyzed_files: usize,
    binary_files: usize,
    mixed_files: usize,
    total_lf: usize,
    total_crlf: usize,
    analysis_duration: std::time::Duration,
    total_duration: std::time::Duration,
) {
    println!("\n--- Summary ---");
    println!("Total files processed: {analyzed_files}");
    if binary_files > 0 {
        println!("Binary files skipped: {binary_files}");
    }
    if mixed_files > 0 {
        println!("Files with mixed line endings: {mixed_files}");
    }
    println!("Total LF line endings: {total_lf}");
    println!("Total CRLF line endings: {total_crlf}");
    println!("Analysis time: {:.3}s", analysis_duration.as_secs_f64());
    println!("Total time: {:.3}s", total_duration.as_secs_f64());
}
