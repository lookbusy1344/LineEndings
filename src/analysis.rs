use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::types::{BomType, ConfigSettings, FileAnalysis};

// Define constants for line ending characters
const BUFFER_SIZE: usize = 4096; // 4KB buffer for more efficient reading
const LF: u8 = b'\n';
const CR: u8 = b'\r';

// Define BOM (Byte Order Marker) constants
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];
const UTF32_LE_BOM: &[u8] = &[0xFF, 0xFE, 0x00, 0x00];
const UTF32_BE_BOM: &[u8] = &[0x00, 0x00, 0xFE, 0xFF];

/// Analyzes a single file for line endings and BOM
pub fn analyze_file(path: impl AsRef<Path>, config: &ConfigSettings) -> FileAnalysis {
    // Only detect BOM if check_bom is true
    let bom_type: Option<BomType> = if config.check_bom {
        match detect_bom(&path) {
            Ok(bom) => Some(bom),
            Err(e) => {
                return FileAnalysis {
                    path: path.as_ref().to_path_buf(),
                    lf_count: 0,
                    crlf_count: 0,
                    bom_type: None,
                    error: Some(format!("Failed to detect BOM: {e}")),
                };
            }
        }
    } else {
        // Skip BOM detection
        None
    };

    // Then count line endings
    match count_line_endings_in_file(&path, bom_type) {
        Ok((lf_count, crlf_count)) => FileAnalysis {
            path: path.as_ref().to_path_buf(),
            lf_count,
            crlf_count,
            bom_type,
            error: None,
        },
        Err(e) => FileAnalysis {
            path: path.as_ref().to_path_buf(),
            lf_count: 0,
            crlf_count: 0,
            bom_type,
            error: Some(e.to_string()),
        },
    }
}

/// Opens a file and counts the line endings
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read.
pub fn count_line_endings_in_file(
    path: impl AsRef<Path>,
    bom_type: Option<BomType>,
) -> Result<(usize, usize)> {
    let file = File::open(&path)?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let (lf_count, crlf_count) = count_line_endings(reader)?;

    let file_name = path.as_ref().display();
    let line_endings;

    if lf_count == 0 && crlf_count == 0 {
        line_endings = String::from("None");
    } else if lf_count > 0 && crlf_count == 0 {
        line_endings = format!("LF {lf_count}");
    } else if lf_count == 0 && crlf_count > 0 {
        line_endings = format!("CRLF {crlf_count}");
    } else {
        line_endings = format!("Mixed LF {lf_count}, CRLF {crlf_count}");
    }

    let bom_info = match bom_type {
        None => "", // no BOM check requested
        Some(bom) => match bom {
            BomType::None => ", BOM: None",
            BomType::Utf8 => ", BOM: UTF-8",
            BomType::Utf16Le => ", BOM: UTF-16 LE",
            BomType::Utf16Be => ", BOM: UTF-16 BE",
            BomType::Utf32Le => ", BOM: UTF-32 LE",
            BomType::Utf32Be => ", BOM: UTF-32 BE",
        },
    };

    // display results immediately, as Rayon is still running tasks in parallel
    // needs to be a single println! to avoid interleaving output
    println!("\"{file_name}\"\t{line_endings}{bom_info}");

    Ok((lf_count, crlf_count))
}

/// Counts LF and Crlf line endings in a reader
///
/// # Errors
///
/// Returns an error if reading from the reader fails.
pub fn count_line_endings<R: Read>(mut reader: BufReader<R>) -> Result<(usize, usize)> {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut lf_count = 0;
    let mut crlf_count = 0;
    let mut prev_was_cr = false;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        for &b in &buffer[..n] {
            match b {
                CR => prev_was_cr = true,
                LF => {
                    if prev_was_cr {
                        crlf_count += 1;
                    } else {
                        lf_count += 1;
                    }
                    prev_was_cr = false;
                }
                _ => prev_was_cr = false,
            }
        }
    }

    Ok((lf_count, crlf_count))
}

/// Detects BOM (Byte Order Marker) in a file
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read.
pub fn detect_bom(file_path: impl AsRef<Path>) -> Result<BomType> {
    let mut file = File::open(file_path)?;
    let mut buffer = [0; 4]; // Maximum BOM size is 4 bytes (UTF-32)

    // Read up to 4 bytes from the beginning of the file
    let bytes_read = file.read(&mut buffer)?;

    if bytes_read >= 3 && buffer[0..3] == UTF8_BOM[..] {
        return Ok(BomType::Utf8);
    } else if bytes_read >= 4 && buffer[0..4] == UTF32_LE_BOM[..] {
        return Ok(BomType::Utf32Le);
    } else if bytes_read >= 4 && buffer[0..4] == UTF32_BE_BOM[..] {
        return Ok(BomType::Utf32Be);
    } else if bytes_read >= 2 && buffer[0..2] == UTF16_LE_BOM[..] {
        return Ok(BomType::Utf16Le);
    } else if bytes_read >= 2 && buffer[0..2] == UTF16_BE_BOM[..] {
        return Ok(BomType::Utf16Be);
    }

    Ok(BomType::None)
}
