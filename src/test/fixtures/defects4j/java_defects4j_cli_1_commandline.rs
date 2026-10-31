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
    // The human deletes whole imports and a method and inserts replacements.
    // `APTED("fast_fallback")` pairs the deleted import lines with the new ones, and
    // `qualified_name` reuses the deleted method's leaves inside the inserted one. One container
    // choice each, counted per leaf.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "java-defects4j-cli-1-commandline",
        77,
        53,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("java-defects4j-cli-1-commandline", 1.34)
}

#[test]
fn invariants() -> Result<()> {
    // All four paintings paint 3 bytes Move on before row 93 and after row 91 that the tree mapping
    // reads Delete/Insert. The two ground truths must be reconciled by hand.
    assert_ground_truth_invariants_with_known_violations("java-defects4j-cli-1-commandline", 8)
}
