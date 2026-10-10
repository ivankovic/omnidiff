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
    test::helper::human_mapping::assert_matches_human_mapping(
        "cpp-microsoft-terminal-add-tests-cases",
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("cpp-microsoft-terminal-add-tests-cases", 0.26)
}

#[test]
fn invariants() -> Result<()> {
    // Recorded as found, not examined: around `TEST_METHOD` on after rows 46 and 50, invariants
    // 11 and 22 in both paintings, and invariant 16 in both for `DontRoundtripNoReloadEnvVars`
    // paired with `RoundtripUserDeletedColorSchemeCollision` on before row 46.
    assert_ground_truth_invariants_with_known_violations(
        "cpp-microsoft-terminal-add-tests-cases",
        6,
    )
}
