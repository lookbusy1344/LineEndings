use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

use line_endings::analysis::{analyze_file, count_line_endings_in_file, detect_bom};
use line_endings::processing::{remove_bom_from_files, rewrite_files};
use line_endings::types::{BomType, ConfigSettings};

/// Helper function to create a temporary directory and copy test files into it
fn setup_test_environment() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let test_folder = Path::new("test_folder");

    // Copy all test files from test_folder to temporary directory
    copy_dir_recursive(test_folder, temp_dir.path()).expect("Failed to copy test files");

    temp_dir
}

/// Recursively copy directory contents
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}

/// Helper function to create test config with default settings
fn create_test_config() -> ConfigSettings {
    ConfigSettings {
        case_sensitive: false,
        set_linux: false,
        set_windows: false,
        check_bom: true,
        remove_bom: false,
        recursive: true,
        delete_backups: false,
        supplied_paths: vec![],
        folder: None,
    }
}

#[test]
fn test_bom_detection() {
    let temp_dir = setup_test_environment();
    let has_bom_path = temp_dir.path().join("has_bom.txt");

    // Test UTF-8 BOM detection
    let bom_type = detect_bom(&has_bom_path).expect("Failed to detect BOM");
    assert_eq!(bom_type, BomType::Utf8);

    // Test file without BOM
    let no_bom_path = temp_dir.path().join("test_linux.txt");
    let bom_type = detect_bom(&no_bom_path).expect("Failed to detect BOM");
    assert_eq!(bom_type, BomType::None);
}

