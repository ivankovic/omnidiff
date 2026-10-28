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
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    // Pins `SHELL_STRING_BODY_KINDS`: a `string_content` rewritten as a `regex` renames into it.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "shellscript-scikit-learn-scikit-learn-string-to-regex",
        1,
        1,
    )
}

#[test]
fn invariants() -> Result<()> {
    assert_ground_truth_invariants("shellscript-scikit-learn-scikit-learn-string-to-regex")
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "shellscript-scikit-learn-scikit-learn-string-to-regex",
        0.18,
    )
}
