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
        "rust-zed-industries-zed-a-lot-of-removed",
        2,
        2,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("rust-zed-industries-zed-a-lot-of-removed", 9.95)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 32, Extended, 2 time(s): before 42-43 -> after 34-35 (2 line(s)) lose 8 byte(s) of
    // indentation, unpainted (want delete from column 0); before 56-57 -> after 39-40 (2 line(s))
    // lose 8 byte(s) of indentation, unpainted (want delete from column 0).
    assert_ground_truth_invariants_with_known_violations(
        "rust-zed-industries-zed-a-lot-of-removed",
        2,
    )
}