#[test]
fn test_line_ending_analysis() {
    let temp_dir = setup_test_environment();
    let config = create_test_config();

    // Test Windows line endings (CRLF)
    let windows_file = temp_dir.path().join("test_windows.txt");
    let analysis = analyze_file(&windows_file, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert!(
        analysis.is_crlf_only(),
        "Windows file should have only CRLF"
    );
    assert!(analysis.crlf_count > 0, "Should have CRLF line endings");
    assert_eq!(analysis.lf_count, 0, "Should not have LF line endings");

    // Test Linux line endings (LF)
    let linux_file = temp_dir.path().join("test_linux.txt");
    let analysis = analyze_file(&linux_file, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert!(analysis.is_lf_only(), "Linux file should have only LF");
    assert!(analysis.lf_count > 0, "Should have LF line endings");
    assert_eq!(analysis.crlf_count, 0, "Should not have CRLF line endings");

    // Test mixed line endings
    let mixed_file = temp_dir.path().join("test_lines.txt");
    let analysis = analyze_file(&mixed_file, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert!(
        analysis.has_mixed_line_endings(),
        "File should have mixed line endings"
    );
    assert!(analysis.lf_count > 0, "Should have LF line endings");
    assert!(analysis.crlf_count > 0, "Should have CRLF line endings");
}

#[test]
fn test_bom_analysis() {
    let temp_dir = setup_test_environment();
    let config = create_test_config();

    // Test file with BOM
    let has_bom_path = temp_dir.path().join("has_bom.txt");
    let analysis = analyze_file(&has_bom_path, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert!(analysis.has_bom(), "File should have BOM");
    assert_eq!(
        analysis.bom_type,
        Some(BomType::Utf8),
        "Should detect UTF-8 BOM"
    );

    // Test file without BOM
    let no_bom_path = temp_dir.path().join("test_linux.txt");
    let analysis = analyze_file(&no_bom_path, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert!(!analysis.has_bom(), "File should not have BOM");
}

#[test]
fn test_bom_check_disabled() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.check_bom = false; // Disable BOM checking

    let has_bom_path = temp_dir.path().join("has_bom.txt");
    let analysis = analyze_file(&has_bom_path, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert_eq!(
        analysis.bom_type, None,
        "BOM should not be checked when disabled"
    );
}

#[test]
fn test_subdirectory_files() {
    let temp_dir = setup_test_environment();
    let config = create_test_config();

    // Test BOM file in subdirectory
    let sub_bom_path = temp_dir.path().join("sub_folder").join("has_bom.txt");
    let analysis = analyze_file(&sub_bom_path, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert!(analysis.has_bom(), "Subdirectory BOM file should have BOM");
    assert_eq!(
        analysis.bom_type,
        Some(BomType::Utf8),
        "Should detect UTF-8 BOM"
    );

    // Test mixed line endings file in subdirectory
    let sub_mixed_path = temp_dir.path().join("sub_folder").join("test_lines.txt");
    let analysis = analyze_file(&sub_mixed_path, &config);
    assert!(analysis.error.is_none(), "Analysis should not have errors");
    assert!(
        analysis.has_mixed_line_endings(),
        "Subdirectory file should have mixed line endings"
    );
}

#[test]
fn test_line_ending_conversion_to_windows() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.set_windows = true; // Enable Windows line ending conversion

    // Convert Linux file to Windows line endings
    let linux_file = temp_dir.path().join("test_linux.txt");
    let original_analysis = analyze_file(&linux_file, &config);
    assert!(
        original_analysis.is_lf_only(),
        "Original file should have LF only"
    );

    let analyses = vec![original_analysis];
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "File rewrite should succeed");

    // Verify conversion
    let converted_analysis = analyze_file(&linux_file, &config);
    assert!(
        converted_analysis.is_crlf_only(),
        "Converted file should have CRLF only"
    );
    assert!(
        converted_analysis.crlf_count > 0,
        "Should have CRLF line endings after conversion"
    );
}

#[test]
fn test_line_ending_conversion_to_linux() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.set_linux = true; // Enable Linux line ending conversion

    // Convert Windows file to Linux line endings
    let windows_file = temp_dir.path().join("test_windows.txt");
    let original_analysis = analyze_file(&windows_file, &config);
    assert!(
        original_analysis.is_crlf_only(),
        "Original file should have CRLF only"
    );

    let analyses = vec![original_analysis];
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "File rewrite should succeed");

    // Verify conversion
    let converted_analysis = analyze_file(&windows_file, &config);
    assert!(
        converted_analysis.is_lf_only(),
        "Converted file should have LF only"
    );
    assert!(
        converted_analysis.lf_count > 0,
        "Should have LF line endings after conversion"
    );
}

#[test]
fn test_bom_removal() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.remove_bom = true; // Enable BOM removal

    // Test BOM removal
    let has_bom_path = temp_dir.path().join("has_bom.txt");
    let original_analysis = analyze_file(&has_bom_path, &config);
    assert!(original_analysis.has_bom(), "Original file should have BOM");

    let analyses = vec![original_analysis];
    let result = remove_bom_from_files(&config, &analyses);
    assert!(result.is_ok(), "BOM removal should succeed");

    // Verify BOM removal
    let converted_analysis = analyze_file(&has_bom_path, &config);
    assert!(!converted_analysis.has_bom(), "BOM should be removed");
    assert_eq!(
        converted_analysis.bom_type,
        Some(BomType::None),
        "BOM type should be None after removal"
    );
}

#[test]
fn test_combined_bom_removal_and_line_ending_conversion() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.remove_bom = true;
    config.set_linux = true; // Convert to LF and remove BOM

    let has_bom_path = temp_dir.path().join("has_bom.txt");
    let original_analysis = analyze_file(&has_bom_path, &config);
    assert!(original_analysis.has_bom(), "Original file should have BOM");
    assert!(
        original_analysis.is_crlf_only(),
        "Original file should have CRLF only"
    );

    let analyses = vec![original_analysis];

    // First convert line endings
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "Line ending conversion should succeed");

    // Then remove BOM
    let intermediate_analysis = analyze_file(&has_bom_path, &config);
    let analyses_after_conversion = vec![intermediate_analysis];
    let result = remove_bom_from_files(&config, &analyses_after_conversion);
    assert!(result.is_ok(), "BOM removal should succeed");

    // Verify both operations
    let final_analysis = analyze_file(&has_bom_path, &config);
    assert!(!final_analysis.has_bom(), "BOM should be removed");
    assert!(
        final_analysis.is_lf_only(),
        "File should have LF only after conversion"
    );
}

