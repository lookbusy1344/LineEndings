use std::path::PathBuf;

/// Represents the type of BOM detected in a file.
/// Note: `Option<BomType>` in `FileAnalysis::bom_type` uses `None` to mean "no BOM found".
/// Use `FileAnalysis::bom_checked` to distinguish "no BOM found" from "check not requested".
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BomType {
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
}

impl std::fmt::Display for BomType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BomType::Utf8 => write!(f, "UTF-8"),
            BomType::Utf16Le => write!(f, "UTF-16 LE"),
            BomType::Utf16Be => write!(f, "UTF-16 BE"),
            BomType::Utf32Le => write!(f, "UTF-32 LE"),
            BomType::Utf32Be => write!(f, "UTF-32 BE"),
        }
    }
}

/// Represents line ending types
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LineEnding {
    Lf,   // Unix/Linux style (\n)
    Crlf, // Windows style (\r\n)
}

/// Target line ending for file conversion
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LineEndingTarget {
    None,    // No conversion
    Linux,   // Convert to LF
    Windows, // Convert to CRLF
}

/// Configuration settings parsed from command line arguments
#[allow(clippy::struct_excessive_bools)]
pub struct ConfigSettings {
    pub case_sensitive: bool,
    pub line_ending_target: LineEndingTarget,
    pub check_bom: bool,
    pub remove_bom: bool,
    pub recursive: bool,
    pub no_trash: bool,
    pub supplied_paths: Vec<String>,
    pub folder: Option<String>,
}

impl ConfigSettings {
    /// Returns true if any line ending rewrite option is set
    #[must_use]
    pub fn has_rewrite_option(&self) -> bool {
        self.line_ending_target != LineEndingTarget::None
    }
}

/// Stores the results of line ending analysis for a file
#[derive(Debug, Clone)]
pub struct FileAnalysis {
    pub path: PathBuf,
    pub lf_count: usize,
    pub crlf_count: usize,
    /// `true` if the BOM check was requested (--bom or --remove-bom flags).
    /// Distinguish "no BOM found" (`bom_checked = true, bom_type = None`) from
    /// "check not requested" (`bom_checked = false`).
    pub bom_checked: bool,
    /// The BOM type found, or `None` if no BOM was found (only valid when `bom_checked = true`).
    pub bom_type: Option<BomType>,
    pub is_binary: bool,
    pub error: Option<String>,
}

impl FileAnalysis {
    /// Returns true if the file has mixed line endings
    #[must_use]
    pub fn has_mixed_line_endings(&self) -> bool {
        self.lf_count > 0 && self.crlf_count > 0
    }

    /// Returns true if the file has only LF line endings
    #[must_use]
    pub fn is_lf_only(&self) -> bool {
        self.lf_count > 0 && self.crlf_count == 0
    }

    /// Returns true if the file has only CRLF line endings
    #[must_use]
    pub fn is_crlf_only(&self) -> bool {
        self.lf_count == 0 && self.crlf_count > 0
    }

    /// Returns true if the BOM check ran and a BOM was found
    #[must_use]
    pub fn has_bom(&self) -> bool {
        self.bom_type.is_some()
    }
}

/// Stores the result of a file rewrite operation
#[derive(Debug, Clone)]
pub struct RewriteResult {
    pub path: PathBuf,
    pub rewritten: bool,
    pub error: Option<String>,
}

/// Stores the result of a BOM removal operation
#[derive(Debug, Clone)]
pub struct BomRemovalResult {
    pub path: PathBuf,
    pub removed: bool,
    pub bom_type: Option<BomType>,
    pub error: Option<String>,
}
