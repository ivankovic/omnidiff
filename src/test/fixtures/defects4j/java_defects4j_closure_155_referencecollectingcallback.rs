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
        "java-defects4j-closure-155-referencecollectingcallback",
        15,
        11,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "java-defects4j-closure-155-referencecollectingcallback",
        0.23,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 8: 6 byte(s) survive under one preset and do not under the other, on after rows 125,
    // 127 - the first reads Insert under 'Minimal' and Move under 'Full'. A Move says the code is
    // still there, so the other preset cannot call it removed or added
    // Invariant 22: painting 'Minimal' paints 2 leaf/leaves wholly deleted or inserted that the tree
    // mapping pairs with an identical leaf, on after rows 125, 127 - the first is `v`
    // Invariant 25: painting 'Full' after row 125 paints the removed `expression_statement` move at
    // its start but its `;` insert - the `;` belongs to its statement
    // Invariant 25: painting 'Full' after row 127 paints the removed `expression_statement` move at
    // its start but its `;` insert - the `;` belongs to its statement
    assert_ground_truth_invariants_with_known_violations(
        "java-defects4j-closure-155-referencecollectingcallback",
        4,
    )
}