#[test]
fn test_original_test_folder_unchanged() {
    // This test ensures the original test_folder files are not modified
    let test_folder = Path::new("test_folder");
    let config = create_test_config();

    // Analyze original files
    let has_bom_path = test_folder.join("has_bom.txt");
    let analysis = analyze_file(&has_bom_path, &config);
    assert!(
        analysis.has_bom(),
        "Original test file should still have BOM"
    );

    let windows_path = test_folder.join("test_windows.txt");
    let analysis = analyze_file(&windows_path, &config);
    assert!(
        analysis.is_crlf_only(),
        "Original Windows test file should still have CRLF"
    );

    let linux_path = test_folder.join("test_linux.txt");
    let analysis = analyze_file(&linux_path, &config);
    assert!(
        analysis.is_lf_only(),
        "Original Linux test file should still have LF"
    );
}

#[test]
fn test_count_line_endings_directly() {
    let temp_dir = setup_test_environment();

    // Test direct line ending counting without config
    let windows_file = temp_dir.path().join("test_windows.txt");
    let (lf_count, crlf_count) =
        count_line_endings_in_file(&windows_file).expect("Should count line endings");
    assert_eq!(lf_count, 0, "Windows file should have no LF");
    assert!(crlf_count > 0, "Windows file should have CRLF");

    let linux_file = temp_dir.path().join("test_linux.txt");
    let (lf_count, crlf_count) =
        count_line_endings_in_file(&linux_file).expect("Should count line endings");
    assert!(lf_count > 0, "Linux file should have LF");
    assert_eq!(crlf_count, 0, "Linux file should have no CRLF");
}

#[test]
fn test_create_custom_test_files() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");

    // Create a file with only CRLF
    let crlf_file = temp_dir.path().join("custom_crlf.txt");
    let mut file = fs::File::create(&crlf_file).expect("Failed to create file");
    write!(file, "Line 1\r\nLine 2\r\nLine 3\r\n").expect("Failed to write to file");

    let config = create_test_config();
    let analysis = analyze_file(&crlf_file, &config);
    assert!(
        analysis.is_crlf_only(),
        "Custom CRLF file should have only CRLF"
    );
    assert_eq!(
        analysis.crlf_count, 3,
        "Should have exactly 3 CRLF line endings"
    );

    // Create a file with only LF
    let lf_file = temp_dir.path().join("custom_lf.txt");
    let mut file = fs::File::create(&lf_file).expect("Failed to create file");
    write!(file, "Line 1\nLine 2\nLine 3\n").expect("Failed to write to file");

    let analysis = analyze_file(&lf_file, &config);
    assert!(analysis.is_lf_only(), "Custom LF file should have only LF");
    assert_eq!(
        analysis.lf_count, 3,
        "Should have exactly 3 LF line endings"
    );

    // Create a file with mixed line endings
    let mixed_file = temp_dir.path().join("custom_mixed.txt");
    let mut file = fs::File::create(&mixed_file).expect("Failed to create file");
    write!(file, "Line 1\r\nLine 2\nLine 3\r\nLine 4\n").expect("Failed to write to file");

    let analysis = analyze_file(&mixed_file, &config);
    assert!(
        analysis.has_mixed_line_endings(),
        "Custom mixed file should have mixed line endings"
    );
    assert_eq!(
        analysis.crlf_count, 2,
        "Should have exactly 2 CRLF line endings"
    );
    assert_eq!(
        analysis.lf_count, 2,
        "Should have exactly 2 LF line endings"
    );
}

