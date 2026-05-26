use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

use crate::types::{
    BomRemovalResult, BomType, ConfigSettings, FileAnalysis, LineEnding, LineEndingTarget,
    RewriteResult,
};

// Define constants for line ending characters and buffer size
const BUFFER_SIZE: usize = 4096; // 4KB buffer for more efficient reading
const CR: u8 = b'\r';
const LF: u8 = b'\n';

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

    let ending = match config.line_ending_target {
        LineEndingTarget::Linux => LineEnding::Lf,
        LineEndingTarget::Windows => LineEnding::Crlf,
        LineEndingTarget::None => {
            return Err(anyhow::anyhow!("No line ending rewrite option set"));
        }
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
    let mut errors: Vec<String> = Vec::new();

    for rewrite_result in &rewrite_results {
        if let Some(error) = &rewrite_result.error {
            errors.push(format!(
                "Failed to rewrite file: {}: {}",
                rewrite_result.path.display(),
                error
            ));
        } else if rewrite_result.rewritten {
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

    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{}", errors.join("\n")))
    }
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
    if (config.line_ending_target == LineEndingTarget::Linux && result.is_crlf_only())
        || (config.line_ending_target == LineEndingTarget::Windows && result.is_lf_only())
    {
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

/// Copies the original file's permissions onto the replacement temp file.
/// `NamedTempFile` is created with restrictive `0600` permissions and the
/// running process's ownership, so without this the rewritten file would lose
/// its original mode (e.g. an executable script's `+x` bit).
fn preserve_permissions(temp_file: &NamedTempFile, original: &Path) -> io::Result<()> {
    let perms = std::fs::metadata(original)?.permissions();
    temp_file.as_file().set_permissions(perms)
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

/// Gets the backup path for a given file.
/// Appends `.bak` to the full filename, preserving the original extension.
/// Handles extensionless files (e.g. `Makefile` → `Makefile.bak`) and
/// dotfiles (e.g. `.gitignore` → `.gitignore.bak`).
fn get_backup_path(input_path: &Path) -> std::path::PathBuf {
    if let Some(ext) = input_path.extension() {
        input_path.with_extension(format!("{}.bak", ext.to_string_lossy()))
    } else {
        let mut name = input_path.as_os_str().to_owned();
        name.push(".bak");
        std::path::PathBuf::from(name)
    }
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

    // Create temporary file in the same directory as the input file
    let parent = input_path.parent().unwrap_or_else(|| Path::new(""));
    let mut temp_file = NamedTempFile::new_in(parent)?;

    // Stream the file at the byte level so any encoding (including non-UTF-8
    // text such as Latin-1) round-trips without loss. CRLF and lone LF are
    // normalised to the target ending; a lone CR (not followed by LF) is
    // preserved verbatim, matching the byte-level analysis stage which counts
    // neither LF nor CRLF for it.
    let infile = File::open(input_path)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, infile);
    let mut writer = BufWriter::with_capacity(BUFFER_SIZE, temp_file.as_file_mut());

    let line_ending: &[u8] = match ending {
        LineEnding::Lf => b"\n",
        LineEnding::Crlf => b"\r\n",
    };

    let mut buffer = [0u8; BUFFER_SIZE];
    let mut prev_was_cr = false;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        for &b in &buffer[..n] {
            match b {
                CR => {
                    // A preceding CR was a lone CR — emit it verbatim.
                    if prev_was_cr {
                        writer.write_all(&[CR])?;
                    }
                    prev_was_cr = true;
                }
                LF => {
                    // Both CRLF (prev_was_cr) and a lone LF map to the target.
                    writer.write_all(line_ending)?;
                    prev_was_cr = false;
                }
                other => {
                    if prev_was_cr {
                        writer.write_all(&[CR])?;
                        prev_was_cr = false;
                    }
                    writer.write_all(&[other])?;
                }
            }
        }
    }

    // A trailing lone CR at EOF is preserved verbatim.
    if prev_was_cr {
        writer.write_all(&[CR])?;
    }

    // Ensure all buffered data reaches the temp file before replacing.
    writer.flush()?;
    drop(writer);

    // Preserve the original file's permissions before replacing it
    preserve_permissions(&temp_file, input_path)?;

    // Atomically replace the original file with the temp file
    temp_file.persist(input_path)?;

    Ok(())
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
    let mut errors: Vec<String> = Vec::new();

    for removal_result in &removal_results {
        if let Some(error) = &removal_result.error {
            errors.push(format!(
                "Failed to remove BOM from {}: {}",
                removal_result.path.display(),
                error
            ));
        } else if removal_result.removed {
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

    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{}", errors.join("\n")))
    }
}

/// Processes a single file for BOM removal
#[must_use]
pub fn process_file_for_bom_removal(result: &FileAnalysis) -> BomRemovalResult {
    // Skip binary files, files without BOMs, or files with errors
    if result.is_binary || result.error.is_some() || !result.has_bom() {
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
        BomType::Utf8 => 3,
        BomType::Utf16Le | BomType::Utf16Be => 2,
        BomType::Utf32Le | BomType::Utf32Be => 4,
    };

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

    // Create temporary file in the same directory as the input file
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let mut temp_file = NamedTempFile::new_in(parent)?;

    // Open the original file for reading
    let mut input_file = File::open(path)?;

    // Skip the BOM
    let mut bom_buffer = vec![0; bom_size];
    input_file.read_exact(&mut bom_buffer)?;

    // Copy the rest of the file directly (preserving line endings)
    let mut copy_buffer = [0; BUFFER_SIZE];
    loop {
        let bytes_read = input_file.read(&mut copy_buffer)?;
        if bytes_read == 0 {
            break;
        }
        temp_file.write_all(&copy_buffer[..bytes_read])?;
    }

    // Ensure all data is written before replacing files
    temp_file.flush()?;

    // Preserve the original file's permissions before replacing it
    preserve_permissions(&temp_file, path)?;

    // Atomically replace the original file with the temp file
    temp_file.persist(path)?;

    Ok(())
}

/// Returns the set of backup paths that already exist for the given files.
///
/// Call this *before* any rewrite/BOM-removal mutates files. Backups that
/// already exist at that point were not created by this run (a stale backup
/// from an aborted run, or an unrelated user file such as a hand-written
/// `notes.txt.bak`) and must never be trashed.
#[must_use]
pub fn existing_backup_paths(results: &[FileAnalysis]) -> HashSet<PathBuf> {
    results
        .iter()
        .filter(|result| result.error.is_none())
        .map(|result| get_backup_path(&result.path))
        .filter(|backup_path| backup_path.exists())
        .collect()
}

/// Determines which backups are safe to trash: those that exist now and were
/// not present before the run (i.e. created by this run).
fn backups_eligible_for_trash<S: std::hash::BuildHasher>(
    results: &[FileAnalysis],
    preexisting: &HashSet<PathBuf, S>,
) -> Vec<PathBuf> {
    results
        .iter()
        .filter(|result| result.error.is_none())
        .map(|result| get_backup_path(&result.path))
        .filter(|backup_path| backup_path.exists() && !preexisting.contains(backup_path))
        .collect()
}

/// Moves backups created by this run to the trash.
///
/// `preexisting` is the set of backup paths that already existed before the
/// run (from [`existing_backup_paths`]); those are left untouched so an
/// unrelated or stale `.bak` is never destroyed.
///
/// # Errors
///
/// Returns an error if backup deletion fails.
pub fn trash_backup_files<S: std::hash::BuildHasher>(
    results: &[FileAnalysis],
    preexisting: &HashSet<PathBuf, S>,
) -> Result<()> {
    println!();

    let to_trash = backups_eligible_for_trash(results, preexisting);
    let mut deleted_count = 0usize;

    for backup_path in &to_trash {
        match trash::delete(backup_path) {
            Ok(()) => {
                println!("\"{}\"\tbackup moved to trash", backup_path.display());
                deleted_count += 1;
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to move backup to trash {}: {}",
                    backup_path.display(),
                    e
                ));
            }
        }
    }

    let preserved = preexisting.len();
    if preserved > 0 {
        println!(
            "Moved {deleted_count} backup file(s) to trash, kept {preserved} pre-existing backup file(s)"
        );
    } else {
        println!("Moved {deleted_count} backup file(s) to trash");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_path_for_file_with_extension() {
        let path = std::path::Path::new("test.txt");
        let backup = get_backup_path(path);
        assert_eq!(backup, std::path::Path::new("test.txt.bak"));
    }

    #[test]
    fn test_backup_path_for_extensionless_file() {
        let path = std::path::Path::new("Makefile");
        let backup = get_backup_path(path);
        assert_eq!(
            backup,
            std::path::Path::new("Makefile.bak"),
            "extensionless file should get .bak suffix, not ..bak"
        );
    }

    #[test]
    fn test_backup_path_for_dotfile() {
        let path = std::path::Path::new(".gitignore");
        let backup = get_backup_path(path);
        assert_eq!(backup, std::path::Path::new(".gitignore.bak"));
    }

    fn analysis_for(path: &Path) -> FileAnalysis {
        FileAnalysis {
            path: path.to_path_buf(),
            lf_count: 1,
            crlf_count: 0,
            cr_count: 0,
            bom_checked: false,
            bom_type: None,
            is_binary: false,
            error: None,
        }
    }

    #[test]
    fn test_preexisting_backup_is_not_eligible_for_trash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, b"data\n").expect("write file");
        // An unrelated/pre-existing backup the user owns.
        let backup = get_backup_path(&file);
        std::fs::write(&backup, b"user data\n").expect("write backup");

        let results = vec![analysis_for(&file)];
        let preexisting = existing_backup_paths(&results);
        assert!(
            preexisting.contains(&backup),
            "pre-existing backup should be snapshotted"
        );

        let eligible = backups_eligible_for_trash(&results, &preexisting);
        assert!(
            eligible.is_empty(),
            "a pre-existing backup must never be trashed, got {eligible:?}"
        );
    }

    #[test]
    fn test_run_created_backup_is_eligible_for_trash() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, b"data\n").expect("write file");

        let results = vec![analysis_for(&file)];
        // Snapshot before the backup exists (as main does before mutation).
        let preexisting = existing_backup_paths(&results);
        assert!(preexisting.is_empty());

        // Simulate the run creating the backup.
        let backup = get_backup_path(&file);
        std::fs::write(&backup, b"data\n").expect("write backup");

        let eligible = backups_eligible_for_trash(&results, &preexisting);
        assert_eq!(
            eligible,
            vec![backup],
            "a backup created during the run should be eligible for trash"
        );
    }
}
