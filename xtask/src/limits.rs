//! Every threshold the workspace is held to, in one visible place.
//!
//! These are deliberately stricter than habit. The point is not that 41 lines
//! is worse than 40 — it is that a function which keeps reaching for one more
//! line is usually two functions, and a limit is the only thing that reliably
//! asks the question.

/// The thresholds a check run enforces.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Code lines in a file above which a warning is reported.
    pub file_warn: usize,
    /// Code lines in a file above which the run fails.
    pub file_fail: usize,
    /// Code lines in one function, closure or method body.
    pub function_body: usize,
    /// Independent paths through a function.
    pub cyclomatic: usize,
    /// Weighted nesting-aware complexity of a function.
    pub cognitive: usize,
    /// Depth of nested blocks inside a function.
    pub nesting: usize,
    /// Parameters a function may take.
    pub arguments: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            file_warn: 120,
            file_fail: 150,
            function_body: 30,
            cyclomatic: 6,
            cognitive: 7,
            nesting: 3,
            arguments: 4,
        }
    }
}
