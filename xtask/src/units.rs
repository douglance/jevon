//! Collecting every unit of code that can get too big.
//!
//! The closure case is the one that earns this module: a multi-line closure is
//! syntactically an expression, so a rule that only walks `fn` items lets the
//! longest, most tangled code in a file pass unmeasured.

use syn::spanned::Spanned as _;
use syn::visit::Visit;
use syn::{Block, ExprClosure, ImplItemFn, ItemFn, Signature, TraitItemFn};

use crate::complexity::Unit;
use crate::metrics;

#[derive(Default)]
pub(crate) struct Collector {
    pub(crate) units: Vec<Unit>,
}

impl Collector {
    /// Records a closure only when it spans several lines.
    ///
    /// A one-liner cannot breach any limit that matters, and flagging them would
    /// bury the real findings in noise.
    fn push_multi_line_closure(&mut self, node: &ExprClosure) {
        let syn::Expr::Block(block) = node.body.as_ref() else {
            return;
        };
        let span = block.span();
        if span.end().line > span.start().line {
            self.push("closure".to_owned(), None, &block.block);
        }
    }

    fn push(&mut self, name: String, signature: Option<&Signature>, block: &Block) {
        let span = block.span();
        self.units.push(Unit {
            name,
            line: span.start().line,
            end_line: span.end().line,
            arguments: signature.map_or(0, |sig| sig.inputs.len()),
            score: metrics::score(block),
        });
    }
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.push(
            format!("fn {}", node.sig.ident),
            Some(&node.sig),
            &node.block,
        );
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        self.push(
            format!("method {}", node.sig.ident),
            Some(&node.sig),
            &node.block,
        );
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        if let Some(block) = node.default.as_ref() {
            self.push(
                format!("default fn {}", node.sig.ident),
                Some(&node.sig),
                block,
            );
        }
        syn::visit::visit_trait_item_fn(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast ExprClosure) {
        self.push_multi_line_closure(node);
        syn::visit::visit_expr_closure(self, node);
    }
}