/// Test BOM output when --bom flag is requested
#[test]
fn test_bom_output_when_requested() {
    let temp_dir = setup_test_environment();
    let config = create_test_config(); // This has check_bom = true

    // Test file with BOM - should show BOM type
    let has_bom_path = temp_dir.path().join("has_bom.txt");

    // We can't easily capture println! output in tests, but we can test the logic
    // by verifying the BOM type is correctly set when check_bom is true
    let analysis = analyze_file(&has_bom_path, &config);
    assert!(
        analysis.bom_type.is_some(),
        "BOM type should be set when check_bom is true"
    );
    assert_eq!(
        analysis.bom_type,
        Some(BomType::Utf8),
        "Should detect UTF-8 BOM"
    );

    // Test file without BOM - should show "BOM: none" when requested
    let no_bom_path = temp_dir.path().join("test_linux.txt");
    let analysis = analyze_file(&no_bom_path, &config);
    assert!(
        analysis.bom_type.is_some(),
        "BOM type should be set when check_bom is true"
    );
    assert_eq!(
        analysis.bom_type,
        Some(BomType::None),
        "Should detect no BOM"
    );
}

/// Test BOM output when --bom flag is NOT requested  
#[test]
fn test_bom_output_when_not_requested() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.check_bom = false; // Disable BOM checking

    // Test file with BOM - should NOT show BOM info
    let has_bom_path = temp_dir.path().join("has_bom.txt");
    let analysis = analyze_file(&has_bom_path, &config);
    assert!(
        analysis.bom_type.is_none(),
        "BOM type should be None when check_bom is false"
    );

    // Test file without BOM - should NOT show BOM info
    let no_bom_path = temp_dir.path().join("test_linux.txt");
    let analysis = analyze_file(&no_bom_path, &config);
    assert!(
        analysis.bom_type.is_none(),
        "BOM type should be None when check_bom is false"
    );
}

/// Test that BOM status shows correct format strings
#[test]
fn test_bom_status_format_strings() {
    use line_endings::analysis::detect_bom;
    use line_endings::types::BomType;

    let temp_dir = setup_test_environment();

    // Test BOM type string formats match expected output
    assert_eq!(
        BomType::None.to_string(),
        "none",
        "BomType::None should display as 'none'"
    );
    assert_eq!(
        BomType::Utf8.to_string(),
        "UTF-8",
        "BomType::Utf8 should display as 'UTF-8'"
    );
    assert_eq!(
        BomType::Utf16Le.to_string(),
        "UTF-16 LE",
        "BomType::Utf16Le should display as 'UTF-16 LE'"
    );
    assert_eq!(
        BomType::Utf16Be.to_string(),
        "UTF-16 BE",
        "BomType::Utf16Be should display as 'UTF-16 BE'"
    );
    assert_eq!(
        BomType::Utf32Le.to_string(),
        "UTF-32 LE",
        "BomType::Utf32Le should display as 'UTF-32 LE'"
    );
    assert_eq!(
        BomType::Utf32Be.to_string(),
        "UTF-32 BE",
        "BomType::Utf32Be should display as 'UTF-32 BE'"
    );

    // Test actual BOM detection
    let has_bom_path = temp_dir.path().join("has_bom.txt");
    let bom_type = detect_bom(&has_bom_path).expect("Should detect BOM");
    assert_eq!(bom_type, BomType::Utf8, "Should detect UTF-8 BOM");

    let no_bom_path = temp_dir.path().join("test_linux.txt");
    let bom_type = detect_bom(&no_bom_path).expect("Should detect no BOM");
    assert_eq!(bom_type, BomType::None, "Should detect no BOM");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_non_existent_file() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let non_existent = temp_dir.path().join("does_not_exist.txt");
    let config = create_test_config();

    let analysis = analyze_file(&non_existent, &config);
    assert!(
        analysis.error.is_some(),
        "Should have error for non-existent file"
    );
}

