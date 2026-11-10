/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, version 3 of the License.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */
//! Phase 9, before `solve_wrap_growth`: a block whose statements gained a wrapper keeps its own
//! delimiters. `{ a(); b(); }` becoming `{ try { a(); b(); } catch ... }` hashes the old block
//! identical to the new inner one, so phase 1 pairs it there, braces included, and reports the
//! method's own braces as new. A reader sees the method's braces where they were and a new `try`
//! with braces of its own around the old statements (java-add-exception-handling,
//! typescript-apache-echarts-envelop-2-lines-with-an-if-block).
//!
//! The pass re-points the old block and its first and last token to the block that sits in its
//! slot on the other side: the child of its parent's partner that has the old block's kind and
//! encloses the new one through levels that are all new. The statements keep their partners in the
//! inner block, which `solve_wrap_growth` then tags. Conditions:
//!
//! * the old block's parent is paired, and the outer block is a direct child of that partner with
//!   the old block's kind: the slot is the same, so the outer block is the old one grown;
//! * every level between the outer block and the inner one is unpaired (the wrapper is new
//!   code), and the outer block and its delimiters are unpaired too;
//! * the block holds statements (`nodes::is_statement_sequence_body`): a call wrapped into another
//!   call keeps its own parentheses, since there the wrapper is what is new;
//! * both blocks open and close with the same tokens, so the re-pointed delimiters stay
//!   `Identical` and the change is cost-neutral.
//!
//! One direction only. A wrapper removed is not the mirror image: nested `if let`s collapsed into
//! one `if let` chain keep the innermost block, whose statements are the ones that stayed
//! (`rust-next-font-imports-generator`), and the corpus has no removed wrapper that reads the other
//! way.

use crate::code::ASTMetadata;
use crate::diff::PassCtx;
use crate::diff::{ASTDiff, ASTMapping, ASTMappingOperation, ASTMappingReason};

/// Climb bound from the inner block to the outer one; a wrapper is a handful of levels.
const MAX_WRAP_DEPTH: usize = 6;

/// One direction of the pass: `kept` is the side whose block sits directly in its slot, `wrapped`
/// the side where the matching block is nested inside new code.
struct View<'a> {
    kept: &'a ASTMetadata,
    wrapped: &'a ASTMetadata,
    /// Whether `kept` is the before side.
    kept_is_before: bool,
}

impl View<'_> {
    fn partner_of_kept(&self, diff: &ASTDiff, id: usize) -> Option<usize> {
        let map = if self.kept_is_before {
            &diff.before_node_map
        } else {
            &diff.after_node_map
        };
        map.get(&id).copied().filter(|&partner| partner != 0)
    }

    fn wrapped_is_paired(&self, diff: &ASTDiff, id: usize) -> bool {
        let map = if self.kept_is_before {
            &diff.after_node_map
        } else {
            &diff.before_node_map
        };
        map.get(&id).is_some_and(|&partner| partner != 0)
    }

    /// `(before, after)` for a kept-side and a wrapped-side id.
    fn ordered(&self, kept: usize, wrapped: usize) -> (usize, usize) {
        if self.kept_is_before {
            (kept, wrapped)
        } else {
            (wrapped, kept)
        }
    }

    /// The first and last child of a block-like node when both are tokens.
    fn delimiters(metadata: &ASTMetadata, id: usize) -> Option<(usize, usize)> {
        let children = &metadata.node_info.get(&id)?.children;
        if children.len() < 2 {
            return None;
        }
        let (first, last) = (children[0], children[children.len() - 1]);
        (metadata.is_leaf(first) && metadata.is_leaf(last)).then_some((first, last))
    }

    fn same_token(&self, kept: usize, wrapped: usize) -> bool {
        match (
            self.kept.node_info.get(&kept),
            self.wrapped.node_info.get(&wrapped),
        ) {
            (Some(kept), Some(wrapped)) => kept.kind == wrapped.kind && kept.text == wrapped.text,
            _ => false,
        }
    }

    /// The outer block for the pair `(block, inner)`, when every condition in the module docs
    /// holds.
    fn outer_block(&self, diff: &ASTDiff, block: usize, inner: usize) -> Option<usize> {
        let block_info = self.kept.node_info.get(&block)?;
        let inner_info = self.wrapped.node_info.get(&inner)?;
        if block_info.kind != inner_info.kind || !holds_statements(&block_info.kind) {
            return None;
        }
        let parent = *self.kept.node_to_parent.get(&block)?;
        let parent_partner = self.partner_of_kept(diff, parent)?;

        let mut current = inner;
        let mut outer = None;
        for _ in 0..MAX_WRAP_DEPTH {
            let above = *self.wrapped.node_to_parent.get(&current)?;
            if above == parent_partner {
                outer = Some(current);
                break;
            }
            if self.wrapped_is_paired(diff, above) {
                return None;
            }
            current = above;
        }
        let outer = outer.filter(|&outer| outer != inner)?;
        if self.wrapped.node_info.get(&outer)?.kind != block_info.kind
            || self.wrapped_is_paired(diff, outer)
        {
            return None;
        }

        let (open, close) = Self::delimiters(self.kept, block)?;
        let (outer_open, outer_close) = Self::delimiters(self.wrapped, outer)?;
        let (inner_open, inner_close) = Self::delimiters(self.wrapped, inner)?;
        let tokens_agree = self.same_token(open, outer_open) && self.same_token(close, outer_close);
        let delimiters_were_inner = self.partner_of_kept(diff, open) == Some(inner_open)
            && self.partner_of_kept(diff, close) == Some(inner_close);
        let outer_delimiters_free =
            !self.wrapped_is_paired(diff, outer_open) && !self.wrapped_is_paired(diff, outer_close);
        (tokens_agree && delimiters_were_inner && outer_delimiters_free).then_some(outer)
    }
}

