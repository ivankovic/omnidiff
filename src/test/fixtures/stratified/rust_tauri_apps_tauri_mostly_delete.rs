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
    test::helper::human_mapping::assert_matches_human_mapping("rust-tauri-apps-tauri-mostly-delete")
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("rust-tauri-apps-tauri-mostly-delete", 0.16)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 30, 3 time(s): the space before an edit (awaiting a repaint):
    //   before row 25: the space before the delete `InvokeResponder,` is not painted with it
    //   before row 257: the indentation before `(invoke_responder, invoke_i...` is painted, though
    //     it did not change
    //   before row 734: the indentation before `(None, "".into())` is painted, though it did not
    //     change
    assert_ground_truth_invariants_with_known_violations("rust-tauri-apps-tauri-mostly-delete", 3)
}
