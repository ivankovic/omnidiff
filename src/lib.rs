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
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
//! Syntax-aware diffing on tree-sitter syntax trees, for code in 26 languages.
//!
//! [`Diff`] is the entry point: [`Diff::files`] reads two files, [`Diff::new`] takes two strings
//! in a [`Language`]. It tells what changed in each function, type and module, named the way the
//! language's imports name them, and which stretches of text changed:
//!
//! ```
//! use omnidiff::{Diff, Language};
//!
//! let before = "struct Parser;\nimpl Parser {\n    fn parse(&self) {}\n}\n";
//! let after = "struct Parser;\nimpl Parser {\n    fn parse(&self) { todo!() }\n    fn peek(&self) {}\n}\n";
//! let diff = Diff::new(before, after, Language::Rust);
//!
//! for change in diff.changed_functions() {
//!     println!("{} {}", change.kind(), change.qualified_name());
//! }
//! // modified Parser::parse
//! // added Parser::peek
//! # let names: Vec<_> = diff.changed_functions().map(|c| (c.kind().as_str(), c.qualified_name())).collect();
//! # assert_eq!(names, [("modified", "Parser::parse"), ("added", "Parser::peek")]);
//! ```
//!
//! A file without a grammar, or too big to parse, is diffed line by line; it has no
//! definitions, and [`Diff::is_structural`] says so. For a library, depend on the crate with
//! `default-features = false`, which leaves out the terminal viewer. The `git` feature adds
//! `omnidiff::git`: a commit, a range, or the staged or unstaged changes of a repository, each
//! changed file a [`Diff`].
//!
//! # Stability
//!
//! The command line, the `--mode json` output (which the editor integrations consume) and the
//! types at the crate root - [`Diff`] and what it returns - are the supported interfaces. Every
//! other module (`code`, `diff` and its submodules, `tui`) is the binary's internals made
//! visible, and changes between 0.x minor versions without notice.
#[cfg(feature = "stats")]
pub mod anomalous_paths;
pub mod code;
pub mod diff;
mod library;
#[cfg(feature = "stats")]
pub mod stats;
// Needs no TUI dependency itself, but its one consumer (the TUI's review picker) is behind `tui`, so
// it shares that gate rather than widen a `default-features = false` consumer's surface.
#[cfg(feature = "tui")]
pub mod review;
#[cfg(feature = "tui")]
pub mod tui;
// The GitHub Pages showcase's data, for src/bin/generate_showcase.rs alone.
#[cfg(feature = "test-fixtures")]
pub mod showcase;

// Unit tests across the crate use `crate::test::helper` whatever the features, so `cfg(test)` always
// sees it; `test-fixtures` exposes it to the src/bin/ tools that depend on it outside tests.
#[cfg(any(test, feature = "test-fixtures"))]
pub mod test;

pub use crate::code::Language;
pub use crate::library::{
    ChangeKind, Definition, DefinitionChange, DefinitionKind, Diff, Error, Operation, Position,
    Preset, Side, Span, TextChange,
};

/// A git repository's commits, staged and unstaged changes, each file a [`Diff`]. Needs the
/// `git` feature.
#[cfg(feature = "git")]
pub use crate::library::git;

use crate::code::Code;

/// Diffs two programs given as source strings in `language`, returning the engine's node mapping.
#[deprecated(
    since = "0.6.0",
    note = "use `omnidiff::Diff::new`, which keeps the trees the mapping's ids refer to"
)]
pub fn diff_strings(before: &str, after: &str, language: &Language) -> diff::Diff {
    let (code_before, code_after) = code::build_sides(
        code::worth_building_in_parallel(before, after),
        || Code::from_string(before, language),
        || Code::from_string(after, language),
    );

    let mut diff = diff::diff_code(&code_before, &code_after);
    diff.language = *language;
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn diff_empty_strings() {
        let d = diff_strings("", "", &Language::Rust);

        assert!(d.ast.is_some());
    }
}
