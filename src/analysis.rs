use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::types::{BomType, ConfigSettings, FileAnalysis, LineEndingCounts};

// Define constants for line ending characters
const BUFFER_SIZE: usize = 4096; // 4KB buffer for more efficient reading
const BINARY_CHECK_SIZE: usize = 8192; // 8KB for binary detection
const LF: u8 = b'\n';
const CR: u8 = b'\r';

// Define BOM (Byte Order Marker) constants
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];
const UTF32_LE_BOM: &[u8] = &[0xFF, 0xFE, 0x00, 0x00];
const UTF32_BE_BOM: &[u8] = &[0x00, 0x00, 0xFE, 0xFF];

/// Outcome of a single streaming pass over a file's bytes.
enum ScanOutcome {
    /// File was classified as binary; line-ending/BOM data is not meaningful.
    Binary,
    /// File is text, with its line-ending tallies and (optionally) BOM.
    Text {
        counts: LineEndingCounts,
        bom_type: Option<BomType>,
    },
}

/// Analyzes a single file for line endings and BOM in one pass over its bytes.
pub fn analyze_file(path: impl AsRef<Path>, config: &ConfigSettings) -> FileAnalysis {
    let path_buf = path.as_ref().to_path_buf();

    let error_analysis = |message: String| FileAnalysis {
        path: path_buf.clone(),
        lf_count: 0,
        crlf_count: 0,
        cr_count: 0,
        bom_checked: false,
        bom_type: None,
        is_binary: false,
        error: Some(message),
    };

    let file = match File::open(&path) {
        Ok(file) => file,
        Err(e) => return error_analysis(format!("Failed to open file: {e}")),
    };

    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    match scan_reader(&mut reader, config.check_bom) {
        Ok(ScanOutcome::Binary) => FileAnalysis {
            path: path_buf,
            lf_count: 0,
            crlf_count: 0,
            cr_count: 0,
            bom_checked: false,
            bom_type: None,
            is_binary: true,
            error: None,
        },
        Ok(ScanOutcome::Text { counts, bom_type }) => FileAnalysis {
            path: path_buf,
            lf_count: counts.lf,
            crlf_count: counts.crlf,
            cr_count: counts.cr,
            bom_checked: config.check_bom,
            bom_type,
            is_binary: false,
            error: None,
        },
        Err(e) => error_analysis(e.to_string()),
    }
}

/// Performs binary detection, BOM detection, and line-ending counting in a
/// single streaming pass, replacing three separate opens of the same file.
///
/// A file is classified as binary if it contains a null byte or more than 30%
/// non-printable bytes within the first [`BINARY_CHECK_SIZE`] bytes. Detection
/// short-circuits as soon as the file is known to be binary, so large binary
/// files are not scanned in full.
fn scan_reader<R: Read>(reader: &mut BufReader<R>, check_bom: bool) -> Result<ScanOutcome> {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut counts = LineEndingCounts::default();
    let mut prev_was_cr = false;

    // Binary-detection state, confined to the first BINARY_CHECK_SIZE bytes.
    let mut head_examined = 0usize;
    let mut non_printable = 0usize;
    let mut head_decided = false;

    // First up-to-4 bytes, for BOM detection.
    let mut bom_buf = [0u8; 4];
    let mut bom_len = 0usize;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        for &b in &buffer[..n] {
            if bom_len < bom_buf.len() {
                bom_buf[bom_len] = b;
                bom_len += 1;
            }

            // Binary detection over the leading window only.
            if !head_decided {
                if b == 0 {
                    return Ok(ScanOutcome::Binary);
                }
                if !is_text_byte(b) {
                    non_printable += 1;
                }
                head_examined += 1;
                if head_examined >= BINARY_CHECK_SIZE {
                    if non_printable > head_examined * 30 / 100 {
                        return Ok(ScanOutcome::Binary);
                    }
                    head_decided = true;
                }
            }

            match b {
                CR => {
                    if prev_was_cr {
                        counts.cr += 1;
                    }
                    prev_was_cr = true;
                }
                LF => {
                    if prev_was_cr {
                        counts.crlf += 1;
                    } else {
                        counts.lf += 1;
                    }
                    prev_was_cr = false;
                }
                _ => {
                    if prev_was_cr {
                        counts.cr += 1;
                    }
                    prev_was_cr = false;
                }
            }
        }
    }

    // A trailing CR at EOF is a lone CR.
    if prev_was_cr {
        counts.cr += 1;
    }

    // For files shorter than the head window, apply the ratio test at EOF.
    if !head_decided && head_examined > 0 && non_printable > head_examined * 30 / 100 {
        return Ok(ScanOutcome::Binary);
    }

    let bom_type = if check_bom {
        bom_from_bytes(&bom_buf[..bom_len])
    } else {
        None
    };

    Ok(ScanOutcome::Text { counts, bom_type })
}

