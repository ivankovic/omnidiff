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

//! The YAML grammar, compiled by `build.rs` from `vendor/tree-sitter-yaml`; why it is vendored is
//! in `build.rs`.

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
    fn tree_sitter_yaml() -> *const ();
}

/// The tree-sitter language for YAML.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_yaml) };
