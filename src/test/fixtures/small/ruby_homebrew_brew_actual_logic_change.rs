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
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    // An `if` modifier moves to a later assignment; omnidiff deletes and inserts most of it, and
    // does not realize two multi-map group pairings.
    // Re-measured against the repainted ground truth (219793df), which deletes the old `if`
    // modifier and inserts the new one where omnidiff keeps it Identical, and pairs the two
    // `chained_string`s omnidiff deletes and inserts: 13/6 -> 21/12.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "ruby-homebrew-brew-actual-logic-change",
        21,
        12,
    )
}

#[test]
fn invariants() -> Result<()> {
    assert_ground_truth_invariants("ruby-homebrew-brew-actual-logic-change")
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("ruby-homebrew-brew-actual-logic-change", 0.36)
}