/// Opens a file and counts the line endings
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read.
pub fn count_line_endings_in_file(path: impl AsRef<Path>) -> Result<LineEndingCounts> {
    let file = File::open(&path)?;
    let reader = BufReader::with_capacity(BUFFER_SIZE, file);
    count_line_endings(reader)
}

/// Counts LF, CRLF, and lone CR line endings in a reader.
///
/// A lone CR (a carriage return not immediately followed by LF, i.e. classic
/// Mac style) is counted in `cr`. CRLF pairs count once in `crlf`, never in
/// `cr`.
///
/// # Errors
///
/// Returns an error if reading from the reader fails.
pub fn count_line_endings<R: Read>(mut reader: BufReader<R>) -> Result<LineEndingCounts> {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut counts = LineEndingCounts::default();
    let mut prev_was_cr = false;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        for &b in &buffer[..n] {
            match b {
                CR => {
                    // A preceding CR with no LF was a lone CR.
                    if prev_was_cr {
                        counts.cr += 1;
                    }
                    prev_was_cr = true;
                }
                LF => {
                    if prev_was_cr {
                        counts.crlf += 1;
                    } else {
                        counts.lf += 1;
                    }
                    prev_was_cr = false;
                }
                _ => {
                    if prev_was_cr {
                        counts.cr += 1;
                    }
                    prev_was_cr = false;
                }
            }
        }
    }

    // A trailing CR at EOF is a lone CR.
    if prev_was_cr {
        counts.cr += 1;
    }

    Ok(counts)
}

/// Detects BOM (Byte Order Marker) in a file.
/// Returns `Ok(Some(bom_type))` if a BOM was found, `Ok(None)` if no BOM was found.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or read.
pub fn detect_bom(file_path: impl AsRef<Path>) -> Result<Option<BomType>> {
    let mut file = File::open(file_path)?;
    let mut buffer = [0; 4]; // Maximum BOM size is 4 bytes (UTF-32)

    // Read up to 4 bytes from the beginning of the file
    let bytes_read = file.read(&mut buffer)?;

    Ok(bom_from_bytes(&buffer[..bytes_read]))
}

/// Identifies a BOM from the leading bytes of a file.
/// Checks longer BOMs first to avoid false matches (UTF-32 LE starts with the
/// same two bytes as UTF-16 LE).
fn bom_from_bytes(head: &[u8]) -> Option<BomType> {
    if head.len() >= 4 && head[0..4] == *UTF32_LE_BOM {
        Some(BomType::Utf32Le)
    } else if head.len() >= 4 && head[0..4] == *UTF32_BE_BOM {
        Some(BomType::Utf32Be)
    } else if head.len() >= 3 && head[0..3] == *UTF8_BOM {
        Some(BomType::Utf8)
    } else if head.len() >= 2 && head[0..2] == *UTF16_LE_BOM {
        Some(BomType::Utf16Le)
    } else if head.len() >= 2 && head[0..2] == *UTF16_BE_BOM {
        Some(BomType::Utf16Be)
    } else {
        None
    }
}

/// Checks if a byte is a typical text character
fn is_text_byte(b: u8) -> bool {
    // Printable ASCII (32-126), or common whitespace
    (32..=126).contains(&b) || b == b'\t' || b == b'\n' || b == b'\r' || b >= 128 // Allow UTF-8
}
