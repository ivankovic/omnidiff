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
    test::helper::human_mapping::assert_matches_human_mapping(
        "java-defects4j-jsoup-77-xmltreebuilder",
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("java-defects4j-jsoup-77-xmltreebuilder", 0.07)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 16: painting "Minimal" before row 117 paints "name" against "normalName" wrongly for
    // a Minimal reading - the differing words are nothing, and everything else in the identifier
    // should be left alone
    // Invariant 16: painting "Minimal" after row 117 paints "normalName" against "name" wrongly for a
    // Minimal reading - the differing words are "ormalN", and everything else in the identifier
    // should be left alone
    assert_ground_truth_invariants_with_known_violations(
        "java-defects4j-jsoup-77-xmltreebuilder",
        2,
    )
}
