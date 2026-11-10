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
//! Phase 9b, beside `solve_stray_terminators`: a `;` that a grammar hangs beside its statement
//! rather than inside it goes with that statement. SQL's `program` holds `statement ; statement ;`
//! as siblings, so nothing ties a `;` to the statement it ends, and the tree edit distance pairs
//! the `;`s of a file by position: a statement inserted in the middle shifts every later `;` onto
//! the previous statement's partner (sql-postgres-postgres-add-only). A reader sees each `;` stay
//! at the end of its statement, and the inserted statement's `;` new with it.
//!
//! A `;` leaf whose previous sibling is a statement paired with another is re-paired with the `;`
//! directly after that partner, when there is one. Both are `;`, so the change is cost-neutral.
//! The `;` it leaves behind on either side is left undecided for `solve_unresolved_nodes`, which
//! deletes or inserts it with its statement.

use crate::code::ASTMetadata;
use crate::diff::{ASTDiff, ASTMapping, ASTMappingReason, PassCtx};

/// The node right after `id` among its parent's children, and the one right before it.
fn neighbours(meta: &ASTMetadata, id: usize) -> (Option<usize>, Option<usize>) {
    let Some(siblings) = meta
        .node_to_parent
        .get(&id)
        .and_then(|parent| meta.node_info.get(parent))
        .map(|info| &info.children)
    else {
        return (None, None);
    };
    let Some(index) = siblings.iter().position(|&sibling| sibling == id) else {
        return (None, None);
    };
    (
        siblings.get(index + 1).copied(),
        index.checked_sub(1).map(|previous| siblings[previous]),
    )
}

fn is_semicolon(meta: &ASTMetadata, id: usize) -> bool {
    meta.node_info
        .get(&id)
        .is_some_and(|info| info.kind == ";" && info.children.is_empty())
}

/// The statement a sibling `;` ends: its previous sibling, when that is an internal node and the
/// two share a parent that holds several such nodes. A `;` that closes a single statement from
/// inside (`expression_statement`, `x = a;`) is `solve_stray_terminators`' case: there the node
/// before it is the statement's value, which may have moved anywhere.
fn ended_statement(meta: &ASTMetadata, id: usize) -> Option<usize> {
    if !is_semicolon(meta, id) {
        return None;
    }
    let parent = meta.node_to_parent.get(&id)?;
    let internal_children = meta
        .node_info
        .get(parent)?
        .children
        .iter()
        .filter(|&&child| !meta.is_leaf(child))
        .take(2)
        .count();
    if internal_children < 2 {
        return None;
    }
    let (_, previous) = neighbours(meta, id);
    previous.filter(|&previous| !meta.is_leaf(previous))
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    let mut moves: Vec<(usize, usize)> = Vec::new();
    for (&id, info) in &before.node_info {
        if info.kind != ";" {
            continue;
        }
        let Some(statement) = ended_statement(before, id) else {
            continue;
        };
        let Some(&partner) = diff.before_node_map.get(&statement) else {
            continue;
        };
        if partner == 0 {
            continue;
        }
        let (Some(target), _) = neighbours(after, partner) else {
            continue;
        };
        if !is_semicolon(after, target) || diff.before_node_map.get(&id) == Some(&target) {
            continue;
        }
        moves.push((id, target));
    }
    // Sorted so the order of removals, and so `mapping`'s history, is reproducible run to run.
    moves.sort_unstable();
    for &(semicolon, target) in &moves {
        if let Some(&old) = diff.before_node_map.get(&semicolon)
            && old != 0
        {
            diff.remove_match_mapping(semicolon, old);
        } else if diff.before_node_map.get(&semicolon) == Some(&0) {
            diff.remove_delete_mapping(semicolon);
        }
        if let Some(&old) = diff.after_node_map.get(&target)
            && old != 0
        {
            diff.remove_match_mapping(old, target);
        } else if diff.after_node_map.get(&target) == Some(&0) {
            diff.remove_insert_mapping(target);
        }
        diff.add_mapping(
            semicolon,
            target,
            ASTMapping::identical(ASTMappingReason::SiblingTerminator),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::diff_code;

    /// The ids of every `;` leaf, in document order.
    fn semicolons(code: &Code) -> Vec<usize> {
        let mut found = Vec::new();
        let mut stack = vec![code.ast.as_ref().unwrap().root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == ";" && node.child_count() == 0 {
                found.push((node.start_byte(), node.id()));
            }
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
        found.sort_unstable();
        found.into_iter().map(|(_, id)| id).collect()
    }

    /// sql-postgres-postgres-add-only, minimized: a statement inserted between two others.
    #[test]
    fn each_semicolon_stays_with_its_statement_when_one_is_inserted_before_it() {
        let before = Code::from_string(
            "select a from t;\nselect b from u;\nselect c from v;\n",
            &Language::SQL,
        );
        let after = Code::from_string(
            "select a from t;\nselect x from y where z = 1;\nselect b from u;\nselect c from v;\n",
            &Language::SQL,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        let (old, new) = (semicolons(&before), semicolons(&after));
        assert_eq!(diff.before_node_map.get(&old[0]), Some(&new[0]));
        assert_eq!(diff.before_node_map.get(&old[1]), Some(&new[2]));
        assert_eq!(diff.before_node_map.get(&old[2]), Some(&new[3]));
        assert_eq!(
            diff.after_node_map.get(&new[1]),
            Some(&0),
            "the new statement's `;`"
        );
    }

    /// A `;` inside its statement is `solve_stray_terminators`' case and stays untouched here:
    /// `x = a;` becoming `return a;` keeps no `;` across the two statements.
    #[test]
    fn a_semicolon_inside_its_statement_is_not_re_paired_after_its_value() {
        let before = Code::from_string(
            "class C {\n    int m() {\n        x = a;\n        return 0;\n    }\n}\n",
            &Language::Java,
        );
        let after = Code::from_string(
            "class C {\n    int m() {\n        return a;\n    }\n}\n",
            &Language::Java,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        assert!(
            diff.mapping
                .values()
                .all(|mapping| mapping.reason != crate::diff::ASTMappingReason::SiblingTerminator)
        );
    }
}
