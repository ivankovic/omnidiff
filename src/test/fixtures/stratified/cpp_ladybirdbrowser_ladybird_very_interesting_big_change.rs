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
    // Recorded as found, not examined. 19 of these pair `update_layout`'s old condition with
    // the new `layout_is_up_to_date()` it was extracted into, across two functions.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "cpp-ladybirdbrowser-ladybird-very-interesting-big-change",
        48,
        33,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "cpp-ladybirdbrowser-ladybird-very-interesting-big-change",
        0.4,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariants 12 and 16: the `layout_node` -> `unsafe_layout_node` rename (before row 1433,
    // after row 1438) is painted under neither preset (12, twice), not as `unsafe_` under Minimal
    // and not whole under Full (16, three times). Invariant 16, twice more: the
    // `svg_roots_to_relayout` <-> `m_svg_roots_needing_relayout` rename (before row 1470, after
    // row 1637) is not painted as just its differing words under Minimal. Invariant 24, 2 time(s):
    // the `.` before `layout_node` on before row 1653 is not painted with the member it calls
    // (both presets).
    assert_ground_truth_invariants_with_known_violations(
        "cpp-ladybirdbrowser-ladybird-very-interesting-big-change",
        9,
    )
}
