//! The single currency every gate reports in.
//!
//! Gates never print and never exit. They return violations, which keeps each
//! gate a pure function of the tree and makes them testable without a process.

use std::fmt;
use std::path::{Path, PathBuf};

/// How much a violation matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Worth fixing, does not fail the gate.
    Warn,
    /// Fails the gate.
    Deny,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Warn => f.write_str("warn"),
            Self::Deny => f.write_str("deny"),
        }
    }
}

/// One rule breach, addressed to a place a person can open.
#[derive(Debug, Clone)]
pub struct Violation {
    /// Which gate produced this.
    pub gate: &'static str,
    /// File the breach lives in.
    pub file: PathBuf,
    /// One-based line, when the gate can attribute one.
    pub line: Option<usize>,
    /// What is wrong, in one sentence.
    pub message: String,
    /// Whether this fails the run.
    pub severity: Severity,
}

impl Violation {
    /// Records a failing breach.
    #[must_use]
    pub fn deny(gate: &'static str, file: &Path, line: Option<usize>, message: String) -> Self {
        Self {
            gate,
            file: file.to_path_buf(),
            line,
            message,
            severity: Severity::Deny,
        }
    }

    /// Records a breach that is reported but tolerated.
    #[must_use]
    pub fn warn(gate: &'static str, file: &Path, line: Option<usize>, message: String) -> Self {
        Self {
            gate,
            file: file.to_path_buf(),
            line,
            message,
            severity: Severity::Warn,
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let where_ = self.line.map_or_else(
            || self.file.display().to_string(),
            |line| format!("{}:{line}", self.file.display()),
        );
        write!(
            f,
            "{:>4} [{}] {} — {}",
            self.severity, self.gate, where_, self.message
        )
    }
}

/// Reports every violation and answers whether the run passed.
///
/// Prints warnings too, because a limit nobody sees is not a limit.
#[must_use]
pub fn report(violations: &[Violation]) -> bool {
    let denied = violations
        .iter()
        .filter(|v| v.severity == Severity::Deny)
        .count();
    let warned = violations.len() - denied;

    for violation in violations {
        println!("{violation}");
    }

    if denied == 0 {
        println!("\nxtask check: passed ({warned} warning(s))");
        return true;
    }
    println!("\nxtask check: FAILED — {denied} violation(s), {warned} warning(s)");
    false
}
