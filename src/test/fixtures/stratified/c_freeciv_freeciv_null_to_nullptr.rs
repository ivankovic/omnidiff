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
use anyhow::Result;

use crate::test;
use crate::test::helper::human_mapping::assert_matches_human_painting_within_limit;
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants_with_known_violations;

#[test]
fn mapping() -> Result<()> {
    // Recorded as found, not examined.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "c-freeciv-freeciv-null-to-nullptr",
        3,
        3,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("c-freeciv-freeciv-null-to-nullptr", 0.24)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 31, 4 time(s): a changed part not painted whole, or past its
    // separator (awaiting a repaint):
    //   before row 349 leaves 9 of the 10 byte(s) of `initialise` unpainted, which becomes
    //     `Initialise`
    //   after row 349 leaves 9 of the 10 byte(s) of `Initialise` unpainted, which becomes
    //     `initialise`
    //   before row 357 leaves 3 of the 4 byte(s) of `test` unpainted, which becomes `Test`
    //   after row 357 leaves 3 of the 4 byte(s) of `Test` unpainted, which becomes `test`
    assert_ground_truth_invariants_with_known_violations("c-freeciv-freeciv-null-to-nullptr", 4)
}
