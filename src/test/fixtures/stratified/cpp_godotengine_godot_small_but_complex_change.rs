/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */
use anyhow::Result;

use crate::test;
use crate::test::helper::human_mapping::assert_matches_human_painting_within_limit;
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants_with_known_violations;

#[test]
fn mapping() -> Result<()> {
    // Two rewrites the human reads as the same code and omnidiff does not. `sub_viewport->set_size(
    // Node3DEditor::get_camera_viewport_size(camera))` becomes the declaration `const Size2i
    // camera_size = Node3DEditor::get_camera_viewport_size(camera)`: the human keeps the inner call,
    // omnidiff deletes and re-inserts it (`APTED("qualified_name")`). And the constructor's member
    // initializers become assignments in its body, which the human pairs and omnidiff calls
    // deleted and inserted.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "cpp-godotengine-godot-small-but-complex-change",
        24,
        17,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "cpp-godotengine-godot-small-but-complex-change",
        3.47,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 16, twice: `set_size` <-> `set_ratio` (before row 83, after row 87) is not painted
    // as its differing words (`size`, `ratio`) under Minimal. Invariant 17: `false` on before row
    // 87 is removed and `true` on after row 109 added inside the subtree the mapping pairs, where
    // one Update entry would say the literal flipped.
    assert_ground_truth_invariants_with_known_violations(
        "cpp-godotengine-godot-small-but-complex-change",
        3,
    )
}
