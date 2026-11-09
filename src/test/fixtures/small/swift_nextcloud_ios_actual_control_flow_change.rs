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
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    // One `directive` inside a lambda is `MatchButNotIdentical` instead of `Identical`: isolated.
    // Since 11b65fc9 the human also takes the second `directive` as new and pairs its `#endif`
    // elsewhere; omnidiff keeps the directive.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "swift-nextcloud-ios-actual-control-flow-change",
        3,
        1,
    )
}

#[test]
fn invariants() -> Result<()> {
    assert_ground_truth_invariants("swift-nextcloud-ios-actual-control-flow-change")
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit(
        "swift-nextcloud-ios-actual-control-flow-change",
        0.1,
    )
}
