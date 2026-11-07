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
        "cpp-libreoffice-interesting-complex-case-with-comments",
        12,
        12,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "cpp-libreoffice-interesting-complex-case-with-comments",
        0.18,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 31, 16 time(s): a changed part not painted whole, or past its
    // separator (awaiting a repaint):
    //   before row 118 leaves 1 of the 2 byte(s) of `46` unpainted, which becomes `45`
    //   after row 119 leaves 1 of the 2 byte(s) of `45` unpainted, which becomes `46`
    //   before row 119 leaves 1 of the 2 byte(s) of `40` unpainted, which becomes `41`
    //   after row 121 leaves 1 of the 2 byte(s) of `41` unpainted, which becomes `40`
    //   before row 120 leaves 1 of the 2 byte(s) of `41` unpainted, which becomes `42`
    //   ... and 11 more
    assert_ground_truth_invariants_with_known_violations(
        "cpp-libreoffice-interesting-complex-case-with-comments",
        16,
    )
}
