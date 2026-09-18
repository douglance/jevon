//! Finding every unit of code that can get too big, and measuring it.
//!
//! "Unit" means free function, method, trait method with a default body, and
//! multi-line closure. The closure case matters: it is where long, tangled code
//! most often hides from a function-length rule, because it is syntactically an
//! expression rather than a function.

use std::path::Path;

use syn::visit::Visit as _;

use crate::allowlist;
use crate::limits::Limits;
use crate::metrics::Score;
use crate::size::code_line_count;
use crate::units::Collector;
use crate::violation::Violation;

const GATE: &str = "complexity";

/// One measured unit of code.
#[derive(Debug, Clone)]
pub struct Unit {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) end_line: usize,
    pub(crate) arguments: usize,
    pub(crate) score: Score,
}

/// Collects and checks every unit in one parsed file.
#[must_use]
pub fn check(path: &Path, source: &str, limits: &Limits) -> Vec<Violation> {
    let Ok(file) = syn::parse_file(source) else {
        // Parse failures are the compiler's job to report, not this gate's.
        return Vec::new();
    };
    let mut collector = Collector::default();
    collector.visit_file(&file);

    let lines: Vec<&str> = source.lines().collect();
    collector
        .units
        .iter()
        .flat_map(|unit| unit.violations(path, &lines, limits))
        .collect()
}

impl Unit {
    fn body_lines(&self, lines: &[&str]) -> usize {
        let start = self.line.saturating_sub(1);
        let end = self.end_line.min(lines.len());
        lines.get(start..end).map_or(0, code_line_count)
    }

    fn violations(&self, path: &Path, lines: &[&str], limits: &Limits) -> Vec<Violation> {
        let checks = [
            ("body-length", self.body_lines(lines), limits.function_body),
            ("cyclomatic", self.score.cyclomatic, limits.cyclomatic),
            ("cognitive", self.score.cognitive, limits.cognitive),
            ("nesting", self.score.nesting, limits.nesting),
            ("arguments", self.arguments, limits.arguments),
        ];

        checks
            .iter()
            .filter(|&&(_, actual, limit)| actual > limit)
            .filter(|&&(metric, _, _)| !allowlist::excused(lines, self.line, metric))
            .map(|&(metric, actual, limit)| {
                Violation::deny(
                    GATE,
                    path,
                    Some(self.line),
                    format!("{} {metric} is {actual}, limit {limit}", self.name),
                )
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use super::check;
    use crate::limits::Limits;

    fn tight() -> Limits {
        Limits {
            function_body: 3,
            cyclomatic: 2,
            cognitive: 2,
            nesting: 1,
            arguments: 1,
            ..Limits::default()
        }
    }

    #[test]
    fn a_small_function_passes() {
        let found = check(Path::new("x.rs"), "fn f(a: u8) { g(a); }", &tight());
        assert!(found.is_empty(), "unexpected: {found:?}");
    }

    #[test]
    fn too_many_arguments_is_caught() {
        let found = check(Path::new("x.rs"), "fn f(a: u8, b: u8, c: u8) {}", &tight());
        assert!(
            found.iter().any(|v| v.message.contains("arguments")),
            "got {found:?}"
        );
    }

    #[test]
    fn deep_nesting_is_caught() {
        let src = "fn f(a: bool, b: bool) { if a { if b { g(); } } }";
        let found = check(Path::new("x.rs"), src, &tight());
        assert!(
            found.iter().any(|v| v.message.contains("nesting")),
            "got {found:?}"
        );
    }

    #[test]
    fn a_long_multi_line_closure_is_caught() {
        let src = "fn f() {\n    let c = |a: u8| {\n        g(a);\n        g(a);\n        g(a);\n        g(a);\n    };\n}";
        let found = check(Path::new("x.rs"), src, &tight());
        assert!(
            found.iter().any(|v| v.message.starts_with("closure")),
            "closures must not escape the length rule: {found:?}"
        );
    }

    #[test]
    fn a_reasoned_directive_excuses_a_unit() {
        let src = "// typesafe-allow arguments reason: mirrors an external signature.\nfn f(a: u8, b: u8, c: u8) {}";
        let found = check(Path::new("x.rs"), src, &tight());
        assert!(found.is_empty(), "unexpected: {found:?}");
    }

    #[test]
    fn methods_are_measured_too() {
        let src = "struct S;\nimpl S {\n    fn m(&self, a: u8, b: u8) {}\n}";
        let found = check(Path::new("x.rs"), src, &tight());
        assert!(
            found.iter().any(|v| v.message.contains("method m")),
            "got {found:?}"
        );
    }
}
