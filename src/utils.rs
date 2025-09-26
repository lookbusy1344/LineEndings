use anyhow::Result;
use std::path::Path;

use crate::types::ConfigSettings;

/// function to take a glob and return a vector of path strings
///
/// # Errors
///
/// Returns an error if glob pattern matching fails.
pub fn get_paths_matching_glob(config: &ConfigSettings) -> Result<Vec<String>> {
    // This function expands given globs and sorted within each glob, but does not sort between globs.
    // eg given z*.txt a*.txt it will return:
    // ["z1.txt", "z2.txt", "a1.txt", "a2.txt"]

    let glob_settings = glob::MatchOptions {
        case_sensitive: config.case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    // create a vector to hold the results, initial capacity is set to the number of supplied paths
    let mut result = Vec::with_capacity(config.supplied_paths.len());

    for pattern in &config.supplied_paths {
        // Build the full search pattern with folder prefix if specified
        let full_pattern = if let Some(folder) = &config.folder {
            // Don't add folder prefix if it's just "." (current directory)
            if folder == "." {
                pattern.clone()
            } else {
                format!("{}/{}", folder.trim_end_matches('/'), pattern)
            }
        } else {
            pattern.clone()
        };

        // If recursive is enabled, modify the pattern to search subdirectories
        let search_pattern = if config.recursive && !full_pattern.contains("**/") {
            if let Some(folder) = &config.folder {
                // Don't add folder prefix if it's just "." (current directory)
                if folder == "." {
                    format!("**/{pattern}")
                } else {
                    format!("{}/**/{}", folder.trim_end_matches('/'), pattern)
                }
            } else {
                format!("**/{pattern}")
            }
        } else {
            full_pattern
        };

        // Try to match the pattern as a glob
        let mut glob_matches: Vec<_> = glob::glob_with(&search_pattern, glob_settings)?
            .filter_map(|entry| match entry {
                Ok(path) if path.is_file() => Some(path.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();

        // If the glob matched nothing, check if the pattern itself is a valid file
        if glob_matches.is_empty() && file_exists(&search_pattern) {
            result.push(search_pattern);
        } else {
            // If glob matches were found, sort them and extend the result vector
            // glob_matches.sort(); // Sorts in lexicographical order
            glob_matches.sort_by_key(|x| x.to_lowercase()); // Sorts in case-insensitive order
            result.extend(glob_matches);
        }
    }

    Ok(result)
}

/// check if file exists
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    let path_ref = path.as_ref();
    path_ref.exists() && path_ref.is_file()
}
