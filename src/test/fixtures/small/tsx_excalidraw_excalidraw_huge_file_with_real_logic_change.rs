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
    // Pointer-event bookkeeping is renamed and reshaped. All of it is
    // `APTED("large_flat_subtree")`, pairing the pointer fields differently from the human
    // (`missingPointerEventCleanupEmitter` with `lastPointerUp`, `pointerStart` with
    // `lastPointerDownEvent`) and deleting statements the human keeps.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "tsx-excalidraw-excalidraw-huge-file-with-real-logic-change",
        // Includes the N:M floor: 13 all-to-all group members a one-to-one output cannot reach.
        69,
        57,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 28, twice:
    // - the `member_expression` on after row 7758 is inserted, but it is made of the children of
    //   the `member_expression` on before row 7700, which is in a multi-map group this one is left
    //   out of.
    // - the `member_expression` on after row 7757 is inserted, but it is made of the children of
    //   the `member_expression` on before row 7698, which is in a multi-map group this one is left
    //   out of.
    assert_ground_truth_invariants_with_known_violations(
        "tsx-excalidraw-excalidraw-huge-file-with-real-logic-change",
        2,
    )
}
