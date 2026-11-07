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
    test::helper::human_mapping::assert_matches_human_mapping("json-twbs-bootstrap-version")
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("json-twbs-bootstrap-version", 0.02)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 31, 4 time(s): a changed part not painted whole, or past its
    // separator (awaiting a repaint):
    //   before row 148 leaves 1 of the 2 byte(s) of `20` unpainted, which becomes `21`
    //   before row 148 paints the `.` between the changed `20` and `0` move (it goes with the right-
    //     hand part: update)
    //   after row 148 leaves 1 of the 2 byte(s) of `21` unpainted, which becomes `20`
    //   after row 148 paints the `.` between the changed `21` and `1` move (it goes with the right-
    //     hand part: update)
    assert_ground_truth_invariants_with_known_violations("json-twbs-bootstrap-version", 4)
}
