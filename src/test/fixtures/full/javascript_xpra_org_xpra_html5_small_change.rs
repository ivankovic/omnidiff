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
    // Residual not yet examined.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "javascript-xpra-org-xpra-html5-small-change",
        3,
        3,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 28, once: the `member_expression` on before row 3380 is deleted, but it is made of
    // the children of the `member_expression` on after row 3380, which is paired with the code
    // around this one instead: the two should pair, and the code around be removed.
    assert_ground_truth_invariants_with_known_violations(
        "javascript-xpra-org-xpra-html5-small-change",
        1,
    )
}
