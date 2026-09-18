//! Finding the things to check.
//!
//! Rust files come from `git ls-files`, not a directory walk, so files that are
//! untracked but not ignored are still covered — the gap a `walkdir` over
//! `crates/` silently leaves open.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// A workspace member on disk.
#[derive(Debug, Clone)]
pub struct CrateDir {
    /// Package name from its manifest.
    pub name: String,
    /// Directory holding its `Cargo.toml`.
    pub dir: PathBuf,
    /// Path to its `Cargo.toml`.
    pub manifest: PathBuf,
}

/// Lists every tracked-or-untracked, non-ignored `.rs` file under `root`.
///
/// # Errors
/// Fails if `git ls-files` cannot run, exits non-zero, or emits non-UTF-8.
pub fn rust_files(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ])
        .output()
        .context("running git ls-files")?;

    if !output.status.success() {
        bail!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let listing = String::from_utf8(output.stdout).context("git ls-files emitted non-UTF-8")?;
    Ok(listing
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| root.join(entry))
        .filter(|path| path.is_file())
        .collect())
}

/// Lists every crate directly under `<root>/crates`, plus `xtask`.
///
/// Reads the directory rather than parsing the root manifest's glob, because the
/// glob and the filesystem disagreeing is itself something worth catching.
///
/// # Errors
/// Fails if a directory cannot be read or a manifest cannot be parsed.
pub fn crates(root: &Path) -> Result<Vec<CrateDir>> {
    let mut found = Vec::new();
    push_children(&mut found, &root.join("crates"))?;
    push_if_crate(&mut found, root.join("xtask"))?;
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

/// Adds every crate directly inside `parent`, if `parent` exists at all.
fn push_children(found: &mut Vec<CrateDir>, parent: &Path) -> Result<()> {
    if !parent.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(parent).context("reading crates/")? {
        let dir = entry.context("reading a crates/ entry")?.path();
        push_if_crate(found, dir)?;
    }
    Ok(())
}

fn push_if_crate(found: &mut Vec<CrateDir>, dir: PathBuf) -> Result<()> {
    let manifest = dir.join("Cargo.toml");
    // Not every entry under crates/ is a crate; stray directories are ignored.
    if !manifest.is_file() {
        return Ok(());
    }
    let name = package_name(&manifest)?;
    found.push(CrateDir {
        name,
        dir,
        manifest,
    });
    Ok(())
}

fn package_name(manifest: &Path) -> Result<String> {
    let text = std::fs::read_to_string(manifest)
        .with_context(|| format!("reading {}", manifest.display()))?;
    let parsed: toml::Value =
        toml::from_str(&text).with_context(|| format!("parsing {}", manifest.display()))?;
    parsed
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .with_context(|| format!("{} has no package.name", manifest.display()))
}

/// Reads and parses a manifest.
///
/// # Errors
/// Fails if the file cannot be read or is not valid TOML.
pub fn manifest(path: &Path) -> Result<toml::Value> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}
