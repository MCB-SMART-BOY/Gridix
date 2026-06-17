//! check-doc-links — validate local Markdown links in README.md and docs/*.md.
//!
//! Usage: cargo run --bin check-doc-links
//! Replaces: scripts/check_doc_links.py

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::LazyLock;

static LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[[^\]]+\]\(([^)]+)\)").expect("invalid link regex"));

fn main() -> ExitCode {
    let root = project_root();
    let mut broken: Vec<(PathBuf, String)> = Vec::new();

    for doc in iter_docs(&root) {
        let content = match std::fs::read_to_string(&doc) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("ERROR reading {}: {e}", doc.display());
                return ExitCode::from(2);
            }
        };

        for cap in LINK_RE.captures_iter(&content) {
            let link = cap[1].trim().to_string();
            if link.is_empty() || is_external(&link) {
                continue;
            }
            // Resolve relative link against the doc's parent directory
            let target = doc.parent().unwrap_or(&root).join(&link);
            if !target.exists() {
                broken.push((doc.strip_prefix(&root).unwrap().to_path_buf(), link));
            }
        }
    }

    if broken.is_empty() {
        println!("OK: no broken local links");
        ExitCode::SUCCESS
    } else {
        println!("Broken local links found:");
        for (source, link) in &broken {
            println!("  - {} -> {}", source.display(), link);
        }
        ExitCode::from(1)
    }
}

fn project_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is set by cargo; fall back to cwd
    std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("no cwd"))
}

fn iter_docs(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![root.join("README.md")];

    let docs_dir = root.join("docs");
    if let Ok(entries) = std::fs::read_dir(&docs_dir) {
        let mut md_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
            .collect();
        md_files.sort();
        files.extend(md_files);
    }

    files.into_iter().filter(|p| p.exists()).collect()
}

fn is_external(link: &str) -> bool {
    link.starts_with("http://") || link.starts_with("https://") || link.starts_with('#')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_external() {
        assert!(is_external("https://example.com"));
        assert!(is_external("http://example.com"));
        assert!(is_external("#section"));
        assert!(!is_external("ARCHITECTURE.md"));
        assert!(!is_external("recovery/02-query.md"));
    }

    #[test]
    fn test_project_root() {
        let root = project_root();
        assert!(
            root.join("Cargo.toml").exists(),
            "root must contain Cargo.toml"
        );
    }
}
