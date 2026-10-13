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
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants;
use anyhow::Result;

#[test]
fn mapping() -> Result<()> {
    // Mostly `StructurallyIdenticalAncestor`: member-table rows and tag scaffolding the human
    // inserts or deletes inherit a partner from a same-shaped element nearby (the `pub-methods`
    // heading anchor takes the new search-results `<div>`, and the rows after it shift). The rest
    // is `APTED("fast_fallback")`. The licence block between @license tags parses as one `raw_text`
    // node, so the tree says much less than the text.
    test::helper::human_mapping::assert_matches_human_mapping_within_limit(
        "html-berndporr-iir1-a-lot-of-new-functionality",
        261,
        177,
    )
}

#[test]
fn invariants() -> Result<()> {
    assert_ground_truth_invariants("html-berndporr-iir1-a-lot-of-new-functionality")
}
