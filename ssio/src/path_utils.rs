use std::path::{Path, PathBuf};
use pathdiff::diff_paths;


/// Converts a raw relative path in source HTML into a virtual path prefixed with `@/`
///
/// ### Example
/// ```
/// let virtual_path = normalize_virtual_path("pages/page1.html", "sample/main.html", "sample/");
/// assert_eq!(virtual_path, "@/pages/page1.html");
/// ```
pub fn normalize_virtual_path(
    href: &str,
    origin_file_path: impl AsRef<Path>,
    project_root: impl AsRef<Path>,
) -> String {
    if is_external_url(href) {
        return href.to_string();
    }

    let href_path = Path::new(href);
    let origin_dir = origin_file_path.as_ref().parent().unwrap();
    let resolved = origin_dir.join(href_path);
    let relative_to_root = pathdiff::diff_paths(&resolved, project_root)
        .unwrap_or_else(|| resolved.clone());
    let cleaned = path_clean::clean(&relative_to_root);
    format!("@/{}", cleaned.to_string_lossy().replace('\\', "/"))
}


/// Resolves a virtual path (starting with `@/`) into a relative path
/// from the HTML file's output location to the target file.
///
/// ### Example
/// ```
/// let final_href = resolve_virtual_path("@/pages/page1.html", "output/pages/index.html");
/// assert_eq!(final_href, "page1.html");
/// ```
pub fn resolve_virtual_path(
    virtual_path: &str,
    current_output_file: impl AsRef<Path>,
) -> String {
    if is_external_url(virtual_path) || !virtual_path.starts_with("@/") {
        return virtual_path.to_string();
    }

    let logical_path = virtual_path.trim_start_matches("@/");
    let target_file = Path::new("output").join(logical_path);
    let current_dir = current_output_file.as_ref().parent().unwrap();

    let resolved = pathdiff::diff_paths(&target_file, current_dir)
        .unwrap_or_else(|| PathBuf::from(logical_path));
    path_clean::clean(&resolved).to_string_lossy().replace('\\', "/")
}



/// Returns true if a link is an external URL and should not be rewritten.
///
/// # Example
///
/// ```rust
/// use your_crate::path_utils::is_external_url;
///
/// assert!(is_external_url("https://example.com"));
/// assert!(is_external_url("//cdn.example.com/lib.css"));
/// assert!(is_external_url("mailto:hi@example.com"));
/// assert!(!is_external_url("pages/page1.html"));
/// ```
pub fn is_external_url(href: &str) -> bool {
    let lowered = href.trim().to_ascii_lowercase();
    lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("//")
        || lowered.starts_with("mailto:")
        || lowered.starts_with("#")
}

pub fn resolve_file_path_paterns(patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    fn resolve_entry_as_glob(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut results = Vec::<PathBuf>::new();
        for pattern in glob::glob(pattern)? {
            match pattern {
                Ok(path) => {
                    results.push(path);
                    continue;
                }
                Err(error) => return Err(Box::new(error)),
            }
        }
        Ok(results)
    }
    fn resolve_entry(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        if let Ok(results) = resolve_entry_as_glob(pattern) {
            return Ok(results)
        }
        let path = PathBuf::from(pattern);
        return Ok(vec![path])
    }
    let mut results = Vec::<PathBuf>::new();
    for pattern in patterns {
        match resolve_entry(&pattern) {
            Ok(paths) => {
                results.extend(paths);
            }
            Err(error) => {
                return Err(error)
            }
        }
    }
    Ok(results)
}

/// Returns the common ancestor (shared prefix) of two paths, if any.
pub fn common_ancestor(p1: impl AsRef<std::path::Path>, p2: impl AsRef<std::path::Path>) -> Option<PathBuf> {
    use std::path::Component;
    let p1 = p1.as_ref();
    let p2 = p2.as_ref();
    /// Converts a `Component` to a string slice
    fn component_as_str<'a>(component: &'a Component) -> &'a std::ffi::OsStr {
        component.as_os_str()
    }
    let mut result = PathBuf::new();
    let mut p1_components = p1.components();
    let mut p2_components = p2.components();

    loop {
        match (p1_components.next(), p2_components.next()) {
            (Some(c1), Some(c2)) if c1 == c2 => result.push(component_as_str(&c1)),
            _ => break,
        }
    }

    if result.as_os_str().is_empty() {
        None
    } else {
        Some(result)
    }
}

