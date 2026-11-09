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
    test::helper::human_mapping::assert_matches_human_mapping(
        "css-lassekongo83-zuki-themes-fails-to-parse",
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariants 9 and 30, Full: row 102 paints the space before the inserted `1.64em; // ...` as
    // Move on both sides, where the tree mapping reads it Delete/Insert and the spacing did not
    // change.
    assert_ground_truth_invariants_with_known_violations(
        "css-lassekongo83-zuki-themes-fails-to-parse",
        3,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("css-lassekongo83-zuki-themes-fails-to-parse", 0.36)
}
