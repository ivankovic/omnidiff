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

//! Phase 9b, beside stray renames: re-pairs a renamed leaf with its unpaired twin. The tree edit
//! distance keeps sibling order, so when a leaf survives but changes place inside its construct
//! it renames whatever now sits in the old slot instead: `e.getMessage()` becoming
//! `ClassUtil.exceptionMessage(e)` renames `e` into `ClassUtil` and inserts the new `e`
//! (`java-defects4j-jacksondatabind-103-*`). Pairing `e` with `e` and inserting `ClassUtil` costs
//! one insert against a rename and an insert, and is what a reader sees: `e` moved into the
//! argument list.
//!
//! For a renamed named leaf, the twin is the one unpaired leaf of the same kind and text inside
//! the partner of the leaf's parent, on the other side. Exactly one candidate on that side, and
//! none left over on this side, so the re-pairing is never a guess between copies. Both
//! directions are tried: the before leaf's twin among the after side's unpaired leaves, and the
//! after leaf's among the before side's.
//!
//! The leaf the rename leaves behind is unpaired; phase 10 records its delete or insert.
//!
//! A leaf that moved within its statement without being renamed over is left unpaired on both
//! sides instead. It is paired with its twin when the twin sits directly in the partner of its
//! statement (not in a statement nested inside it), the statement holds no body (a block, a loop's
//! or a function's), and each is the only unpaired copy of that text in its statement.

use crate::code::ASTMetadata;
use crate::diff::{ASTDiff, ASTMapping, ASTMappingOperation, ASTMappingReason, PassCtx};

/// Every unpaired named leaf under `root` (inclusive) reading like `like`.
fn unpaired_twins(
    meta: &ASTMetadata,
    root: usize,
    like: &crate::code::ASTNodeMetadata,
    node_map: &rustc_hash::FxHashMap<usize, usize>,
) -> Vec<usize> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let Some(info) = meta.node_info.get(&id) else {
            continue;
        };
        if info.children.is_empty() {
            if info.is_named
                && info.kind == like.kind
                && info.text == like.text
                && node_map.get(&id).is_none_or(|&partner| partner == 0)
            {
                found.push(id);
            }
        } else {
            stack.extend(info.children.iter().copied());
        }
    }
    found
}

/// The nearest ancestor of `id` (inclusive) that is a statement, declaration or function: the
/// unit a reader takes a leaf's role from. Rust's `generic_function` is a call with type
/// arguments, not a function.
fn enclosing_statement(meta: &ASTMetadata, id: usize) -> Option<usize> {
    let mut current = id;
    loop {
        let kind = meta.node_info.get(&current)?.kind.as_str();
        if kind.contains("statement")
            || kind.ends_with("declaration")
            || kind.contains("lambda")
            || kind.contains("closure")
            || kind.starts_with("function")
            || kind == "arrow_function"
        {
            return Some(current);
        }
        current = *meta.node_to_parent.get(&current)?;
    }
}