/// Whether `kind` holds a body's statements. Only those: an argument list wrapped into another
/// call (`f(x)` becoming `g(f(x), y)`) keeps its own parentheses in every reading.
fn holds_statements(kind: &str) -> bool {
    crate::diff::nodes::is_statement_sequence_body(kind) || kind == "statement_block"
}

/// Removes whatever decision `wrapped_id` (a node of the wrapped side) carries, leaving it
/// undecided for `solve_unresolved_nodes`.
fn clear_wrapped(diff: &mut ASTDiff, view: &View, wrapped_id: usize) {
    if view.kept_is_before {
        if diff.after_node_map.get(&wrapped_id) == Some(&0) {
            diff.remove_insert_mapping(wrapped_id);
        }
    } else if diff.before_node_map.get(&wrapped_id) == Some(&0) {
        diff.remove_delete_mapping(wrapped_id);
    }
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let before_metadata = ctx.before_metadata();
    let after_metadata = ctx.after_metadata();
    let preorder = |id: &(usize, usize)| {
        before_metadata
            .node_info
            .get(&id.0)
            .map(|info| info.preorder_index)
            .unwrap_or(usize::MAX)
    };
    // Inner nodes paired `Identical`, in preorder for determinism.
    let mut candidates: Vec<(usize, usize)> = diff
        .mapping
        .iter()
        .filter(|&(&(before_id, after_id), mapping)| {
            before_id != 0
                && after_id != 0
                && mapping.operation == ASTMappingOperation::Identical
                && !before_metadata.is_leaf(before_id)
        })
        .map(|(&ids, _)| ids)
        .collect();
    if candidates.is_empty() {
        return;
    }
    candidates.sort_unstable_by_key(preorder);

    let views = [View {
        kept: before_metadata,
        wrapped: after_metadata,
        kept_is_before: true,
    }];
    for (before_id, after_id) in candidates {
        for view in &views {
            let (block, inner) = view.ordered(before_id, after_id);
            if !diff.mapping.contains_key(&(before_id, after_id)) {
                break;
            }
            let Some(outer) = view.outer_block(diff, block, inner) else {
                continue;
            };
            let (
                Some((open, close)),
                Some((inner_open, inner_close)),
                Some((outer_open, outer_close)),
            ) = (
                View::delimiters(view.kept, block),
                View::delimiters(view.wrapped, inner),
                View::delimiters(view.wrapped, outer),
            )
            else {
                continue;
            };
            for (kept_id, inner_id, outer_id, mapping) in [
                (
                    block,
                    inner,
                    outer,
                    ASTMapping::matched_not_identical(ASTMappingReason::WrappedBlock),
                ),
                (
                    open,
                    inner_open,
                    outer_open,
                    ASTMapping::identical(ASTMappingReason::WrappedBlock),
                ),
                (
                    close,
                    inner_close,
                    outer_close,
                    ASTMapping::identical(ASTMappingReason::WrappedBlock),
                ),
            ] {
                let (old_before, old_after) = view.ordered(kept_id, inner_id);
                diff.remove_match_mapping(old_before, old_after);
                clear_wrapped(diff, view, outer_id);
                let (new_before, new_after) = view.ordered(kept_id, outer_id);
                diff.add_mapping(new_before, new_after, mapping);
            }
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::{ASTMappingOperation, diff_code};
    use tree_sitter::Node;

    /// Every node of `kind` in document order.
    fn all_of_kind<'a>(node: Node<'a>, kind: &str, found: &mut Vec<Node<'a>>) {
        if node.kind() == kind {
            found.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            all_of_kind(child, kind, found);
        }
    }

    fn blocks<'a>(code: &'a Code, kind: &str) -> Vec<Node<'a>> {
        let mut found = Vec::new();
        all_of_kind(code.ast.as_ref().unwrap().root_node(), kind, &mut found);
        found
    }

    fn delimiters(block: Node) -> [Option<Node>; 2] {
        [block.child(0), block.child(block.child_count() - 1)]
    }

    fn partner(code_map: &rustc_hash::FxHashMap<usize, usize>, node: Node) -> Option<usize> {
        code_map.get(&node.id()).copied()
    }

    /// java-add-exception-handling, minimized: the method keeps its braces, the `try` brings its own.
    #[test]
    fn a_method_body_wrapped_in_try_keeps_its_braces() {
        let before = Code::from_string(
            "class C {\n  void f() {\n    a();\n    b();\n  }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n  void f() {\n    try {\n      a();\n      b();\n    } catch (E e) {\n      c();\n    }\n  }\n}\n",
            &Language::Java,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        let (old, new) = (blocks(&before, "block"), blocks(&after, "block"));
        // `new[0]` is the method body, `new[1]` the `try`'s block.
        assert_eq!(partner(&diff.before_node_map, old[0]), Some(new[0].id()));
        for (old_delimiter, new_delimiter) in delimiters(old[0]).into_iter().zip(delimiters(new[0]))
        {
            assert_eq!(
                partner(&diff.before_node_map, old_delimiter.unwrap()),
                Some(new_delimiter.unwrap().id()),
                "the method's own delimiters stay the method's",
            );
        }
        assert_eq!(
            partner(&diff.after_node_map, new[1].child(0).unwrap()),
            Some(0),
            "the try block's `{{` is new",
        );
        // The statements stay paired, now inside the `try`.
        let statement = new[1].named_child(0).unwrap();
        assert_eq!(
            partner(&diff.after_node_map, statement),
            Some(old[0].named_child(0).unwrap().id())
        );
    }

    /// A wrapper removed is left to the passes before: the inner block keeps its statements.
    #[test]
    fn a_method_body_unwrapped_from_try_is_left_alone() {
        let before = Code::from_string(
            "class C {\n  void f() {\n    try {\n      a();\n      b();\n    } catch (E e) {\n      c();\n    }\n  }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n  void f() {\n    a();\n    b();\n  }\n}\n",
            &Language::Java,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        assert!(
            diff.mapping
                .values()
                .all(|mapping| mapping.reason != crate::diff::ASTMappingReason::WrappedBlock)
        );
    }

    /// A call wrapped into another call keeps its own parentheses: argument lists hold no
    /// statements (java-defects4j-closure-112-typeinference).
    #[test]
    fn a_call_wrapped_into_another_call_keeps_its_own_parentheses() {
        let before = Code::from_string(
            "class C {\n  void f() {\n    x = g(a, b);\n  }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n  void f() {\n    x = h(g(a, b), c);\n  }\n}\n",
            &Language::Java,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        let (old, new) = (
            blocks(&before, "argument_list"),
            blocks(&after, "argument_list"),
        );
        // `new[0]` is `h(...)`'s list, `new[1]` the old call's.
        assert_eq!(partner(&diff.before_node_map, old[0]), Some(new[1].id()));
        assert_eq!(
            diff.mapping
                .get(&(old[0].child(0).unwrap().id(), new[1].child(0).unwrap().id()))
                .map(|mapping| mapping.operation.clone()),
            Some(ASTMappingOperation::Identical)
        );
    }

    /// The outer block must sit in the old block's slot: a block that moved into a different
    /// method's wrapper is not this pass's to re-point.
    #[test]
    fn a_block_whose_slot_partner_is_already_paired_is_left_alone() {
        let before = Code::from_string(
            "class C {\n  void f() {\n    a();\n    b();\n  }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n  void f() {\n    a();\n    b();\n  }\n  void g() {\n    if (x) {\n      a();\n      b();\n    }\n  }\n}\n",
            &Language::Java,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        let (old, new) = (blocks(&before, "block"), blocks(&after, "block"));
        assert_eq!(partner(&diff.before_node_map, old[0]), Some(new[0].id()));
        assert_eq!(
            partner(&diff.before_node_map, old[0].child(0).unwrap()),
            Some(new[0].child(0).unwrap().id())
        );
    }
}
