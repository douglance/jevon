//! Entry point for the workspace gates.
//!
//! ```text
//! cargo xtask check    # structure, size, complexity, manifests
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("xtask: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool> {
    let task = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "check".to_owned());
    match task.as_str() {
        "check" => {
            let violations = xtask::check(&workspace_root()?)?;
            Ok(xtask::violation::report(&violations))
        }
        other => bail!("unknown task `{other}`; expected `check`"),
    }
}

/// The workspace root, derived from this crate's location rather than the cwd,
/// so the gate reports the same result from anywhere in the tree.
fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match manifest_dir.parent() {
        Some(root) => Ok(root.to_path_buf()),
        None => bail!("xtask has no parent directory"),
    }
}
