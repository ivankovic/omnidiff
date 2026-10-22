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
    // Two literals become one named constant: an all-to-all 2:1 group (see
    // `MultiMapGroup::pairing`). One before literal is unavoidably unmatched; omnidiff's
    // `qualified_name` pass also deletes the other and inserts the after one, an ordinary gap.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "java-refactor-constants",
        3,
        3,
    )
}

#[test]
fn painting() -> Result<()> {
    // `FULL` sets the limit.
    assert_matches_human_painting_within_limit("java-refactor-constants", 12.8)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 25, 2 time(s): the inserted `field_declaration` on after row 2 starts insert but
    // its `;` is unpainted (both presets).
    assert_ground_truth_invariants_with_known_violations("java-refactor-constants", 2)
}
