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
    test::helper::human_mapping::assert_matches_human_mapping("yaml-gyulyvgc-sniffnet-version")
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("yaml-gyulyvgc-sniffnet-version", 6.75)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 31, 2 time(s): a changed part not painted whole, or past its
    // separator (awaiting a repaint):
    //   before row 5 paints the unchanged `.` beside the changed `8` update
    //   after row 5 paints the unchanged `.` beside the changed `8t64` update
    assert_ground_truth_invariants_with_known_violations("yaml-gyulyvgc-sniffnet-version", 2)
}
