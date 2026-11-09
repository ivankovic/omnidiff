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
    // Another interesting case of code being quoted in string so code nodes need to match
    // string node.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "python-pandas-dev-pandas-interesting-case-code-becomes-string",
        4,
        4,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "python-pandas-dev-pandas-interesting-case-code-becomes-string",
        0.01,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 12: the Minimal painting of row 5229 (`np` -> `np.ndarray`, and two more edited
    // leaves) paints nothing on either side although the tree mapping says the text changed.
    assert_ground_truth_invariants_with_known_violations(
        "python-pandas-dev-pandas-interesting-case-code-becomes-string",
        1,
    )
}
