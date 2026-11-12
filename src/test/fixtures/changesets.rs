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

//! One file per **solved** changeset (`test::helper::human_changeset`), named for it with `-` as
//! `_`, whatever its dataset: a changeset's name is unique across them.
//!
//! There is no multi-file engine to grade yet, so a stub holds one test, `ground_truth()`: the
//! fixture loads, its `source.json` fits its trees, and its truth is whole and passes every check.
//! A changeset nobody has solved has no stub
//! (`solved_changesets_have_a_stub_and_only_they_do`). Clamps and their explanations will live
//! here once there is something to clamp, for the reasons `test::fixtures` gives.

#[cfg(test)]
mod rust_function_copied;
#[cfg(test)]
mod rust_function_moved_intact;
