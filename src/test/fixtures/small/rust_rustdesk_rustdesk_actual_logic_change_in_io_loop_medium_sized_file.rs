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
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    // The four leaves of a `.is_file_transfer()` call form a multi-map group with one after node,
    // of which omnidiff matches none; and `APTED("qualified_name")` deletes an `if` keyword the
    // human keeps.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "rust-rustdesk-rustdesk-actual-logic-change-in-io-loop-medium-sized-file",
        10,
        5,
    )
}

#[test]
fn invariants() -> Result<()> {
    assert_ground_truth_invariants(
        "rust-rustdesk-rustdesk-actual-logic-change-in-io-loop-medium-sized-file",
    )
}
