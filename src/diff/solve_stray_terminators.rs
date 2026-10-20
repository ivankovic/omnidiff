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

//! Phase 9b, after stray renames: undoes a pairing of two statement terminators whose statements
//! are not paired with each other. A `;` belongs to the statement it closes; when that statement
//! is deleted, inserted, or paired with a different one, its `;` goes with it. The tree edit
//! distance pairs identical `;` leaves cheaply wherever two sequences of statements line up, so
//! `break;` replaced by `continue;` otherwise keeps a `;` the reader sees outlive the `break` before
//! it - painted as `break` gone and `continue` new around one surviving `;`.
//!
//! Only unpairs; phase 10 records the delete and the insert.

use crate::code::ASTMetadata;
use crate::diff::{ASTDiff, PassCtx};

/// The parent `id` closes, if `id` is a `;` leaf and its parent's last child.
fn closed_statement(meta: &ASTMetadata, id: usize) -> Option<usize> {
    let info = meta.node_info.get(&id)?;
    if info.kind != ";" || !info.children.is_empty() {
        return None;
    }
    let parent = *meta.node_to_parent.get(&id)?;
    let siblings = &meta.node_info.get(&parent)?.children;
    (siblings.last() == Some(&id) && siblings.len() > 1).then_some(parent)
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    let mut stray: Vec<(usize, usize)> = diff
        .mapping
        .keys()
        .filter(|&&(b, a)| b != 0 && a != 0)
        .filter(|&&(b, a)| {
            let (Some(before_statement), Some(after_statement)) =
                (closed_statement(before, b), closed_statement(after, a))
            else {
                return false;
            };
            diff.before_node_map.get(&before_statement) != Some(&after_statement)
        })
        .copied()
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

    fn semicolon_partner(before: &Code, after: &Code, row: usize) -> Option<usize> {
        let diff = diff_code(before, after).ast.expect("ast diff");
        let root = before.ast.as_ref().unwrap().root_node();
        let mut cursor = root.walk();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if node.kind() == ";" && node.start_position().row == row {
                return diff
                    .before_node_map
                    .get(&node.id())
                    .copied()
                    .filter(|&id| id != 0);
            }
            stack.extend(node.children(&mut cursor));
        }
        panic!("no `;` on row {row}");
    }

    /// java-defects4j-cli-28-parser, minimized.
    #[test]
    fn a_replaced_statement_does_not_keep_its_semicolon() {
        let before = Code::from_string(
            "class C {\n    void m() {\n        for (;;) {\n            if (a) {\n                break;\n            }\n        }\n    }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n    void m() {\n        for (;;) {\n            if (a) {\n                continue;\n            }\n        }\n    }\n}\n",
            &Language::Java,
        );
        assert_eq!(semicolon_partner(&before, &after, 4), None);
    }

    #[test]
    fn a_surviving_statement_keeps_its_semicolon() {
        let before = Code::from_string(
            "class C {\n    void m() {\n        a(1);\n    }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n    void m() {\n        a(2);\n    }\n}\n",
            &Language::Java,
        );
        assert!(semicolon_partner(&before, &after, 2).is_some());
    }
}
