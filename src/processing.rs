use anyhow::Result;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

use crate::types::{BomType, ConfigSettings, FileAnalysis, LineEnding, RewriteResult};

// Define constants for line ending characters and buffer size
const BUFFER_SIZE: usize = 4096; // 4KB buffer for more efficient reading

/// Rewrites files with specified line endings based on the configuration settings.
///
/// # Errors
///
/// Returns an error if no rewrite option is set or if file rewriting fails.
pub fn rewrite_files(config: &ConfigSettings, results: &[FileAnalysis]) -> Result<()> {
    // error out if no rewrite option is set
    if !config.has_rewrite_option() {
        return Err(anyhow::anyhow!("No line ending rewrite option set"));
    }

    let ending = if config.set_linux {
        LineEnding::Lf
    } else {
        LineEnding::Crlf
    };

    println!();

    // Process files in parallel using rayon
    let rewrite_results: Vec<RewriteResult> = results
        .par_iter()
        .map(|result| process_file_for_rewrite(result, config, ending))
        .collect();

    // Process results sequentially for consistent output and counting
    let mut rewritten_files = 0usize;
    let mut skipped_files = 0usize;

    for rewrite_result in &rewrite_results {
        if let Some(error) = &rewrite_result.error {
            return Err(anyhow::anyhow!(
                "Failed to rewrite file: {}: {}",
                rewrite_result.path.display(),
                error
            ));
        }

        if rewrite_result.rewritten {
            println!("\"{}\"\trewritten", rewrite_result.path.display());
            rewritten_files += 1;
        } else {
            println!("\"{}\"\trewrite skipped", rewrite_result.path.display());
            skipped_files += 1;
        }
    }

    println!(
        "Rewritten {} file(s) with {} line endings, skipped {}",
        rewritten_files,
        match ending {
            LineEnding::Lf => "Linux (LF)",
            LineEnding::Crlf => "Windows (CRLF)",
        },
        skipped_files
    );

    Ok(())
}

/// Processes a single file for rewriting based on configuration and line ending analysis
#[must_use]
pub fn process_file_for_rewrite(
    result: &FileAnalysis,
    config: &ConfigSettings,
    ending: LineEnding,
) -> RewriteResult {
    let mut rebuild = false;

    if result.has_mixed_line_endings() {
        // mixed line endings, always rebuild
        rebuild = true;
    }
    if (config.set_linux && result.is_crlf_only()) || (config.set_windows && result.is_lf_only()) {
        // rebuild if its exclusively the wrong type
        rebuild = true;
    }

    if rebuild {
        match rewrite_file_with_line_ending(&result.path, ending) {
            Ok(()) => RewriteResult {
                path: result.path.clone(),
                rewritten: true,
                error: None,
            },
            Err(e) => RewriteResult {
                path: result.path.clone(),
                rewritten: false,
                error: Some(e.to_string()),
            },
        }
    } else {
        // file is already in the correct format, skip it
        RewriteResult {
            path: result.path.clone(),
            rewritten: false,
            error: None,
        }
    }
}

/// Creates a backup of a file if it doesn't already exist
fn create_backup_if_needed(input_path: &Path) -> io::Result<()> {
    let backup_path = get_backup_path(input_path);

    // Only create backup if it doesn't exist yet
    if !backup_path.exists() {
        std::fs::copy(input_path, &backup_path)?;
    }
    Ok(())
}

/// Gets the backup path for a given file
fn get_backup_path(input_path: &Path) -> std::path::PathBuf {
    input_path.with_extension(format!(
        "{}.bak",
        input_path
            .extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or_default()
    ))
}

