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
        "css-microsoft-vscode-add-with-whitespace-change",
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "css-microsoft-vscode-add-with-whitespace-change",
        0.01,
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 6, once: Minimal paints the leading tab of after row 52
    // (`\t.monaco-list-row.selected .agent-session-item,`).
    assert_ground_truth_invariants_with_known_violations(
        "css-microsoft-vscode-add-with-whitespace-change",
        1,
    )
}
