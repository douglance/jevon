//! Scoring one function body.
//!
//! Cyclomatic complexity counts independent paths; cognitive complexity weights
//! each branch by how deeply it is nested, which is what actually makes code
//! hard to hold in your head. Both are computed from the syntax tree rather than
//! from text, so a branch cannot hide inside a macro-looking line.
//!
//! Closure bodies are deliberately not descended into. They are collected as
//! units in their own right, and counting them twice would charge a function for
//! complexity a reader never has to unpack inline.

use syn::visit::Visit;
use syn::{BinOp, Block, Expr, ExprIf};

/// What one body scored.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    /// Independent paths, starting at one.
    pub cyclomatic: usize,
    /// Nesting-weighted branch count.
    pub cognitive: usize,
    /// Deepest nested control structure.
    pub nesting: usize,
}

/// Scores a function body.
#[must_use]
pub fn score(block: &Block) -> Score {
    let mut scorer = Scorer::default();
    scorer.visit_block(block);
    Score {
        cyclomatic: scorer.cyclomatic + 1,
        cognitive: scorer.cognitive,
        nesting: scorer.max_depth,
    }
}

#[derive(Default)]
struct Scorer {
    cyclomatic: usize,
    cognitive: usize,
    depth: usize,
    max_depth: usize,
}

impl Scorer {
    /// Counts a branch and walks its children one level deeper.
    fn branch(&mut self, expr: &Expr, paths: usize) {
        self.cyclomatic += paths;
        self.cognitive += 1 + self.depth;
        self.nested(|scorer| syn::visit::visit_expr(scorer, expr));
    }

    /// Runs `walk` one nesting level deeper, recording the deepest point reached.
    fn nested(&mut self, walk: impl FnOnce(&mut Self)) {
        self.depth += 1;
        self.max_depth = self.max_depth.max(self.depth);
        walk(self);
        self.depth -= 1;
    }

    /// Scores an `if`, treating `else if` as a flat chain rather than nesting.
    ///
    /// Syntactically `else if` *is* a nested `if`, but nobody reads it that way,
    /// and charging it depth would push code toward the `match` or early-return
    /// rewrite it was already fine without. Each arm still costs a path.
    fn if_chain(&mut self, node: &ExprIf) {
        self.cyclomatic += 1;
        self.cognitive += 1 + self.depth;
        self.visit_expr(&node.cond);
        self.nested(|scorer| scorer.visit_block(&node.then_branch));

        let Some((_, otherwise)) = node.else_branch.as_ref() else {
            return;
        };
        match otherwise.as_ref() {
            Expr::If(inner) => self.if_chain(inner),
            other => self.nested(|scorer| scorer.visit_expr(other)),
        }
    }
}

impl<'ast> Visit<'ast> for Scorer {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            // A closure is its own unit; see the module note.
            Expr::Closure(_) => (),
            Expr::If(node) => self.if_chain(node),
            Expr::While(_) | Expr::ForLoop(_) | Expr::Loop(_) => self.branch(expr, 1),
            // Every arm past the first is another path through the function.
            Expr::Match(node) => self.branch(expr, node.arms.len().saturating_sub(1)),
            other => self.visit_linear(other),
        }
    }
}

impl Scorer {
    /// Expressions that add cost without nesting: `&&`, `||` and `?`.
    fn visit_linear(&mut self, expr: &Expr) {
        match expr {
            Expr::Binary(node) if is_short_circuit(node.op) => {
                self.cyclomatic += 1;
                self.cognitive += 1;
            }
            Expr::Try(_) => self.cyclomatic += 1,
            _ => (),
        }
        syn::visit::visit_expr(self, expr);
    }
}

const fn is_short_circuit(op: BinOp) -> bool {
    matches!(op, BinOp::And(_) | BinOp::Or(_))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::score;

    fn score_of(body: &str) -> super::Score {
        let item: syn::ItemFn = syn::parse_str(body).expect("test fixture should parse");
        score(&item.block)
    }

    #[test]
    fn a_straight_line_function_scores_one() {
        let found = score_of("fn f() { let a = 1; let b = 2; }");
        assert_eq!(found.cyclomatic, 1);
        assert_eq!(found.cognitive, 0);
        assert_eq!(found.nesting, 0);
    }

    #[test]
    fn one_branch_adds_one_path() {
        assert_eq!(score_of("fn f(a: bool) { if a { g(); } }").cyclomatic, 2);
    }

    #[test]
    fn nesting_weights_cognitive_but_not_cyclomatic() {
        let flat = score_of("fn f(a: bool, b: bool) { if a { g(); } if b { g(); } }");
        let nested = score_of("fn f(a: bool, b: bool) { if a { if b { g(); } } }");
        assert_eq!(flat.cyclomatic, nested.cyclomatic, "same number of paths");
        assert!(
            nested.cognitive > flat.cognitive,
            "nested code costs more to read"
        );
        assert_eq!(nested.nesting, 2);
    }

    #[test]
    fn match_arms_past_the_first_each_add_a_path() {
        let found = score_of("fn f(a: u8) { match a { 1 => g(), 2 => g(), _ => g() } }");
        assert_eq!(found.cyclomatic, 3);
    }

    #[test]
    fn short_circuit_operators_are_branches() {
        assert_eq!(
            score_of("fn f(a: bool, b: bool) -> bool { a && b }").cyclomatic,
            2
        );
    }

    #[test]
    fn the_question_mark_is_a_path() {
        assert_eq!(
            score_of("fn f() -> Result<(), E> { g()?; Ok(()) }").cyclomatic,
            2
        );
    }

    #[test]
    fn an_else_if_chain_reads_flat_and_is_scored_flat() {
        let chain = score_of(
            "fn f(a: u8) { if a == 1 { g(); } else if a == 2 { g(); } else if a == 3 { g(); } }",
        );
        assert_eq!(
            chain.nesting, 1,
            "an else-if chain is not three levels deep"
        );
        assert_eq!(chain.cyclomatic, 4, "but each arm is still a path");
    }

    #[test]
    fn a_genuinely_nested_if_still_costs_depth() {
        let nested = score_of("fn f(a: bool, b: bool) { if a { if b { g(); } } }");
        assert_eq!(nested.nesting, 2);
    }

    #[test]
    fn closure_bodies_are_not_charged_to_the_parent() {
        let found = score_of("fn f() { let c = |a: bool| if a { g(); }; }");
        assert_eq!(found.cyclomatic, 1, "the closure is scored as its own unit");
    }
}
