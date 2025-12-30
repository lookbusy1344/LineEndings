// #![allow(unused_imports)]
#![allow(dead_code)]
// #![allow(unused_variables)]

use anyhow::{Context, Result};
use pico_args::Arguments;
use rayon::prelude::*;
use std::time::Instant;

mod analysis;
mod config;
mod help;
mod processing;
mod types;
mod utils;

use analysis::analyze_file;
use config::parse_args;
use help::show_help;
use processing::{remove_bom_from_files, rewrite_files};
use utils::get_paths_matching_glob;

fn main() -> Result<()> {
    // Help debugging in Zed by passing arguments directly
    // let debug_args: Vec<std::ffi::OsString> = vec!["test*.txt".into()];
    // let mut p_args = Arguments::from_vec(debug_args);

    // Parse command line arguments
    let mut p_args = Arguments::from_env();

    // special handling of help
    if p_args.contains(["-h", "--help"]) {
        show_help();
        return Ok(());
    }

    let config = parse_args(p_args)?;

    let start_time = Instant::now();

    // expand glob patterns and get file paths
    let expanded_paths =
        get_paths_matching_glob(&config).with_context(|| "Failed to expand glob patterns")?;

    if expanded_paths.is_empty() {
        return Err(anyhow::anyhow!("No input files found"));
    }

    // Build configuration display, only showing non-default/active options
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

    // Only show line ending alteration if one is set
    match (config.set_linux, config.set_windows) {
        (true, false) => config_parts.push("Line ending alteration: Linux (LF)".to_string()),
        (false, true) => config_parts.push("Line ending alteration: Windows (CRLF)".to_string()),
        (true, true) => config_parts.push("Line ending alteration: Invalid (both set)".to_string()),
        (false, false) => {} // Don't show anything for no alteration
    }

    // Display configuration if there are any non-default options
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
        if let Some(error) = &result.error {
            let filename = result.path.display();
            if error.contains("Binary file detected") {
                binary_files += 1;
            } else {
                println!("\nFile: {filename}\terror: {error}");
                has_errors += 1;
            }
        } else {
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

    // optionally rewrite files if requested
    if config.has_rewrite_option() {
        rewrite_files(&config, &results)?;
    }

    // Remove BOMs if requested (can happen alongside line ending changes)
    if config.remove_bom {
        remove_bom_from_files(&config, &results)?;
    }

    // Print summary statistics
    let total_duration = start_time.elapsed();
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

    Ok(())
}
