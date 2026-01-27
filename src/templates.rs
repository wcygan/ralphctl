//! Template fetching from GitHub for ralphctl init command.
//!
//! Fetches SPEC.md, IMPLEMENTATION_PLAN.md, and PROMPT.md templates
//! from the ralphctl GitHub repository. Templates are cached locally
//! in the XDG cache directory for offline use.

#![allow(dead_code)] // Used by init command (future task)

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Base URL for raw template content on GitHub.
const TEMPLATE_BASE_URL: &str = "https://raw.githubusercontent.com/wcygan/ralphctl/main/templates";

/// Template file names for forward mode (init command).
pub const TEMPLATE_FILES: &[&str] = &["SPEC.md", "IMPLEMENTATION_PLAN.md", "PROMPT.md"];

/// Template file name for reverse mode.
pub const REVERSE_PROMPT_TEMPLATE: &str = "REVERSE_PROMPT.md";

/// Application name for cache directory.
const APP_NAME: &str = "ralphctl";

/// Subdirectory within app cache for templates.
const TEMPLATES_SUBDIR: &str = "templates";

/// Get the XDG-compliant cache directory for ralphctl templates.
///
/// Returns the path to the templates cache directory:
/// - Linux: `~/.cache/ralphctl/templates/`
/// - macOS: `~/Library/Caches/ralphctl/templates/`
///
/// # Errors
///
/// Returns an error if the cache directory cannot be determined (rare on Unix systems).
pub fn get_cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().context("failed to determine cache directory")?;
    Ok(base.join(APP_NAME).join(TEMPLATES_SUBDIR))
}

/// Get the cache file path for a specific template.
///
/// Returns the full path where a template should be cached.
pub fn get_cache_path(filename: &str) -> Result<PathBuf> {
    Ok(get_cache_dir()?.join(filename))
}

/// Ensure the cache directory exists, creating it if necessary.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn ensure_cache_dir() -> Result<PathBuf> {
    let cache_dir = get_cache_dir()?;
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).with_context(|| {
            format!("failed to create cache directory: {}", cache_dir.display())
        })?;
    }
    Ok(cache_dir)
}

/// Save a template to the cache.
///
/// Creates the cache directory if it doesn't exist.
///
/// # Errors
///
/// Returns an error if the cache directory cannot be created or the file cannot be written.
pub fn save_to_cache(filename: &str, content: &str) -> Result<()> {
    ensure_cache_dir()?;
    let path = get_cache_path(filename)?;
    fs::write(&path, content)
        .with_context(|| format!("failed to write cache file: {}", path.display()))?;
    Ok(())
}

/// Load a template from the cache.
///
/// # Errors
///
/// Returns an error if the cached file doesn't exist or cannot be read.
pub fn load_from_cache(filename: &str) -> Result<String> {
    let path = get_cache_path(filename)?;
    fs::read_to_string(&path)
        .with_context(|| format!("failed to read cache file: {}", path.display()))
}

/// Fetch a single template file from GitHub.
///
/// Returns the template content as a string.
///
/// # Errors
///
/// Returns an error if the network request fails or the response is not successful.
pub async fn fetch_template(filename: &str) -> Result<String> {
    let url = format!("{}/{}", TEMPLATE_BASE_URL, filename);

    let response = reqwest::get(&url)
        .await
        .with_context(|| format!("failed to fetch {}", filename))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "failed to fetch {}: HTTP {}",
            filename,
            response.status().as_u16()
        );
    }

    response
        .text()
        .await
        .with_context(|| format!("failed to read response for {}", filename))
}

/// Fetch all template files from GitHub.
///
/// Returns a vector of (filename, content) tuples.
///
/// # Errors
///
/// Returns an error if any template fetch fails.
pub async fn fetch_all_templates() -> Result<Vec<(&'static str, String)>> {
    let mut templates = Vec::with_capacity(TEMPLATE_FILES.len());

    for &filename in TEMPLATE_FILES {
        let content = fetch_template(filename).await?;
        templates.push((filename, content));
    }

    Ok(templates)
}

/// Fetch a template with network-first strategy and cache fallback.
///
/// Tries to fetch the template from GitHub first. On success, the template is
/// saved to the local cache for offline use. On network failure, falls back to
/// the cached version if available.
///
/// # Arguments
///
/// * `filename` - The template filename (e.g., "SPEC.md")
///
/// # Returns
///
/// The template content as a string.
///
/// # Errors
///
/// Returns an error only if both the network fetch fails AND no cached version exists.
pub async fn get_template(filename: &str) -> Result<String> {
    // Try network first
    match fetch_template(filename).await {
        Ok(content) => {
            // Cache the fetched content for offline use
            // Ignore cache write errors - it's just an optimization
            let _ = save_to_cache(filename, &content);
            Ok(content)
        }
        Err(network_err) => {
            // Fall back to cache
            load_from_cache(filename).with_context(|| {
                format!(
                    "failed to fetch {} (network: {}, no cache available)",
                    filename, network_err
                )
            })
        }
    }
}

