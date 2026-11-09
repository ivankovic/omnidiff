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
    // The `color` attribute's value moves into a new `sx={{...}}` object. omnidiff pairs the
    // surrounding attributes by position, so the value reads as changed rather than moved.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "tsx-mui-material-ui-move-colour-to-a-new-attribute",
        16,
        12,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 27, both presets: the attribute's `=` (before row 146) is painted deleted and the
    // object's `:` (after row 147) inserted, where the tree mapping pairs them as one edit.
    assert_ground_truth_invariants_with_known_violations(
        "tsx-mui-material-ui-move-colour-to-a-new-attribute",
        2,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "tsx-mui-material-ui-move-colour-to-a-new-attribute",
        1.1,
    )
}
