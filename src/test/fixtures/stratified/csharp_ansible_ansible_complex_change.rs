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
        "csharp-ansible-ansible-complex-change",
        21,
        15,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("csharp-ansible-ansible-complex-change", 0.27)
}

#[test]
fn invariants() -> Result<()> {
    // Invariants 12 and 16: the `unsupportedParameters` -> `unsupportedSorted` and `legalInputs`
    // -> `legalInputsSorted` renames (before rows 1077-1078, after rows 1128-1129) are painted
    // under neither preset (12, twice), not as just their differing words under Minimal and not
    // whole under Full (16, seven times). Invariant 24, 6 time(s): the `.` before
    // `CreateDirectory` on after row 191 is not painted with the member it calls (both presets);
    // the `.` before `SetAccessControl` on before row 167 is not painted with the member it calls
    // (both presets); the `.` before `SetAccessControl` on before row 198 is not painted with the
    // member it calls (both presets).
    assert_ground_truth_invariants_with_known_violations(
        "csharp-ansible-ansible-complex-change",
        15,
    )
}
