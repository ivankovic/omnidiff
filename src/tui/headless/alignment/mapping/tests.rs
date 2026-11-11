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
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use super::*;
use crate::code::Language;
use crate::diff::Diff;

const MOVE_BEFORE: &str = "def total(numbers):\n    s = 0\n    for n in numbers:\n        s += n\n    return s\n\n\ndef mean(numbers):\n    return total(numbers) / len(numbers)\n";
const MOVE_AFTER: &str = "def mean(numbers):\n    return sum(numbers) / len(numbers)\n\n\ndef total(numbers):\n    return sum(numbers)\n";

/// The two sides of a Python pair, parsed, and the engine's diff of them.
fn python(before: &str, after: &str) -> (Code, Code, ASTDiff) {
    let (before, after) = (
        Code::from_string(before, &Language::Python),
        Code::from_string(after, &Language::Python),
    );
    let ast = Diff::pending(&before, &after).finish().ast.unwrap();
    (before, after, ast)
}

/// Each module-level pair of `def`s of the moved-function example and its weight, the mapped
/// node pairs inside the two subtrees.
fn function_weights(before: &Code, after: &Code, ast: &ASTDiff) -> Vec<(String, u64)> {
    fn tree(code: &Code) -> (Vec<Node<'_>>, Vec<u32>, FxHashMap<usize, u32>) {
        let (nodes, end) = preorder(code.ast.as_ref().unwrap().root_node());
        let index: FxHashMap<usize, u32> = nodes
            .iter()
            .enumerate()
            .map(|(at, node)| (node.id(), at as u32))
            .collect();
        (nodes, end, index)
    }
    let ((before_nodes, before_end, before_index), (after_nodes, after_end, after_index)) =
        (tree(before), tree(after));
    let before_side = Side {
        partner: partners(&before_nodes, &ast.before_node_map, &after_index),
        nodes: before_nodes,
        end: before_end,
    };
    let after_side = Side {
        partner: partners(&after_nodes, &ast.after_node_map, &before_index),
        nodes: after_nodes,
        end: after_end,
    };
    let pairs: Vec<(u32, u32)> = before_side
        .children(0)
        .filter(|&old| before_side.partner[old as usize] != NONE)
        .map(|old| (old, before_side.partner[old as usize]))
        .collect();
    let weights = weights(&before_side, &after_side, &pairs);
    pairs
        .iter()
        .zip(weights)
        .map(|(&(old, _), weight)| {
            let node = before_side.nodes[old as usize];
            let name = node
                .child_by_field_name("name")
                .map_or(String::new(), |name| {
                    before.contents[name.byte_range()].to_string()
                });
            (name, weight)
        })
        .collect()
}

/// The design's first example: `mean` is matched throughout (`total` -> `sum` an update), while
/// `total` keeps only its header and its `return`; the heavier `mean` keeps its rows.
#[test]
fn the_rewritten_function_outweighs_the_moved_one() {
    let (before, after, ast) = python(MOVE_BEFORE, MOVE_AFTER);
    let weights = function_weights(&before, &after, &ast);
    eprintln!("{weights:?}");
    let weight = |name: &str| weights.iter().find(|(n, _)| n == name).unwrap().1;
    assert!(weight("mean") > weight("total"), "{weights:?}");

    let lines = node_lines(Trees::of(&before, &after).unwrap(), &before, &after, &ast);
    use LineClass::*;
    assert_eq!(
        lines.old[..9],
        [
            Moved(4),
            Unmatched,
            Unmatched,
            Unmatched,
            Moved(5),
            Blank,
            Blank,
            Kept,
            Kept
        ]
    );
    assert_eq!(
        lines.new[..6],
        [Kept, Kept, Blank, Blank, Moved(0), Moved(4)]
    );
    assert!(
        lines
            .anchors
            .iter()
            .all(|pair| (pair.old, pair.new) == (7, 0) || (pair.old, pair.new) == (8, 1)),
        "{:?}",
        lines.anchors
    );
}

/// The design's second example: `return 1`, mapped from one block into a new one inside it,
/// stays in order inside the kept `if` and keeps its row.
#[test]
fn a_node_mapped_into_a_new_parent_in_order_is_kept() {
    let (before, after, ast) = python(
        "def f(x):\n    if x:\n        return 1\n    return 0\n",
        "def f(x):\n    if x:\n        if x > 1:\n            return 1\n    return 0\n",
    );
    let lines = node_lines(Trees::of(&before, &after).unwrap(), &before, &after, &ast);
    use LineClass::*;
    assert_eq!(lines.old[..4], [Kept, Kept, Kept, Kept]);
    assert_eq!(lines.new[..5], [Kept, Kept, Unmatched, Kept, Kept]);
    let pairs: Vec<(usize, usize)> = lines.anchors.iter().map(|p| (p.old, p.new)).collect();
    assert_eq!(pairs, [(0, 0), (1, 1), (2, 3), (3, 4)]);
}
