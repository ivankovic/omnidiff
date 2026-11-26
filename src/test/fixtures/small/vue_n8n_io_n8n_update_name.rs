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
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
//! Painted by hand against the plain-text fallback, before `.vue` had a grammar: a text-level
//! painting, graded against what the tree diff renders now that both sides parse. The 0.21
//! limit (measured 0.203%, 44 of 21,650 bytes) is the reading the painting cannot accept: the
//! two `SETUP` -> `PLAN` renames. The raw-text one narrows to the word under both presets, but
//! the attribute value on row 169 is an identifier-shaped leaf, so `EXTENDED` paints it whole
//! (`whole_identifier_updates`, ground-truth invariant 16) while the lone painting says only
//! the word changed.

use crate::test::helper::human_mapping::assert_matches_human_painting_within_limit;
use crate::test::helper::human_mapping::invariants::assert_ground_truth_invariants;
use anyhow::Result;

#[test]
fn painting() -> Result<()> {
    assert_matches_human_painting_within_limit("vue-n8n-io-n8n-update-name", 0.21)
}

#[test]
fn invariants() -> Result<()> {
    assert_ground_truth_invariants("vue-n8n-io-n8n-update-name")
}