#[test]
fn test_empty_file() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let empty_file = temp_dir.path().join("empty.txt");
    fs::File::create(&empty_file).expect("Failed to create empty file");

    let config = create_test_config();
    let analysis = analyze_file(&empty_file, &config);

    assert!(analysis.error.is_none(), "Empty file should not error");
    assert_eq!(analysis.lf_count, 0, "Empty file should have no LF");
    assert_eq!(analysis.crlf_count, 0, "Empty file should have no CRLF");
}

#[test]
fn test_file_with_only_cr() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let cr_file = temp_dir.path().join("cr_only.txt");

    // Write file with only CR (old Mac style)
    let content = b"Line 1\rLine 2\rLine 3\r";
    fs::write(&cr_file, content).expect("Failed to write CR file");

    let config = create_test_config();
    let analysis = analyze_file(&cr_file, &config);

    assert!(analysis.error.is_none(), "CR file should not error");
    assert_eq!(analysis.lf_count, 0, "Should have no LF");
    assert_eq!(analysis.crlf_count, 0, "Should have no CRLF");
}

#[test]
fn test_file_ending_with_cr_no_lf() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let cr_ending = temp_dir.path().join("cr_ending.txt");

    // Write file ending with CR but no LF
    let content = b"Line 1\nLine 2\r";
    fs::write(&cr_ending, content).expect("Failed to write file");

    let config = create_test_config();
    let analysis = analyze_file(&cr_ending, &config);

    assert!(analysis.error.is_none(), "Should not error");
    assert_eq!(analysis.lf_count, 1, "Should have 1 LF");
    assert_eq!(analysis.crlf_count, 0, "Should have no CRLF");
}

#[test]
fn test_large_file() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let large_file = temp_dir.path().join("large.txt");

    // Create a file larger than buffer size (4KB) - create ~10KB file
    let mut file = fs::File::create(&large_file).expect("Failed to create file");
    for i in 0..1000 {
        writeln!(
            file,
            "Line number {i} with some extra text to increase size"
        )
        .expect("Failed to write to file");
    }

    let config = create_test_config();
    let analysis = analyze_file(&large_file, &config);

    assert!(analysis.error.is_none(), "Large file should not error");
    assert_eq!(analysis.lf_count, 1000, "Should have 1000 LF endings");
}

// ============================================================================
// Backup File Tests
// ============================================================================

#[test]
fn test_backup_file_created_on_line_ending_conversion() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.set_windows = true;

    let linux_file = temp_dir.path().join("test_linux.txt");
    let backup_file = linux_file.with_extension("txt.bak");

    // Ensure backup doesn't exist initially
    assert!(!backup_file.exists(), "Backup should not exist initially");

    let analysis = analyze_file(&linux_file, &config);
    let analyses = vec![analysis];
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "Rewrite should succeed");

    // Verify backup was created
    assert!(
        backup_file.exists(),
        "Backup file should be created after rewrite"
    );
}

#[test]
fn test_backup_file_created_on_bom_removal() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.remove_bom = true;

    let has_bom_path = temp_dir.path().join("has_bom.txt");
    let backup_file = has_bom_path.with_extension("txt.bak");

    // Ensure backup doesn't exist initially
    assert!(!backup_file.exists(), "Backup should not exist initially");

    let analysis = analyze_file(&has_bom_path, &config);
    let analyses = vec![analysis];
    let result = remove_bom_from_files(&config, &analyses);
    assert!(result.is_ok(), "BOM removal should succeed");

    // Verify backup was created
    assert!(
        backup_file.exists(),
        "Backup file should be created after BOM removal"
    );
}

