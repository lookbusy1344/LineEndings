#[cfg(test)]
mod tests {
    use crate::types::{BomType, FileAnalysis};
    use std::path::PathBuf;

    #[test]
    fn test_bom_checked_flag_distinguishes_not_checked_from_no_bom() {
        // bom_checked = false means the check was not requested
        let not_checked = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 5,
            crlf_count: 0,
            bom_type: None,
            bom_checked: false,
            is_binary: false,
            error: None,
        };
        assert!(
            !not_checked.bom_checked,
            "bom_checked should be false when not requested"
        );
        assert!(
            !not_checked.has_bom(),
            "has_bom() should be false when check not requested"
        );

        // bom_checked = true, bom_type = None means checked, no BOM found
        let checked_no_bom = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 5,
            crlf_count: 0,
            bom_type: None,
            bom_checked: true,
            is_binary: false,
            error: None,
        };
        assert!(
            checked_no_bom.bom_checked,
            "bom_checked should be true when check ran"
        );
        assert!(
            !checked_no_bom.has_bom(),
            "has_bom() should be false when no BOM found"
        );

        // bom_checked = true, bom_type = Some means BOM found
        let checked_with_bom = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 5,
            crlf_count: 0,
            bom_type: Some(BomType::Utf8),
            bom_checked: true,
            is_binary: false,
            error: None,
        };
        assert!(
            checked_with_bom.bom_checked,
            "bom_checked should be true when check ran"
        );
        assert!(
            checked_with_bom.has_bom(),
            "has_bom() should be true when BOM found"
        );
    }

    #[test]
    fn test_binary_file_analysis_has_is_binary_flag() {
        let binary = FileAnalysis {
            path: PathBuf::from("image.png"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: false,
            bom_type: None,
            is_binary: true,
            error: None,
        };
        assert!(binary.is_binary, "binary file should have is_binary = true");

        let text = FileAnalysis {
            path: PathBuf::from("readme.txt"),
            lf_count: 10,
            crlf_count: 0,
            bom_checked: false,
            bom_type: None,
            is_binary: false,
            error: None,
        };
        assert!(!text.is_binary, "text file should have is_binary = false");
    }

    /// Test the `has_bom()` method with different `BomType` variants
    /// This test ensures the bug fix for unsafe unwrap operation works correctly
    #[test]
    #[allow(clippy::similar_names)] // BOM variant names are intentionally similar
    fn test_has_bom_method_with_different_bom_types() {
        // No BOM found (check ran, nothing found) — bom_type: None
        let no_bom = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: None,
            is_binary: false,
            error: None,
        };
        assert!(!no_bom.has_bom(), "no BOM found should return false");

        // BOM check not requested — bom_type: None, bom_checked: false
        let not_checked = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: false,
            bom_type: None,
            is_binary: false,
            error: None,
        };
        assert!(!not_checked.has_bom(), "unchecked file should return false");

        // UTF-8 BOM
        let analysis_utf8 = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: Some(BomType::Utf8),
            is_binary: false,
            error: None,
        };
        assert!(analysis_utf8.has_bom(), "BomType::Utf8 should return true");

        // UTF-16 LE BOM
        let analysis_utf16_le = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: Some(BomType::Utf16Le),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf16_le.has_bom(),
            "BomType::Utf16Le should return true"
        );

        // UTF-16 BE BOM
        let analysis_utf16_be = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: Some(BomType::Utf16Be),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf16_be.has_bom(),
            "BomType::Utf16Be should return true"
        );

        // UTF-32 LE BOM
        let analysis_utf32_le = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: Some(BomType::Utf32Le),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf32_le.has_bom(),
            "BomType::Utf32Le should return true"
        );

        // UTF-32 BE BOM
        let analysis_utf32_be = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: Some(BomType::Utf32Be),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf32_be.has_bom(),
            "BomType::Utf32Be should return true"
        );
    }

    /// Test BOM type string conversion
    #[test]
    fn test_bom_type_to_string() {
        assert_eq!(BomType::Utf8.to_string(), "UTF-8");
        assert_eq!(BomType::Utf16Le.to_string(), "UTF-16 LE");
        assert_eq!(BomType::Utf16Be.to_string(), "UTF-16 BE");
        assert_eq!(BomType::Utf32Le.to_string(), "UTF-32 LE");
        assert_eq!(BomType::Utf32Be.to_string(), "UTF-32 BE");
    }

    /// Test edge case scenarios for `has_bom` method — idempotency
    #[test]
    fn test_has_bom_is_idempotent() {
        let with_bom = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: Some(BomType::Utf8),
            is_binary: false,
            error: None,
        };
        assert!(with_bom.has_bom());
        assert!(with_bom.has_bom());

        let without_bom = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_checked: true,
            bom_type: None,
            is_binary: false,
            error: None,
        };
        assert!(!without_bom.has_bom());
        assert!(!without_bom.has_bom());
    }
}
