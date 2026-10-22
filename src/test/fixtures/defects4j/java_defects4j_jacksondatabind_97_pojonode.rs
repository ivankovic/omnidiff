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
        "java-defects4j-jacksondatabind-97-pojonode",
        8,
        6,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("java-defects4j-jacksondatabind-97-pojonode", 0.51)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 3: mapping before marks "(" on row 114 as deleted but its matching ")" on row 114 as
    // matched
    // Invariant 3: mapping after marks "(" on row 114 as inserted but its matching ")" on row 114 as
    // matched
    // Invariant 10: painting 'Minimal' paints 1 leaf pair(s) gone on one side and new on the other
    // that the tree mapping calls the same text, on before row 114 and after row 114 - the first is
    // `)`
    // Invariant 10: painting 'Full' paints 1 leaf pair(s) gone on one side and new on the other that
    // the tree mapping calls the same text, on before row 114 and after row 114 - the first is `)`
    assert_ground_truth_invariants_with_known_violations(
        "java-defects4j-jacksondatabind-97-pojonode",
        4,
    )
}