/// The one unpaired twin of `leaf` (on `this` side) inside the partner of its parent on the
/// `other` side, when `leaf`'s own side under its parent holds no other unpaired copy.
fn sole_twin(
    this: &ASTMetadata,
    other: &ASTMetadata,
    leaf: usize,
    renamed_to: usize,
    this_map: &rustc_hash::FxHashMap<usize, usize>,
    other_map: &rustc_hash::FxHashMap<usize, usize>,
) -> Option<usize> {
    let info = this.node_info.get(&leaf)?;
    if !info.is_named || !info.children.is_empty() {
        return None;
    }
    let parent = *this.node_to_parent.get(&leaf)?;
    let partner = *this_map.get(&parent)?;
    if partner == 0 {
        return None;
    }
    let twins = unpaired_twins(other, partner, info, other_map);
    let [twin] = twins[..] else {
        return None;
    };
    // The twin must play its part in the same statement as the leaf renamed in its place, one
    // small enough to read roles from: a loop variable renamed `num` -> `i` is not `num` read
    // again in the body (`python-bugfix-loop`).
    let statement = enclosing_statement(other, twin)?;
    if enclosing_statement(other, renamed_to) != Some(statement) || has_body(other, statement) {
        return None;
    }
    unpaired_twins(this, parent, info, this_map)
        .is_empty()
        .then_some(twin)
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    let mut renames: Vec<(usize, usize)> = diff
        .mapping
        .iter()
        .filter(|&(&(b, a), mapping)| {
            b != 0 && a != 0 && mapping.operation == ASTMappingOperation::Update
        })
        .map(|(&pair, _)| pair)
        .collect();
    // Sorted so the re-pairing order, and so `mapping`'s history, is reproducible run to run.
    renames.sort_unstable();
    for (b, a) in renames {
        let after_twin = sole_twin(
            before,
            after,
            b,
            a,
            &diff.before_node_map,
            &diff.after_node_map,
        );
        let before_twin = sole_twin(
            after,
            before,
            a,
            b,
            &diff.after_node_map,
            &diff.before_node_map,
        );
        if after_twin.is_none() && before_twin.is_none() {
            continue;
        }
        diff.remove_match_mapping(b, a);
        for (before_id, after_id) in [(Some(b), after_twin), (before_twin, Some(a))] {
            let (Some(before_id), Some(after_id)) = (before_id, after_id) else {
                continue;
            };
            diff.remove_delete_mapping(before_id);
            diff.remove_insert_mapping(after_id);
            diff.add_mapping(
                before_id,
                after_id,
                ASTMapping {
                    cost: 0,
                    operation: ASTMappingOperation::Identical,
                    reason: ASTMappingReason::RenamedTwin,
                },
            );
        }
    }
    pair_orphans_within_statements(before, after, diff);
}

/// Whether `id` holds a body - a block, a nested statement or clause, a shell command - below it.
/// Shell's `if` holds its commands with no block between (`shellscript-openlightingproject-ola-real-small-change`). Such a
/// statement is too large to read a moved leaf's role from: the `b` of `i < b.N` is not the `b` of
/// `b.Loop()` in the same `for` (`go-gohugoio-hugo-for-loop-change`).
fn has_body(meta: &ASTMetadata, id: usize) -> bool {
    let mut stack: Vec<usize> = meta
        .node_info
        .get(&id)
        .map(|info| info.children.clone())
        .unwrap_or_default();
    while let Some(id) = stack.pop() {
        let Some(info) = meta.node_info.get(&id) else {
            continue;
        };
        let kind = info.kind.as_str();
        if kind.contains("block")
            || kind.contains("body")
            || kind.contains("compound")
            || kind == "do_group"
            || kind.contains("statement")
            || kind.ends_with("clause")
            || kind == "command"
            || crate::diff::nodes::is_statement_sequence_body(kind)
        {
            return true;
        }
        stack.extend(info.children.iter().copied());
    }
    false
}

