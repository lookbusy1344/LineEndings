use anyhow::Result;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::path::Path;

use crate::types::{
    BomRemovalResult, BomType, ConfigSettings, FileAnalysis, LineEnding, RewriteResult,
};

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

    // Check if file ends with a newline by reading only the last byte
    let has_trailing_newline = check_trailing_newline(input_path)?;

    // Process file line by line without loading into memory
    let infile = File::open(input_path)?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, infile);
    let mut outfile = File::create(&output_path)?;

    let line_ending: &[u8] = match ending {
        LineEnding::Lf => &b"\n"[..],
        LineEnding::Crlf => &b"\r\n"[..],
    };

    let mut lines = reader.lines();
    let mut last_line: Option<String> = None;

    // Process all lines except the last
    for line in lines.by_ref() {
        if let Some(prev_line) = last_line.take() {
            outfile.write_all(prev_line.as_bytes())?;
            outfile.write_all(line_ending)?;
        }
        last_line = Some(line?);
    }

    // Write the last line, adding line ending only if original had trailing newline
    if let Some(line) = last_line {
        outfile.write_all(line.as_bytes())?;
        if has_trailing_newline {
            outfile.write_all(line_ending)?;
        }
    }

    // Ensure all data is written before replacing files
    outfile.flush()?;

    // Replace the original file with the new one
    std::fs::rename(output_path, input_path)?;

    Ok(())
}

/// Checks if a file ends with a newline without reading the entire file
fn check_trailing_newline(path: &Path) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();

    if file_size == 0 {
        return Ok(false);
    }

    // Seek to the last byte
    file.seek(io::SeekFrom::End(-1))?;
    let mut last_byte = [0u8; 1];
    file.read_exact(&mut last_byte)?;

    Ok(last_byte[0] == b'\n')
}

/// Removes BOMs from files based on the file analysis
///
/// # Errors
///
/// Returns an error if BOM detection is not enabled or if BOM removal fails.
pub fn remove_bom_from_files(config: &ConfigSettings, results: &[FileAnalysis]) -> Result<()> {
    // Make sure we're only processing files that have been checked for BOMs
    if !config.check_bom {
        return Err(anyhow::anyhow!(
            "BOM detection must be enabled (--bom) to remove BOMs"
        ));
    }

    println!();

    // Process files in parallel using rayon
    let removal_results: Vec<BomRemovalResult> = results
        .par_iter()
        .map(process_file_for_bom_removal)
        .collect();

    // Process results sequentially for consistent output and counting
    let mut bom_removed = 0usize;
    let mut files_skipped = 0usize;

    for removal_result in &removal_results {
        if let Some(error) = &removal_result.error {
            return Err(anyhow::anyhow!(
                "Failed to remove BOM from {}: {}",
                removal_result.path.display(),
                error
            ));
        }

        if removal_result.removed {
            if let Some(bom_type) = removal_result.bom_type {
                println!(
                    "\"{}\"\tBOM removed: {bom_type}",
                    removal_result.path.display()
                );
            }
            bom_removed += 1;
        } else {
            files_skipped += 1;
        }
    }

    println!("BOM removed from {bom_removed} file(s), skipped {files_skipped}");

    Ok(())
}

/// Processes a single file for BOM removal
#[must_use]
pub fn process_file_for_bom_removal(result: &FileAnalysis) -> BomRemovalResult {
    // Skip files without BOMs or with errors
    if result.error.is_some() || !result.has_bom() {
        return BomRemovalResult {
            path: result.path.clone(),
            removed: false,
            bom_type: None,
            error: None,
        };
    }

    // Get the BOM type safely using if-let
    let Some(bom_type) = result.bom_type else {
        return BomRemovalResult {
            path: result.path.clone(),
            removed: false,
            bom_type: None,
            error: None,
        };
    };

    // Get the size of the BOM to skip
    let bom_size = match bom_type {
        BomType::None => 0,
        BomType::Utf8 => 3,
        BomType::Utf16Le | BomType::Utf16Be => 2,
        BomType::Utf32Le | BomType::Utf32Be => 4,
    };

    if bom_size == 0 {
        return BomRemovalResult {
            path: result.path.clone(),
            removed: false,
            bom_type: Some(bom_type),
            error: None,
        };
    }

    // Process the file to remove the BOM
    match remove_bom_from_file(&result.path, bom_size) {
        Ok(()) => BomRemovalResult {
            path: result.path.clone(),
            removed: true,
            bom_type: Some(bom_type),
            error: None,
        },
        Err(e) => BomRemovalResult {
            path: result.path.clone(),
            removed: false,
            bom_type: Some(bom_type),
            error: Some(e.to_string()),
        },
    }
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

/// Deletes backup files for the given file analyses
///
/// # Errors
///
/// Returns an error if backup deletion fails.
pub fn delete_backup_files(results: &[FileAnalysis]) -> Result<()> {
    println!();

    let mut deleted_count = 0usize;
    let mut not_found_count = 0usize;

    for result in results {
        // Skip files with errors
        if result.error.is_some() {
            continue;
        }

        let backup_path = get_backup_path(&result.path);
        if backup_path.exists() {
            match std::fs::remove_file(&backup_path) {
                Ok(()) => {
                    println!("\"{}\"\tbackup deleted", backup_path.display());
                    deleted_count += 1;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to delete backup {}: {}",
                        backup_path.display(),
                        e
                    ));
                }
            }
        } else {
            not_found_count += 1;
        }
    }

    println!("Deleted {deleted_count} backup file(s), {not_found_count} not found");

    Ok(())
}
