#[cfg(test)]
mod tests {
    use crate::types::{BomType, FileAnalysis};
    use std::path::PathBuf;

    #[test]
    fn test_binary_file_analysis_has_is_binary_flag() {
        let binary = FileAnalysis {
            path: PathBuf::from("image.png"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: None,
            is_binary: true,
            error: None,
        };
        assert!(binary.is_binary, "binary file should have is_binary = true");

        let text = FileAnalysis {
            path: PathBuf::from("readme.txt"),
            lf_count: 10,
            crlf_count: 0,
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
        // Test with BomType::None - should return false
        let analysis_none = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::None),
            is_binary: false,
            error: None,
        };
        assert!(
            !analysis_none.has_bom(),
            "BomType::None should return false"
        );

        // Test with BomType::Utf8 - should return true
        let analysis_utf8 = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf8),
            is_binary: false,
            error: None,
        };
        assert!(analysis_utf8.has_bom(), "BomType::Utf8 should return true");

        // Test with BomType::Utf16Le - should return true
        let analysis_utf16_le = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf16Le),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf16_le.has_bom(),
            "BomType::Utf16Le should return true"
        );

        // Test with BomType::Utf16Be - should return true
        let analysis_utf16_be = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf16Be),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf16_be.has_bom(),
            "BomType::Utf16Be should return true"
        );

        // Test with BomType::Utf32Le - should return true
        let analysis_utf32_le = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf32Le),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf32_le.has_bom(),
            "BomType::Utf32Le should return true"
        );

        // Test with BomType::Utf32Be - should return true
        let analysis_utf32_be = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf32Be),
            is_binary: false,
            error: None,
        };
        assert!(
            analysis_utf32_be.has_bom(),
            "BomType::Utf32Be should return true"
        );

        // Test with None bom_type - should return false
        let analysis_no_bom_check = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: None,
            is_binary: false,
            error: None,
        };
        assert!(
            !analysis_no_bom_check.has_bom(),
            "None bom_type should return false"
        );
    }

    /// Test BOM type string conversion
    /// This ensures the BOM info display logic generates correct static strings
    #[test]
    fn test_bom_type_to_string() {
        assert_eq!(BomType::None.to_string(), "none");
        assert_eq!(BomType::Utf8.to_string(), "UTF-8");
        assert_eq!(BomType::Utf16Le.to_string(), "UTF-16 LE");
        assert_eq!(BomType::Utf16Be.to_string(), "UTF-16 BE");
        assert_eq!(BomType::Utf32Le.to_string(), "UTF-32 LE");
        assert_eq!(BomType::Utf32Be.to_string(), "UTF-32 BE");
    }

    /// Test edge case scenarios for `has_bom` method
    #[test]
    fn test_has_bom_edge_cases() {
        // Edge case: Multiple calls should be consistent
        let analysis = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf8),
            is_binary: false,
            error: None,
        };

        // Multiple calls should return the same result
        assert!(analysis.has_bom());
        assert!(analysis.has_bom());
        assert!(analysis.has_bom());

        // Test with BomType::None multiple times
        let analysis_none = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::None),
            is_binary: false,
            error: None,
        };

        assert!(!analysis_none.has_bom());
        assert!(!analysis_none.has_bom());
        assert!(!analysis_none.has_bom());
    }

    /// Test that the original unsafe scenario would have been handled correctly
    /// This is a regression test for the original bug
    #[test]
    fn test_original_bug_scenario() {
        // The original bug was: self.bom_type.is_some() && self.bom_type.unwrap() != BomType::None
        // This would panic if bom_type was Some(BomType::None) due to the unsafe unwrap

        // Scenario that would have caused the original bug:
        let analysis_that_would_panic = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::None), // This is Some, but contains BomType::None
            is_binary: false,
            error: None,
        };

        // With the fixed implementation using matches!, this should return false safely
        assert!(
            !analysis_that_would_panic.has_bom(),
            "Fixed implementation should safely handle Some(BomType::None)"
        );
    }
}
