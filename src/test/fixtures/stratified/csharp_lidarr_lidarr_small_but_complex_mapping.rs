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
        "csharp-lidarr-lidarr-small-but-complex-mapping",
        20,
        12,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "csharp-lidarr-lidarr-small-but-complex-mapping",
        1.41,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 16, four times: `FindByProviderId` <-> `DeleteByProviderId` (row 154) and `Delete`
    // -> `DeleteByProviderId` (before row 158, after row 154) are not painted as their differing
    // words (`Find`/`Delete`, `ByProviderId`) under Minimal. Invariant 25, 2 time(s): the deleted
    // `local_declaration_statement` on before row 154 starts delete but its `;` is move (both
    // presets).
    assert_ground_truth_invariants_with_known_violations(
        "csharp-lidarr-lidarr-small-but-complex-mapping",
        6,
    )
}
