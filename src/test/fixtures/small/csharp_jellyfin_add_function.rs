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
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants_with_known_violations;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "csharp-jellyfin-add-function",
        // Includes the N:M floor: 4 all-to-all group members a one-to-one output cannot reach.
        55,
        36,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 26, twice: `var fanoutConcurrency = ...;` on before row 103 is paired with `var
    // operationFanout = ...;` on after row 123, but its `;` with the `;` of the identical statement
    // on after row 109, which the mapping inserts.
    assert_ground_truth_invariants_with_known_violations("csharp-jellyfin-add-function", 2)
}
