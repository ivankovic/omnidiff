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
        "java-defects4j-jacksondatabind-96-basicdeserializerfactory",
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "java-defects4j-jacksondatabind-96-basicdeserializerfactory",
        0.01,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 16: painting "Minimal" before row 735 paints "findImplicitParamName" against
    // "paramName" wrongly for a Minimal reading - the differing words are "findImplicitP", and
    // everything else in the identifier should be left alone
    // Invariant 16: painting "Minimal" after row 735 paints "paramName" against
    // "findImplicitParamName" wrongly for a Minimal reading - the differing words are "p", and
    // everything else in the identifier should be left alone
    assert_ground_truth_invariants_with_known_violations(
        "java-defects4j-jacksondatabind-96-basicdeserializerfactory",
        2,
    )
}
