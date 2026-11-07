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
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants_with_known_violations;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    // Residual not yet root-caused.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "javascript-home-sweet-gnome-dash-to-panel-change-set-style-arguments",
        52,
        33,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 28, once: the `binary_expression` on after row 1122 is inserted, but it is made of
    // the children of the `binary_expression` on before row 1117, which is paired with the code
    // around this one instead: the two should pair, and the code around be new.
    assert_ground_truth_invariants_with_known_violations(
        "javascript-home-sweet-gnome-dash-to-panel-change-set-style-arguments",
        1,
    )
}