#[test]
fn test_backup_not_overwritten_on_multiple_operations() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.set_windows = true;

    let linux_file = temp_dir.path().join("test_linux.txt");
    let backup_file = linux_file.with_extension("txt.bak");

    // First conversion
    let analysis = analyze_file(&linux_file, &config);
    let analyses = vec![analysis];
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "First rewrite should succeed");

    // Get backup creation time
    let backup_metadata = fs::metadata(&backup_file).expect("Backup should exist");
    let first_modified = backup_metadata
        .modified()
        .expect("Should get modified time");

    // Wait a moment to ensure timestamps would differ
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Second conversion (convert back to Linux)
    config.set_windows = false;
    config.set_linux = true;
    let analysis = analyze_file(&linux_file, &config);
    let analyses = vec![analysis];
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "Second rewrite should succeed");

    // Verify backup was NOT overwritten
    let backup_metadata = fs::metadata(&backup_file).expect("Backup should still exist");
    let second_modified = backup_metadata
        .modified()
        .expect("Should get modified time");

    assert_eq!(
        first_modified, second_modified,
        "Backup should not be overwritten on second operation"
    );
}

// ============================================================================
// Trailing Newline Preservation Tests
// ============================================================================

#[test]
fn test_trailing_newline_preserved_on_conversion() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let file_with_trailing = temp_dir.path().join("trailing.txt");

    // Create file with trailing newline
    fs::write(&file_with_trailing, b"Line 1\nLine 2\nLine 3\n").expect("Failed to write file");

    let mut config = create_test_config();
    config.set_windows = true;

    let analysis = analyze_file(&file_with_trailing, &config);
    let analyses = vec![analysis];
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "Conversion should succeed");

    // Verify trailing newline is preserved
    let content = fs::read(&file_with_trailing).expect("Should read file");
    assert!(
        content.ends_with(b"\r\n"),
        "Trailing newline should be preserved as CRLF"
    );
}

#[test]
fn test_no_trailing_newline_preserved_on_conversion() {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let file_no_trailing = temp_dir.path().join("no_trailing.txt");

    // Create file WITHOUT trailing newline
    fs::write(&file_no_trailing, b"Line 1\nLine 2\nLine 3").expect("Failed to write file");

    let mut config = create_test_config();
    config.set_windows = true;

    let analysis = analyze_file(&file_no_trailing, &config);
    let analyses = vec![analysis];
    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "Conversion should succeed");

    // Verify no trailing newline is added
    let content = fs::read(&file_no_trailing).expect("Should read file");
    assert!(
        !content.ends_with(b"\r\n") && !content.ends_with(b"\n"),
        "Should not add trailing newline"
    );
    assert!(
        content.ends_with(b"3"),
        "Should end with last character of content"
    );
}

// ============================================================================
// CLI/Config Tests
// ============================================================================

#[test]
fn test_conflicting_line_ending_flags() {
    use line_endings::config::parse_args;
    use pico_args::Arguments;
    use std::ffi::OsString;

    let args: Vec<OsString> = vec![
        "program".into(),
        "-l".into(),
        "-w".into(),
        "test.txt".into(),
    ];
    let pico_args = Arguments::from_vec(args);
    let result = parse_args(pico_args);

    assert!(
        result.is_err(),
        "Should error when both -l and -w are specified"
    );
    let error_msg = result.err().unwrap().to_string();
    assert!(
        error_msg.contains("Cannot set both Linux and Windows"),
        "Error message should mention conflicting flags"
    );
}

#[test]
fn test_unrecognized_switch() {
    use line_endings::config::parse_args;
    use pico_args::Arguments;
    use std::ffi::OsString;

    let args: Vec<OsString> = vec!["program".into(), "--invalid-flag".into(), "test.txt".into()];
    let pico_args = Arguments::from_vec(args);
    let result = parse_args(pico_args);

    assert!(result.is_err(), "Should error on unrecognized switch");
}

#[test]
fn test_remove_bom_enables_check_bom() {
    use line_endings::config::parse_args;
    use pico_args::Arguments;
    use std::ffi::OsString;

    let args: Vec<OsString> = vec!["program".into(), "-m".into(), "test.txt".into()];
    let pico_args = Arguments::from_vec(args);
    let result = parse_args(pico_args);

    assert!(result.is_ok(), "Should parse successfully");
    let config = result.unwrap();
    assert!(
        config.check_bom,
        "check_bom should be enabled when remove_bom is set"
    );
    assert!(config.remove_bom, "remove_bom should be set");
}

