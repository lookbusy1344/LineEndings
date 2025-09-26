use std::path::PathBuf;

/// Represents the type of BOM detected in a file
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BomType {
    None,
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
}

impl BomType {
    #[must_use]
    pub fn to_string(self) -> &'static str {
        match self {
            BomType::None => "none",
            BomType::Utf8 => "UTF-8",
            BomType::Utf16Le => "UTF-16 LE",
            BomType::Utf16Be => "UTF-16 BE",
            BomType::Utf32Le => "UTF-32 LE",
            BomType::Utf32Be => "UTF-32 BE",
        }
    }
}

/// Represents line ending types
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LineEnding {
    Lf,   // Unix/Linux style (\n)
    Crlf, // Windows style (\r\n)
}

/// Configuration settings parsed from command line arguments
#[allow(clippy::struct_excessive_bools)]
pub struct ConfigSettings {
    pub case_sensitive: bool,
    pub set_linux: bool,
    pub set_windows: bool,
    pub check_bom: bool,
    pub remove_bom: bool,
    pub recursive: bool,
    pub supplied_paths: Vec<String>,
    pub folder: Option<String>,
}

impl ConfigSettings {
    /// Returns true if any line ending rewrite option is set
    #[must_use]
    pub fn has_rewrite_option(&self) -> bool {
        self.set_linux || self.set_windows
    }
}

/// Stores the results of line ending analysis for a file
#[derive(Debug, Clone)]
pub struct FileAnalysis {
    pub path: PathBuf,
    pub lf_count: usize,
    pub crlf_count: usize,
    pub bom_type: Option<BomType>,
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

    /// Returns true if the file has a BOM
    #[must_use]
    pub fn has_bom(&self) -> bool {
        matches!(self.bom_type, Some(bom_type) if bom_type != BomType::None)
    }
}

/// Stores the result of a file rewrite operation
#[derive(Debug, Clone)]
pub struct RewriteResult {
    pub path: PathBuf,
    pub rewritten: bool,
    pub error: Option<String>,
}
