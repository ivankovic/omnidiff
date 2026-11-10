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
//! Phase 9b: undoes a rename that crossed into another construct. A single-token node the
//! mapping renames (`Update`) must sit in the same place on both sides: under parents that are
//! paired with each other, or at least of one kind. Otherwise the tree edit distance has taken one
//! leaf of a replaced expression and relabelled it into the new one - `RegularTimePeriod` of
//! `RegularTimePeriod.DEFAULT_TIME_ZONE` renamed into `zone` (`java-defects4j-chart-8-week`) -
//! where the reader sees the old expression gone and a new one in its place.
//!
//! A pair across kinds (`MatchButNotIdentical` leaves: the type `IllegalStateException` of a
//! `throw new` taken for the variable `nextLineTabStop` of the assignment that replaced it,
//! `java-defects4j-cli-24-helpformatter`) is the same relabelling and is undone on the same
//! evidence. One that reads the same on both sides is not: a name moved into a context the
//! grammar parses differently (`self.added` into a Rust macro's token tree, where `added` is a
//! plain `identifier`) is still that name. Dropping the lost-sibling condition for cross-kind
//! pairs undoes renames a reader keeps (`[` -> `(` of a type argument list turned call).
//!
//! Only unpairs; phase 10 records the delete and the insert.

use crate::code::ASTMetadata;
use crate::diff::{ASTDiff, ASTMappingOperation, PassCtx};

/// Whether another named child of `parent` than `leaf` has no partner - deleted, inserted on the
/// after side, or still undecided: the construct lost content beside the renamed leaf, so it was replaced rather than
/// reduced to that leaf (`!hasColorSpaceId` loses only its `!`).
fn lost_named_sibling(
    meta: &ASTMetadata,
    parent: usize,
    leaf: usize,
    node_map: &rustc_hash::FxHashMap<usize, usize>,
) -> bool {
    meta.node_info.get(&parent).is_some_and(|info| {
        info.children.iter().any(|&child| {
            child != leaf
                && node_map.get(&child).is_none_or(|&partner| partner == 0)
                && meta.node_info.get(&child).is_some_and(|c| c.is_named)
        })
    })
}

fn text(meta: &ASTMetadata, id: usize) -> Option<&str> {
    meta.node_info.get(&id).map(|info| info.text.as_str())
}

fn kind(meta: &ASTMetadata, id: usize) -> Option<&str> {
    meta.node_info.get(&id).map(|info| info.kind.as_str())
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    let is_leaf = |meta: &ASTMetadata, id: usize| {
        meta.node_info
            .get(&id)
            .is_some_and(|info| info.children.is_empty())
    };

    let mut stray: Vec<(usize, usize)> = diff
        .mapping
        .iter()
        .filter(|&(&(b, a), mapping)| {
            b != 0
                && a != 0
                && matches!(
                    mapping.operation,
                    ASTMappingOperation::Update | ASTMappingOperation::MatchButNotIdentical
                )
        })
        .map(|(&pair, _)| pair)
        .filter(|&(b, a)| {
            if !is_leaf(before, b) || !is_leaf(after, a) {
                return false;
            }
            // The same name in another grammar context (`self.added` moved into a macro's token
            // tree reads `added` as a plain identifier) is no rename.
            if text(before, b) == text(after, a) {
                return false;
            }
            let (Some(&before_parent), Some(&after_parent)) =
                (before.node_to_parent.get(&b), after.node_to_parent.get(&a))
            else {
                return false;
            };
            diff.before_node_map.get(&before_parent) != Some(&after_parent)
                && kind(before, before_parent) != kind(after, after_parent)
                && lost_named_sibling(before, before_parent, b, &diff.before_node_map)
        })
        .collect();
    // Sorted so the removal order, and so `mapping`'s history, is reproducible run to run.
    stray.sort_unstable();
    for (b, a) in stray {
        diff.remove_match_mapping(b, a);
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::diff_code;

    /// The id of the first leaf reading `text`, in document order.
    fn leaf(code: &Code, text: &str) -> usize {
        let mut found = Vec::new();
        let mut stack = vec![code.ast.as_ref().unwrap().root_node()];
        while let Some(node) = stack.pop() {
            if node.child_count() == 0 && code.contents.get(node.byte_range()) == Some(text) {
                found.push((node.start_byte(), node.id()));
            }
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
        found.sort_unstable();
        found[0].1
    }

    /// java-defects4j-cli-24-helpformatter, minimized: a `throw new` replaced by an assignment.
    #[test]
    fn a_type_name_is_not_renamed_into_the_variable_that_replaced_its_statement() {
        let before = Code::from_string(
            "class C {\n    void m() {\n        if (a) {\n            throw new IllegalStateException(\"no room\");\n        }\n    }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n    void m() {\n        if (a) {\n            nextLineTabStop = width - 1;\n        }\n    }\n}\n",
            &Language::Java,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        assert_eq!(
            diff.before_node_map
                .get(&leaf(&before, "IllegalStateException")),
            Some(&0)
        );
    }

    /// A name moved into a macro's token tree changes kind, not identity.
    #[test]
    fn a_name_that_reads_the_same_across_kinds_keeps_its_partner() {
        let before = Code::from_string(
            "fn f(s: &S) -> String {\n    Label::new(s.added.to_string()).render()\n}\n",
            &Language::Rust,
        );
        let after = Code::from_string(
            "fn f(s: &S) -> String {\n    Label::new(format!(\"+{}\", s.added)).render()\n}\n",
            &Language::Rust,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        let partner = diff.before_node_map.get(&leaf(&before, "added")).copied();
        assert_ne!(partner, Some(0));
        assert_ne!(partner, None);
    }
}
