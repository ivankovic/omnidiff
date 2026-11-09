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
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "cpp-ladybird-refactor-variables-if-changes",
        141,
        87,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 25, twice: the inserted `expression_statement` on after row 211 starts move but
    // its `;` is insert (both presets).
    // Invariant 28, twice:
    // - the `binary_expression` on after row 574 is inserted, but it is made of the children of the
    //   `binary_expression` on before row 548, which is paired with the code around this one
    //   instead: the two should pair, and the code around be new.
    // - the `binary_expression` on after row 462 is inserted, but it is made of the children of the
    //   `binary_expression` on before row 458, which is paired with the code around this one
    //   instead: the two should pair, and the code around be new.
    // Invariant 30, 2 time(s): the space before an edit (awaiting a repaint):
    //   after row 462: the space before the insert `|| is<SVGPatternBox>(*ances...` is not painted
    //     with it
    //   after row 574: the space before the insert `|| is<SVGPatternBox>(child)` is not painted with
    //     it
    assert_ground_truth_invariants_with_known_violations(
        "cpp-ladybird-refactor-variables-if-changes",
        6,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("cpp-ladybird-refactor-variables-if-changes", 1.86)
}
