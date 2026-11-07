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
        "c-intel-vpl-gpu-rt-whitespace-only-change",
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 29: the files differ only in whitespace (trailing spaces removed on before row 403),
    // and the lone painting is empty, so Full paints nothing; awaiting a `Full` painting.
    assert_ground_truth_invariants_with_known_violations(
        "c-intel-vpl-gpu-rt-whitespace-only-change",
        1,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("c-intel-vpl-gpu-rt-whitespace-only-change", 0.0)
}
