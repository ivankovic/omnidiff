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
    // Wrap/reparent: `class_specifier` gains a `template_declaration` parent. Pins
    // `resolve_residual_forest_via_myers_lcs`'s trivial-entry filtering (`TRIVIAL_ENTRY_MAX_SIZE`)
    // and `rescue_wrapped_trivial_entries`, which re-points the wrapped `;`'s insert to its
    // before-side twin. Keep that part exact. The three mismatches left are `int` becoming `T` in
    // `field_declaration.type`, `parameter_declaration.type` and `function_definition.type`: the
    // human mapping updates each, omnidiff deletes `int` and inserts `T`.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "cpp-add-templates",
        3,
        3,
    )
}

#[test]
fn painting() -> Result<()> {
    // `MINIMAL` sets the limit.
    assert_matches_human_painting_within_limit("cpp-add-templates", 29.21)
}

#[test]
fn invariants() -> Result<()> {
    // None: the human mapping updates `int` to `T` (invariant 18's reading); omnidiff's delete and
    // insert of it are counted as mapping mismatches above.
    assert_ground_truth_invariants_with_known_violations("cpp-add-templates", 0)
}
