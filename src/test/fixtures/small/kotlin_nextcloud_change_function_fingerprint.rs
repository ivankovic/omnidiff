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
    // Pins `apted::prematch_unique_named_locals` for parameters shifted by an inserted one. The
    // rest: `viewModel` (inserted) and `showTranslateScreen` (deleted) are cross-matched by APTED's
    // same-kind cost preference despite sharing no name - the near-duplicate reuse-vs-replace gap
    // (see `kotlin-remove-function`); plus the two `,` separators and one lambda `identifier`.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "kotlin-nextcloud-change-function-fingerprint",
        4,
        4,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 30, 1 time(s): the space before an edit (awaiting a repaint):
    //   after row 55: the space before the insert `viewModel: AssistantViewModel,` is not painted
    //     with it
    assert_ground_truth_invariants_with_known_violations(
        "kotlin-nextcloud-change-function-fingerprint",
        1,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("kotlin-nextcloud-change-function-fingerprint", 1.25)
}
