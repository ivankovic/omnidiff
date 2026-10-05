/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
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
        "csharp-radarr-radarr-nice-move-into-added-block",
        7,
        5,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "csharp-radarr-radarr-nice-move-into-added-block",
        12.73,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 16, three times: `RadarrAuth` <-> `Auth` (before row 38, after row 41) is not
    // painted as its differing word `Radarr` under Minimal, nor entire under Full. Invariant 5:
    // Full leaves columns 63..64 of before row 36 unpainted between two painted regions.
    assert_ground_truth_invariants_with_known_violations(
        "csharp-radarr-radarr-nice-move-into-added-block",
        4,
    )
}