/// Fetch all forward mode templates with network-first strategy and cache fallback.
///
/// For each template, tries to fetch from GitHub first, falling back to cache
/// on network failure. Successfully fetched templates are saved to cache.
///
/// Note: This does NOT include REVERSE_PROMPT.md, which is fetched separately
/// via `get_reverse_template()` for the reverse command.
///
/// # Returns
///
/// A vector of (filename, content) tuples for all forward mode templates.
///
/// # Errors
///
/// Returns an error if any template cannot be obtained from either network or cache.
pub async fn get_all_templates() -> Result<Vec<(&'static str, String)>> {
    let mut templates = Vec::with_capacity(TEMPLATE_FILES.len());

    for &filename in TEMPLATE_FILES {
        let content = get_template(filename).await?;
        templates.push((filename, content));
    }

    Ok(templates)
}

/// Fetch the reverse mode prompt template with network-first strategy and cache fallback.
///
/// Tries to fetch REVERSE_PROMPT.md from GitHub first. On success, the template
/// is saved to the local cache for offline use. On network failure, falls back
/// to the cached version if available.
///
/// # Returns
///
/// The REVERSE_PROMPT.md template content as a string.
///
/// # Errors
///
/// Returns an error only if both the network fetch fails AND no cached version exists.
pub async fn get_reverse_template() -> Result<String> {
    get_template(REVERSE_PROMPT_TEMPLATE).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_base_url_format() {
        // Verify the URL is well-formed
        assert!(TEMPLATE_BASE_URL.starts_with("https://"));
        assert!(TEMPLATE_BASE_URL.contains("github"));
        assert!(TEMPLATE_BASE_URL.ends_with("/templates"));
    }

    #[test]
    fn test_template_files_list() {
        // Verify expected forward mode templates are listed
        assert!(TEMPLATE_FILES.contains(&"SPEC.md"));
        assert!(TEMPLATE_FILES.contains(&"IMPLEMENTATION_PLAN.md"));
        assert!(TEMPLATE_FILES.contains(&"PROMPT.md"));
        assert_eq!(TEMPLATE_FILES.len(), 3);

        // REVERSE_PROMPT.md should NOT be in TEMPLATE_FILES (fetched separately)
        assert!(!TEMPLATE_FILES.contains(&REVERSE_PROMPT_TEMPLATE));
    }

    #[test]
    fn test_reverse_prompt_template_constant() {
        assert_eq!(REVERSE_PROMPT_TEMPLATE, "REVERSE_PROMPT.md");
    }

    #[test]
    fn test_url_construction() {
        let url = format!("{}/{}", TEMPLATE_BASE_URL, "SPEC.md");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/wcygan/ralphctl/main/templates/SPEC.md"
        );
    }

    #[test]
    fn test_reverse_template_url_construction() {
        let url = format!("{}/{}", TEMPLATE_BASE_URL, REVERSE_PROMPT_TEMPLATE);
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/wcygan/ralphctl/main/templates/REVERSE_PROMPT.md"
        );
    }

    #[test]
    fn test_get_cache_dir_structure() {
        let cache_dir = get_cache_dir().unwrap();
        let path_str = cache_dir.to_string_lossy();

        // Should contain app name and templates subdir
        assert!(path_str.contains("ralphctl"));
        assert!(path_str.ends_with("templates"));
    }

    #[test]
    fn test_get_cache_path_includes_filename() {
        let path = get_cache_path("SPEC.md").unwrap();
        assert!(path.ends_with("SPEC.md"));
        assert!(path.to_string_lossy().contains("ralphctl"));
    }

    #[test]
    fn test_cache_dir_is_xdg_compliant() {
        let cache_dir = get_cache_dir().unwrap();

        // On macOS, should be in Library/Caches
        // On Linux, should be in .cache
        let path_str = cache_dir.to_string_lossy();
        let is_macos_path = path_str.contains("Library/Caches");
        let is_linux_path = path_str.contains(".cache");

        assert!(
            is_macos_path || is_linux_path,
            "Cache dir should follow XDG or macOS conventions: {}",
            path_str
        );
    }

    // Note: Integration tests for actual HTTP fetching should use mock servers
    // or be run as part of E2E testing to avoid flaky tests due to network issues.

    #[test]
    fn test_cache_roundtrip() {
        let filename = "test_roundtrip.md";
        let content = "# Test Content\n\nThis is test content.";

        // Save to cache
        save_to_cache(filename, content).expect("save should succeed");

        // Load from cache
        let loaded = load_from_cache(filename).expect("load should succeed");
        assert_eq!(loaded, content);

        // Clean up
        let path = get_cache_path(filename).unwrap();
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_load_from_cache_missing_file() {
        let result = load_from_cache("nonexistent_file_12345.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_cache_dir_creates_directory() {
        let cache_dir = ensure_cache_dir().expect("ensure should succeed");
        assert!(cache_dir.exists());
        assert!(cache_dir.is_dir());
    }
}
