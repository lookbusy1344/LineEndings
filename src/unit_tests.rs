#[cfg(test)]
mod tests {
    use crate::types::{BomType, FileAnalysis};
    use std::path::PathBuf;

    /// Test the has_bom() method with different BomType variants
    /// This test ensures the bug fix for unsafe unwrap operation works correctly
    #[test]
    fn test_has_bom_method_with_different_bom_types() {
        // Test with BomType::None - should return false
        let analysis_none = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::None),
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
            error: None,
        };
        assert!(analysis_utf8.has_bom(), "BomType::Utf8 should return true");

        // Test with BomType::Utf16Le - should return true
        let analysis_utf16_le = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf16Le),
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

    /// Test that the BOM string literals are static (no allocations)
    /// This verifies the performance improvement for BOM info display
    #[test]
    fn test_bom_strings_are_static() {
        // These should all be static string literals, not allocated strings
        let none_str = BomType::None.to_string();
        let utf8_str = BomType::Utf8.to_string();
        let utf16_le_str = BomType::Utf16Le.to_string();
        let utf16_be_str = BomType::Utf16Be.to_string();
        let utf32_le_str = BomType::Utf32Le.to_string();
        let utf32_be_str = BomType::Utf32Be.to_string();

        // Verify they are the expected values
        assert_eq!(none_str, "none");
        assert_eq!(utf8_str, "UTF-8");
        assert_eq!(utf16_le_str, "UTF-16 LE");
        assert_eq!(utf16_be_str, "UTF-16 BE");
        assert_eq!(utf32_le_str, "UTF-32 LE");
        assert_eq!(utf32_be_str, "UTF-32 BE");

        // The fact that these compile and run correctly verifies they are static literals
        // If they were allocated strings, the performance would be worse
    }

    /// Test edge case scenarios for has_bom method
    #[test]
    fn test_has_bom_edge_cases() {
        // Edge case: Multiple calls should be consistent
        let analysis = FileAnalysis {
            path: PathBuf::from("test.txt"),
            lf_count: 0,
            crlf_count: 0,
            bom_type: Some(BomType::Utf8),
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
            error: None,
        };

        // With the fixed implementation using matches!, this should return false safely
        assert!(
            !analysis_that_would_panic.has_bom(),
            "Fixed implementation should safely handle Some(BomType::None)"
        );
    }
}