// ============================================================================
// Glob/Utils Tests
// ============================================================================

#[test]
fn test_glob_pattern_matching() {
    use line_endings::utils::get_paths_matching_glob;

    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.folder = Some(temp_dir.path().to_string_lossy().to_string());
    config.supplied_paths = vec!["*.txt".to_string()];
    config.recursive = false;

    let paths = get_paths_matching_glob(&config).expect("Should match glob pattern");

    assert!(!paths.is_empty(), "Should match at least one file");
    assert!(
        paths.iter().all(|p| p.ends_with(".txt")),
        "All matched files should end with .txt"
    );
}

#[test]
fn test_recursive_glob_pattern() {
    use line_endings::utils::get_paths_matching_glob;

    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.folder = Some(temp_dir.path().to_string_lossy().to_string());
    config.supplied_paths = vec!["has_bom.txt".to_string()];
    config.recursive = true;

    let paths = get_paths_matching_glob(&config).expect("Should match glob pattern");

    // Should find has_bom.txt in both root and sub_folder
    assert!(
        paths.len() >= 2,
        "Should find files in subdirectories with recursive flag"
    );
}

#[test]
fn test_case_sensitive_glob() {
    use line_endings::utils::get_paths_matching_glob;

    let temp_dir = TempDir::new().expect("Failed to create temporary directory");
    let lowercase_file = temp_dir.path().join("test.txt");
    let uppercase_file = temp_dir.path().join("TEST.txt");

    fs::write(&lowercase_file, b"content").expect("Failed to create file");
    fs::write(&uppercase_file, b"content").expect("Failed to create file");

    // Case-sensitive search
    let mut config = create_test_config();
    config.folder = Some(temp_dir.path().to_string_lossy().to_string());
    config.supplied_paths = vec!["test.txt".to_string()];
    config.case_sensitive = true;
    config.recursive = false;

    let paths = get_paths_matching_glob(&config).expect("Should match glob pattern");

    assert_eq!(paths.len(), 1, "Should match only exact case");
    assert!(
        paths[0].ends_with("test.txt"),
        "Should match lowercase file"
    );
}

#[test]
fn test_non_matching_glob_pattern() {
    use line_endings::utils::get_paths_matching_glob;

    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.folder = Some(temp_dir.path().to_string_lossy().to_string());
    config.supplied_paths = vec!["*.doesnotexist".to_string()];
    config.recursive = false;

    let paths = get_paths_matching_glob(&config).expect("Should not error on no matches");

    assert!(
        paths.is_empty(),
        "Should return empty vector for no matches"
    );
}

// ============================================================================
// Multiple File Processing Tests
// ============================================================================

#[test]
fn test_multiple_files_processed_correctly() {
    let temp_dir = setup_test_environment();
    let mut config = create_test_config();
    config.set_linux = true;

    // Analyze multiple files
    let windows_file = temp_dir.path().join("test_windows.txt");
    let linux_file = temp_dir.path().join("test_linux.txt");
    let mixed_file = temp_dir.path().join("test_lines.txt");

    let analyses = vec![
        analyze_file(&windows_file, &config),
        analyze_file(&linux_file, &config),
        analyze_file(&mixed_file, &config),
    ];

    let result = rewrite_files(&config, &analyses);
    assert!(result.is_ok(), "Should process multiple files successfully");

    // Verify all files now have LF only
    let windows_analysis = analyze_file(&windows_file, &config);
    let linux_analysis = analyze_file(&linux_file, &config);
    let mixed_analysis = analyze_file(&mixed_file, &config);

    assert!(
        windows_analysis.is_lf_only(),
        "Windows file should be converted to LF"
    );
    assert!(linux_analysis.is_lf_only(), "Linux file should remain LF");
    assert!(
        mixed_analysis.is_lf_only(),
        "Mixed file should be converted to LF"
    );
}
