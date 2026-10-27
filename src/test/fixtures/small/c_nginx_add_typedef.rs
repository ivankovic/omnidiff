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
    // Two deliberate human choices:
    // 1. `pwd = passwords->elts` -> `cb_data.pwd = (*passwords)->elts` should match: a small
    //    textual edit whose AST change is large (the dereference and parentheses), subjectively
    //    optimal rather than edit-distance optimal.
    // 2. `pwd++` -> `cb_data.pwd++` maps `identifier` "pwd" to `field_identifier` "pwd": matching
    //    by kind alone would update `pwd` into `cb_data` and insert a new "pwd", which a human would
    //    not. So cross-kind matches must be allowed.
    // Partly resolved by `prematch_identical_statement_siblings` and `ContainmentCtx`'s
    // sibling-order-consistency check.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "c-nginx-add-typedef",
        46,
        28,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 9, both presets and both sides: 14 bytes on before rows 479 and 574 and after rows
    // 490 and 591 are painted Move where the tree mapping deletes and inserts them. Invariant 11,
    // both presets: the first `identifier` `pwd` on after row 548 is removed but left unpainted.
    // Invariant 25, 8 time(s): the deleted `declaration` on before row 479 starts delete but its
    // `;` is move (both presets); the inserted `declaration` on after row 490 starts insert but
    // its `;` is move (both presets); the inserted `declaration` on after row 591 starts insert
    // but its `;` is unpainted (both presets); the inserted `expression_statement` on after row
    // 548 starts unpainted but its `;` is insert (both presets).
    assert_ground_truth_invariants_with_known_violations("c-nginx-add-typedef", 14)
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("c-nginx-add-typedef", 0.65)
}
