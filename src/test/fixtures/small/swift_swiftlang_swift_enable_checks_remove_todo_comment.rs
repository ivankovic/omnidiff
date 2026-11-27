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
    test::helper::human_mapping::assert_matches_human_mapping(
        "swift-swiftlang-swift-enable-checks-remove-todo-comment",
    )
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 12, both paintings: the edited `// CHECK2: sil private ...` comment (before row 73,
    // after row 66) is unpainted. Invariant 31, Extended: before rows 37 and 65 (after 30 and 58)
    // paint the unchanged `$` beside the changed mangled name.
    assert_ground_truth_invariants_with_known_violations(
        "swift-swiftlang-swift-enable-checks-remove-todo-comment",
        6,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "swift-swiftlang-swift-enable-checks-remove-todo-comment",
        4.89,
    )
}