/// Rewrites a file with specified line endings.
/// Creates a backup of the original file with .BAK extension (if not already created) and
/// replaces the original file with the new version.
///
/// # Errors
///
/// Returns an error if file operations (backup creation, reading, writing, or renaming) fail.
pub fn rewrite_file_with_line_ending(input_path: &Path, ending: LineEnding) -> io::Result<()> {
    // Create backup if needed
    create_backup_if_needed(input_path)?;

    // Create output_path by prepending an underscore to the filename
    let parent = input_path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = input_path.file_name().unwrap_or_default();
    let mut new_file_name = String::from("_");
    new_file_name.push_str(&file_name.to_string_lossy());
    let output_path = parent.join(new_file_name);

    // Create a temporary file for new content
    let infile = File::open(input_path)?;
    let reader = BufReader::new(infile);
    let mut outfile = File::create(&output_path)?;

    let line_ending: &[u8] = match ending {
        LineEnding::Lf => &b"\n"[..],
        LineEnding::Crlf => &b"\r\n"[..],
    };

    for line in reader.lines() {
        let line = line?;
        outfile.write_all(line.as_bytes())?;
        outfile.write_all(line_ending)?;
    }

    // Ensure all data is written before replacing files
    outfile.flush()?;

    // Replace the original file with the new one
    std::fs::rename(output_path, input_path)?;

    Ok(())
}

/// Removes BOMs from files based on the file analysis
///
/// # Errors
///
/// Returns an error if BOM detection is not enabled or if BOM removal fails.
///
/// # Panics
///
/// Panics if a file marked as having a BOM doesn't have a valid BOM type.
pub fn remove_bom_from_files(config: &ConfigSettings, results: &[FileAnalysis]) -> Result<()> {
    // Make sure we're only processing files that have been checked for BOMs
    if !config.check_bom {
        return Err(anyhow::anyhow!(
            "BOM detection must be enabled (--bom) to remove BOMs"
        ));
    }

    println!();

    // Keep track of how many files were processed
    let mut bom_removed = 0;
    let mut files_skipped = 0;

    // Process each file that has a BOM
    for result in results {
        // Skip files without BOMs or with errors
        if result.error.is_some() || !result.has_bom() {
            files_skipped += 1;
            continue;
        }

        // Get the BOM type
        let bom_type = result.bom_type.unwrap();

        // Get the size of the BOM to skip
        let bom_size = match bom_type {
            BomType::None => 0,
            BomType::Utf8 => 3,
            BomType::Utf16Le | BomType::Utf16Be => 2,
            BomType::Utf32Le | BomType::Utf32Be => 4,
        };

        if bom_size == 0 {
            files_skipped += 1;
            continue;
        }

        // Process the file to remove the BOM
        match remove_bom_from_file(&result.path, bom_size) {
            Ok(()) => {
                println!(
                    "\"{}\"\tBOM removed: {}",
                    result.path.display(),
                    bom_type.to_string()
                );
                bom_removed += 1;
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to remove BOM from {}: {}",
                    result.path.display(),
                    e
                ));
            }
        }
    }

    println!("BOM removed from {bom_removed} file(s), skipped {files_skipped}");

    Ok(())
}

/// Removes a BOM from a file while preserving its content and line endings
///
/// # Errors
///
/// Returns an error if file operations (backup creation, reading, writing, or renaming) fail.
pub fn remove_bom_from_file(path: &Path, bom_size: usize) -> io::Result<()> {
    // Create backup if needed
    create_backup_if_needed(path)?;

    // Create output_path by prepending an underscore to the filename
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = path.file_name().unwrap_or_default();
    let mut new_file_name = String::from("_");
    new_file_name.push_str(&file_name.to_string_lossy());
    let output_path = parent.join(new_file_name);

    // Open the original file for reading
    let mut input_file = File::open(path)?;
    let mut output_file = File::create(&output_path)?;

    // Skip the BOM
    let mut buffer = vec![0; bom_size];
    input_file.read_exact(&mut buffer)?;

    // Copy the rest of the file directly (preserving line endings)
    let mut buffer = [0; BUFFER_SIZE];
    loop {
        let bytes_read = input_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        output_file.write_all(&buffer[..bytes_read])?;
    }

    // Ensure all data is written before replacing files
    output_file.flush()?;

    // Replace the original file with the new one
    std::fs::rename(output_path, path)?;

    Ok(())
}
