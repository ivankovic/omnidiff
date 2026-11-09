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
use crate::test;
use crate::test::helper::human_mapping::assert_matches_human_painting_within_limit;
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants_with_known_violations;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    // The residual is the nested `if` this commit did not touch - the container choice, not the
    // delimiters.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "cpp-godot-small-bugfix",
        13,
        7,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 28, once: the `binary_expression` on before row 120 is deleted, but it is made of
    // the children of the `binary_expression` on after row 120, which is paired with the code
    // around this one instead: the two should pair, and the code around be removed.
    assert_ground_truth_invariants_with_known_violations("cpp-godot-small-bugfix", 1)
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("cpp-godot-small-bugfix", 0.56)
}