/// An unpaired named leaf and its one unpaired twin directly inside the partner of its statement:
/// `get_n(12)` becoming `read::<12, u16>()` (`rust-quietvoid-dovi_tool-change-to-templated-call`)
/// moves the `12` from the arguments into the type arguments, whose parents the tree edit
/// distance cannot pair.
fn pair_orphans_within_statements(before: &ASTMetadata, after: &ASTMetadata, diff: &mut ASTDiff) {
    let mut orphans: Vec<(usize, usize)> = before
        .node_info
        .iter()
        .filter(|&(&id, info)| {
            info.children.is_empty()
                && info.is_named
                && diff
                    .before_node_map
                    .get(&id)
                    .is_none_or(|&partner| partner == 0)
        })
        .map(|(&id, info)| (info.preorder_index, id))
        .collect();
    orphans.sort_unstable();
    for (_, leaf) in orphans {
        let Some(info) = before.node_info.get(&leaf) else {
            continue;
        };
        let Some(statement) = enclosing_statement(before, leaf) else {
            continue;
        };
        let Some(&partner) = diff.before_node_map.get(&statement) else {
            continue;
        };
        if partner == 0 {
            continue;
        }
        let twins = unpaired_twins(after, partner, info, &diff.after_node_map);
        let [twin] = twins[..] else {
            continue;
        };
        if enclosing_statement(after, twin) != Some(partner)
            || has_body(before, statement)
            || has_body(after, partner)
            || unpaired_twins(before, statement, info, &diff.before_node_map) != [leaf]
        {
            continue;
        }
        diff.remove_delete_mapping(leaf);
        diff.remove_insert_mapping(twin);
        diff.add_mapping(
            leaf,
            twin,
            ASTMapping {
                cost: 0,
                operation: ASTMappingOperation::Identical,
                reason: ASTMappingReason::RenamedTwin,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::diff_code;

    fn all_nodes(root: tree_sitter::Node) -> Vec<tree_sitter::Node> {
        let mut all = vec![];
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            all.push(node);
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
        all
    }

    /// Where the after partner of the first before leaf reading `text` starts, if it is paired.
    fn partner_start(before: &Code, after: &Code, text: &str) -> Option<usize> {
        let diff = diff_code(before, after).ast.expect("ast diff");
        let leaf = all_nodes(before.ast.as_ref().unwrap().root_node())
            .into_iter()
            .filter(|n| n.child_count() == 0 && before.contents.get(n.byte_range()) == Some(text))
            .min_by_key(|n| n.start_byte())?;
        let partner = *diff.before_node_map.get(&leaf.id())?;
        all_nodes(after.ast.as_ref().unwrap().root_node())
            .into_iter()
            .find(|n| n.id() == partner)
            .map(|n| n.start_byte())
    }

    #[test]
    fn a_receiver_that_became_the_argument_keeps_its_identity() {
        let before_src = "class C { void f() { log(e.getMessage()); } }";
        let after_src = "class C { void f() { log(ClassUtil.exceptionMessage(e)); } }";
        let before = Code::from_string(before_src, &Language::Java);
        let after = Code::from_string(after_src, &Language::Java);
        let after_start = partner_start(&before, &after, "e").expect("e is paired");
        assert_eq!(
            &after_src[after_start..after_start + 1],
            "e",
            "`e` pairs with the argument `e`, not renamed into `ClassUtil`"
        );
    }

    #[test]
    fn a_renamed_loop_variable_is_not_pulled_into_the_body() {
        let before_src = "def f(nums):\n    for num in nums:\n        print(num)\n";
        let after_src = "def f(nums):\n    for i in range(len(nums)):\n        num = nums[i]\n        print(num)\n";
        let before = Code::from_string(before_src, &Language::Python);
        let after = Code::from_string(after_src, &Language::Python);
        let after_start = partner_start(&before, &after, "num").expect("num is paired");
        assert_eq!(&after_src[after_start..after_start + 1], "i");
    }

    #[test]
    fn a_value_moved_into_type_arguments_keeps_its_identity() {
        let before_src = "fn f() { let a = reader.get_n(12)?; }";
        let after_src = "fn f() { let a = reader.read::<12, u16>()?; }";
        let before = Code::from_string(before_src, &Language::Rust);
        let after = Code::from_string(after_src, &Language::Rust);
        let after_start = partner_start(&before, &after, "12").expect("12 is paired");
        assert_eq!(&after_src[after_start..after_start + 2], "12");
    }

    #[test]
    fn a_plain_rename_stays_a_rename() {
        let before_src = "class C { void f() { log(e.getMessage()); } }";
        let after_src = "class C { void f() { log(ex.getMessage()); } }";
        let before = Code::from_string(before_src, &Language::Java);
        let after = Code::from_string(after_src, &Language::Java);
        let after_start = partner_start(&before, &after, "e").expect("e is paired");
        assert_eq!(&after_src[after_start..after_start + 2], "ex");
    }
}
