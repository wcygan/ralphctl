//! Template fetching from GitHub for ralphctl init command.
//!
//! Fetches SPEC.md, IMPLEMENTATION_PLAN.md, and PROMPT.md templates
//! from the ralphctl GitHub repository.

#![allow(dead_code)] // Used by init command (future task)

use anyhow::{Context, Result};

/// Base URL for raw template content on GitHub.
const TEMPLATE_BASE_URL: &str = "https://raw.githubusercontent.com/wcygan/ralphctl/main/templates";

/// Template file names that can be fetched.
pub const TEMPLATE_FILES: &[&str] = &["SPEC.md", "IMPLEMENTATION_PLAN.md", "PROMPT.md"];

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
        // Verify expected templates are listed
        assert!(TEMPLATE_FILES.contains(&"SPEC.md"));
        assert!(TEMPLATE_FILES.contains(&"IMPLEMENTATION_PLAN.md"));
        assert!(TEMPLATE_FILES.contains(&"PROMPT.md"));
        assert_eq!(TEMPLATE_FILES.len(), 3);
    }

    #[test]
    fn test_url_construction() {
        let url = format!("{}/{}", TEMPLATE_BASE_URL, "SPEC.md");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/wcygan/ralphctl/main/templates/SPEC.md"
        );
    }

    // Note: Integration tests for actual HTTP fetching should use mock servers
    // or be run as part of E2E testing to avoid flaky tests due to network issues.
}
