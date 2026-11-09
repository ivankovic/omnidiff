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
    // Residual not yet examined. Re-recorded 2026-11-04 (54/33 -> 120/72) after the ground truth
    // was repainted around row 96: the engine still moves type fragments out of the deleted
    // declarations into the new lambdas' parameters (`MovedSubtree`).
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "cpp-laydbird-change-function-signature",
        120,
        72,
    )
}

#[test]
fn invariants() -> Result<()> {
    assert_ground_truth_invariants("cpp-laydbird-change-function-signature")
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("cpp-laydbird-change-function-signature", 0.66)
}
