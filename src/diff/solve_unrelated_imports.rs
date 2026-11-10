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
//! Phase 9b: two import statements that name nothing in common are not one import edited. Phase
//! 1 already refuses to pair imports by shape alone (`nodes::is_import_kind`), but the whole-file
//! fallback still lines up a run of removed includes with a run of added ones by position, so
//! `#include "ggml.h"` comes out as `#include <functional>` with its path renamed
//! (`c-ollama-ollama-change-imports`). A reader sees one import removed and another added.
//!
//! A paired import whose path tokens (alphanumeric runs of its text, keywords dropped as in
//! `solve_syntax_aware_matching`'s import passes) share none with its partner's is unpaired with
//! everything paired inside it, for `solve_unresolved_nodes` to delete and insert. Imports that
//! share a token (a moved module, a changed symbol list) are left to the passes that paired them.

use rustc_hash::FxHashSet;

use crate::code::ASTMetadata;
use crate::diff::solve_syntax_aware_matching::IMPORT_KEYWORD_TOKENS;
use crate::diff::{ASTDiff, ASTMappingOperation, PassCtx, nodes};

/// Every node of `id`'s subtree, `id` included.
fn subtree(meta: &ASTMetadata, id: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = vec![id];
    while let Some(node) = stack.pop() {
        out.push(node);
        if let Some(info) = meta.node_info.get(&node) {
            stack.extend(info.children.iter().copied());
        }
    }
    out
}

/// The import's path tokens: its leaves' alphanumeric runs, keywords left out.
fn path_tokens(meta: &ASTMetadata, nodes: &[usize]) -> FxHashSet<String> {
    let mut tokens = FxHashSet::default();
    for id in nodes {
        let Some(info) = meta.node_info.get(id) else {
            continue;
        };
        if !info.children.is_empty() {
            continue;
        }
        for token in info.text.split(|c: char| !c.is_alphanumeric()) {
            if !token.is_empty()
                && !IMPORT_KEYWORD_TOKENS
                    .iter()
                    .any(|keyword| token.eq_ignore_ascii_case(keyword))
            {
                tokens.insert(token.to_string());
            }
        }
    }
    tokens
}

pub fn solve(ctx: &PassCtx, diff: &mut ASTDiff) {
    let (before, after) = (ctx.before_metadata(), ctx.after_metadata());
    let language = ctx.language();
    let mut unrelated: Vec<(usize, usize)> = diff
        .mapping
        .iter()
        .filter(|&(&(b, a), mapping)| {
            b != 0
                && a != 0
                && mapping.operation != ASTMappingOperation::Identical
                && before
                    .node_info
                    .get(&b)
                    .is_some_and(|info| nodes::is_import_kind(&info.kind, &language))
                && after
                    .node_info
                    .get(&a)
                    .is_some_and(|info| nodes::is_import_kind(&info.kind, &language))
        })
        .map(|(&pair, _)| pair)
        .collect();
    // Sorted so the removal order, and so `mapping`'s history, is reproducible run to run.
    unrelated.sort_unstable();

    for (b, a) in unrelated {
        let before_nodes = subtree(before, b);
        let after_nodes = subtree(after, a);
        let (before_tokens, after_tokens) = (
            path_tokens(before, &before_nodes),
            path_tokens(after, &after_nodes),
        );
        if before_tokens.is_empty()
            || after_tokens.is_empty()
            || !before_tokens.is_disjoint(&after_tokens)
        {
            continue;
        }
        let inside_after: FxHashSet<usize> = after_nodes.into_iter().collect();
        for node in before_nodes {
            if let Some(&partner) = diff.before_node_map.get(&node)
                && partner != 0
                && inside_after.contains(&partner)
            {
                diff.remove_match_mapping(node, partner);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::code::{Code, Language};
    use crate::diff::diff_code;

    fn includes(code: &Code) -> Vec<usize> {
        let root = code.ast.as_ref().unwrap().root_node();
        let mut cursor = root.walk();
        root.children(&mut cursor)
            .filter(|node| node.kind() == "preproc_include")
            .map(|node| node.id())
            .collect()
    }

    /// c-ollama-ollama-change-imports, minimized: includes replaced by unrelated ones.
    #[test]
    fn unrelated_includes_are_removed_and_added_not_renamed() {
        let before = Code::from_string(
            "#include \"ggml.h\"\n#include \"json.hpp\"\nint f();\n",
            &Language::C,
        );
        let after = Code::from_string(
            "#include <functional>\n#include <string>\nint f();\n",
            &Language::C,
        );
        let diff = diff_code(&before, &after).ast.unwrap();
        for id in includes(&before) {
            assert_eq!(diff.before_node_map.get(&id), Some(&0));
        }
        for id in includes(&after) {
            assert_eq!(diff.after_node_map.get(&id), Some(&0));
        }
    }

    /// An include whose path kept a token is an edited include and stays paired.
    #[test]
    fn an_include_that_moved_directory_stays_paired() {
        let before = Code::from_string("#include \"json.hpp\"\nint f();\n", &Language::C);
        let after = Code::from_string("#include <nlohmann/json.hpp>\nint f();\n", &Language::C);
        let diff = diff_code(&before, &after).ast.unwrap();
        let (old, new) = (includes(&before), includes(&after));
        assert_eq!(diff.before_node_map.get(&old[0]), Some(&new[0]));
    }
}
