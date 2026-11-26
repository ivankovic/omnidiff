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

//! Compiles the vendored YAML grammar, `vendor/tree-sitter-yaml`: tree-sitter-yaml 0.7.2
//! (upstream commit 7708026) with `vendor/tree-sitter-yaml/omnidiff.patch` applied to its scanner.
//! The patch carries two upstream fixes no release has yet:
//!
//! - rows and columns in `int32_t`, not `int16_t` (upstream PR #35, issue #49): past row 32,767
//!   the scanner lost its place and the tree ended there, so OmniDiff reported no change in the
//!   rest of the file (OmniDiff issue #114);
//! - `serialize` checks for room for a whole indentation-stack entry (upstream PR #47, issue #48):
//!   about 250 nested block mappings wrote past tree-sitter's 1,024-byte buffer, which aborts the
//!   process on tree-sitter's own assertion.
//!
//! The crate is not patched with `[patch.crates-io]` because a published crate does not carry its
//! patches: `cargo install omnidiff` would get the unfixed grammar. Drop this file and the
//! `vendor/` directory once a tree-sitter-yaml release has both fixes.

fn main() {
    let src = std::path::Path::new("vendor/tree-sitter-yaml/src");
    let mut build = cc::Build::new();
    build
        .std("c11")
        .include(src)
        .file(src.join("parser.c"))
        .file(src.join("scanner.c"))
        // The grammar's own build.rs silences the same three.
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-unused-value")
        .flag_if_supported("-Wno-implicit-fallthrough");
    #[cfg(target_env = "msvc")]
    build.flag("-utf-8");
    build.compile("tree-sitter-yaml");
    println!("cargo:rerun-if-changed=vendor/tree-sitter-yaml/src");
}
