/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
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
    // Mostly listpackTest's new "Benchmark lpCompare with number and caching" block, a copy of the
    // benchmark before it: the human calls the copy inserted and keeps the old block with its edited
    // original, while omnidiff pairs some of the old block's statements with the copy's identical
    // ones (`IdenticalHashOfAncestor`, `APTED("large_flat_subtree")`). The rest is lpCompare, whose
    // old `lpStringToInt64` branch the human maps to both its cached and uncached copies.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "c-redis-redis-good-multi-map-test-case",
        330,
        219,
    )
}

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("c-redis-redis-good-multi-map-test-case", 0.49)
}

#[test]
fn invariants() -> Result<()> {
    // Invariant 12, both presets, and 16, four times: `lp` -> `eptr` (before row 3267, after row
    // 3281) is painted on neither side, though the mapping says the leaf changed. Invariant 5:
    // Full leaves columns 84..85 of after row 1738 unpainted between two painted regions.
    assert_ground_truth_invariants_with_known_violations(
        "c-redis-redis-good-multi-map-test-case",
        7,
    )
}
