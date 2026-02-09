//! Find ISO/IMG files matching a query pattern.

use std::fs;
use std::path::{Path, PathBuf};

/// Find ISO/IMG files matching the query. Searches common locations and current directory.
pub fn find_iso_files(query: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    // Common locations to search
    let search_paths = vec![
        PathBuf::from("."),
        PathBuf::from("/home"),
        PathBuf::from("/mnt"),
        PathBuf::from("/media"),
        PathBuf::from("/tmp"),
    ];

    for base_path in search_paths {
        if let Ok(entries) = find_files_recursive(&base_path, &query_lower, 3) {
            results.extend(entries);
        }
    }

    // Sort by relevance: exact matches first, then by path length
    results.sort_by(|a, b| {
        let a_str = a.to_string_lossy().to_lowercase();
        let b_str = b.to_string_lossy().to_lowercase();
        let a_exact = a_str.contains(&query_lower);
        let b_exact = b_str.contains(&query_lower);
        match (a_exact, b_exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a_str.len().cmp(&b_str.len()),
        }
    });

    // Limit results
    results.truncate(50);
    results
}

fn find_files_recursive(dir: &Path, query: &str, max_depth: usize) -> Result<Vec<PathBuf>, std::io::Error> {
    if max_depth == 0 {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files/dirs and common system paths
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }
        }

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if ext_lower == "iso" || ext_lower == "img" {
                    let path_str = path.to_string_lossy().to_lowercase();
                    if query.is_empty() || path_str.contains(query) {
                        results.push(path.clone());
                    }
                }
            }
        } else if path.is_dir() {
            // Skip common non-user directories
            let skip_dirs = ["proc", "sys", "dev", "run", "boot", "lost+found"];
            if let Some(name) = path.file_name() {
                if skip_dirs.contains(&name.to_string_lossy().as_ref()) {
                    continue;
                }
            }

            if let Ok(mut sub_results) = find_files_recursive(&path, query, max_depth - 1) {
                results.append(&mut sub_results);
            }
        }
    }

    Ok(results)
}
