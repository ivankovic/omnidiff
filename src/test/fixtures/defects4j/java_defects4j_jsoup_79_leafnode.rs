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
        "java-defects4j-jsoup-79-leafnode",
        4,
        4,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("java-defects4j-jsoup-79-leafnode", 0.96)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 25, 4 time(s): the deleted `throw_statement` on before row 94 starts delete but
    // its `;` is unpainted (both presets); the inserted `return_statement` on after row 96 starts
    // insert but its `;` is unpainted (both presets).
    assert_ground_truth_invariants_with_known_violations("java-defects4j-jsoup-79-leafnode", 4)
}
