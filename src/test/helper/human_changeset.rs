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

//! Human-authored ground truth for a **changeset**: a whole commit, or two directories, where
//! code may move between files (issue #63). The single-file truth is `human_mapping`; this module
//! is the specification of the multi-file format, its loader, and the checks of a truth against
//! itself. Nothing here runs a diff: there is no multi-file engine to grade yet.
//!
//! # A fixture directory
//!
//! ```text
//! src/test/data/changesets/<dataset>/<name>/
//!   before/<repo path>.test    every file of the before side, at its real path, `.test` appended
//!   after/<repo path>.test     likewise for the after side
//!   source.json                provenance and what the VCS would say (ChangesetSource)
//!   human_changeset.json       the truth (HumanChangeset); absent until someone solves it
//!   README.md                  provenance and licence, for a human
//! ```
//!
//! Datasets are subdirectories, as under `diffs/` ([`CHANGESET_DATASETS`]), and a name is unique
//! across them. A file's language comes from its real path (`before/src/a.rs.test` is Rust). A
//! text file in no language with a grammar (a `.md`, a `.snap`) is text without a tree, and a
//! file that is not text (a PNG) is carried as bytes and has neither. A side with no files has no
//! directory, because git does not keep empty ones.
//!
//! A changeset is of one of two kinds ([`ChangesetKind`]), and its limits go by kind:
//!
//! * a **commit** holds every file one commit changes and the few untouched files a signal
//!   points at (the source of a copy): at most [`MAX_CHANGED_FILES`] changed and
//!   [`MAX_COMMIT_UNTOUCHED_FILES`] untouched;
//! * **folders** are two directories as they lie on disk, complete, the files that did not
//!   change included, with no VCS and so no hints: at most [`MAX_CHANGED_FILES`] changed, and
//!   [`MAX_TREE_FILES`] files a side.
//!
//! A sample (`src/test/data/samples/changesets/<name>/`, git-ignored, written by
//! `materialize_test_diffs --changesets`) is the same directory before anyone solved it;
//! `human_solver` moves it under its dataset here on the save that finds it solved and valid.
//!
//! **Paths in the JSON** are relative to the side's root, `/`-separated, without the `.test`:
//! `src/a.rs`. [`check_repo_path`] rejects an absolute path, a `..` or `.` component, a backslash
//! and an empty component, and the loader rejects a symbolic link and any file under `before/` or
//! `after/` that does not end in `.test`, so nothing a fixture names can leave its directory.
//!
//! # `source.json`
//!
//! One type, [`ChangesetSource`], is the file: the materializer writes it, `human_solver` reads
//! it and rewrites its `status`, and an unknown key is an error. A commit:
//!
//! ```json
//! { "kind": "commit", "repository": "caddy", "commit": "5f44…", "parent": "c8e4…",
//!   "signals": ["block-move", "rename-edited"], "dataset": "changeset-pilot", "status": "SAMPLED",
//!   "files": [ { "before": "src/a.rs", "after": "src/a.rs" },
//!              { "before": "src/u.rs", "after": "src/u.rs", "untouched": true },
//!              { "before": "src/b.rs" }, { "after": "lib/b.rs" } ],
//!   "vcs_hints": [ { "status": "M", "path": "src/a.rs" },
//!                  { "status": "R", "path": "lib/b.rs", "old_path": "src/b.rs", "similarity": 92 } ] }
//! ```
//!
//! and folders:
//!
//! ```json
//! { "kind": "folders", "repository": "scrcpy", "commit": "7404…", "parent": "f91f…",
//!   "root": "app/data", "signals": ["control"], "dataset": "changeset-pilot", "status": "SAMPLED",
//!   "files": [ { "before": "icon.png" }, { "after": "scrcpy.png" },
//!              { "before": "open.bat", "after": "open.bat", "untouched": true } ],
//!   "omitted": [ { "side": "before", "path": "data", "reason": "symlink", "bytes": 7,
//!                  "id": "a1b2…" } ] }
//! ```
//!
//! * `kind` - `commit` or `folders`.
//! * `repository`, `commit`, `parent` - where it was sampled from: `after/` is the tree of
//!   `commit` and `before/` that of `parent`, which for folders is any earlier point in history.
//!   All three, or none for a changeset written by hand (the `handmade` dataset).
//! * `root` - folders only: the sub-folder of the repository the two trees are, `.` for its
//!   root. Every path of the fixture is relative to it.
//! * `signals` - every phenomenon the changeset was drawn or written for, of [`SIGNALS`]
//!   (defined in `stats::changeset`), sorted by name; none ranks above another, and a directory
//!   renamed with one file edited is `directory-move`, `rename-edited` and `rename-unedited` at
//!   once. `["control"]` when there is none. A signal says why the changeset is here; it is not
//!   truth, and a person solving it may find another.
//! * `dataset` - which is also the changeset's parent directory once it is promoted.
//! * `status` - `human_solver`'s triage state: `SAMPLED`, `PROMOTED` or `REJECTED`.
//! * `files` - every path in the fixture once, in path order: a path both sides have carries
//!   both keys, and they are equal - which path became which is truth, and `files` is not.
//!   `untouched` marks a file that is byte-identical at one path on both sides, and every such
//!   file is marked. In a commit it is there because a signal points at it; in folders it is
//!   every file that did not change. This is the one place that says which files are untouched
//!   (`changesets.csv` has an `untouched_paths` column only so that the materializer knows which
//!   files to fetch for a commit).
//! * `omitted` - folders only: what a directory held and its tree here does not, by `side`,
//!   `path`, `reason` (`size`: an unchanged file over the sampler's cap; `symlink`;
//!   `submodule`), `bytes` (not for a submodule) and git `id`. The trees are complete except
//!   exactly there, and none of these paths is a file of that side.
//! * `vcs_hints` - commits only, and always there for one: what `git diff --name-status -M`
//!   reports for the two trees about the files the fixture holds. **Hints are data, never
//!   truth**: they live here and nowhere in `human_changeset.json`, and no check of the truth
//!   reads them. Folders have no such key, because two directories have no VCS to ask.
//!
//! [`source_violations`] holds `source.json` to the trees and to its kind: the limits above; for
//! a commit, that the hints name exactly the changed paths; for folders, that there are no
//! hints, and that nothing `omitted` is in the trees. The handmade set are commits without a
//! repository, and their hints are generated, not written: the ignored test
//! `write_handmade_sources` builds a temporary git repository from the two trees, and
//! `handmade_hints_are_what_git_reports` holds them to it.
//!
//! # `human_changeset.json`
//!
//! ```json
//! { "files": [ { "before": "src/a.rs", "after": "src/a.rs", "relation": "same", "solved": true },
//!              { "before": "img/l.png", "after": "art/l.png", "relation": "renamed",
//!                "solved": true, "verdict": "invisible" } ],
//!   "entries": [
//!     { "operation": "match_but_not_identical",
//!       "before": { "file": "src/a.rs", "path": [] }, "after": { "file": "src/a.rs", "path": [] } },
//!     { "operation": "identical_with_children",
//!       "before": { "file": "src/a.rs", "path": ["function_item:2"] },
//!       "after": { "file": "src/b.rs", "path": ["function_item:1"] } },
//!     { "operation": "delete", "before": { "file": "src/a.rs", "path": ["line_comment:1"] } } ],
//!   "text_mappings": [ { "name": "Full", "entries": [
//!     { "operation": "Match",
//!       "before": [ { "file": "src/a.rs",
//!                     "range": { "start_row": 4, "start_column": 0, "end_row": 7, "end_column": 1 } } ],
//!       "after": [ { "file": "src/b.rs",
//!                    "range": { "start_row": 9, "start_column": 0, "end_row": 12, "end_column": 1 } } ] } ] } ] }
//! ```
//!
//! **`files`** relates files, many-to-many: one record per pair, and a path may be in several. The
//! relation says what the pair is - `same` (one file, same path), `renamed` (one file, new path),
//! `split` (the before file went to several after files; one record each), `merged` (several
//! before files went into the after file), `added` and `removed` (one side only). `same` and
//! `renamed` are one-to-one, so a file with two counterparts is `split` or `merged` in every
//! record. `solved` is per record, so a session can stop half way; [`HumanChangeset::is_solved`]
//! says whether the whole changeset is done. `verdict` is for a pair of files without a tree, in
//! `human_content`'s spelling.
//!
//! A relation is about files and constrains no node: a function may leave a `same` file for
//! another, and a file that is `added` may hold a copy of a function from elsewhere. `added` only
//! says that no before file is this file.
//!
//! **`entries`** is the node truth, with `human_mapping`'s operations and tree paths
//! ([`super::path_for_node`]); each side names its file. **A node may be in several match
//! entries**: one before node matched to two after nodes is a copy, two matched to one is a
//! merge, and no partner is the "origin". On disk a whole identical subtree is one
//! `identical_with_children` entry, as in `human_mapping.json`; [`HumanChangeset::from_json`]
//! expands it, and nothing past loading knows it exists.
//!
//! **`text_mappings`** are paintings, as in `human_mapping.json`, each range naming its file:
//! `Match` (both ends; `Move` when every range reads the same, `Update` otherwise), `Delete`,
//! `Insert`. A `Minimal`/`Full` name ties a painting to a preset. Unpainted text is unchanged and
//! where it was, in the file a `same` or `renamed` relation gives its file.
//!
//! **Saving is deterministic** ([`HumanChangeset::to_json`]): relations sorted by their paths;
//! entries collapsed, then sorted by file pair and by where their nodes are in the two trees;
//! a painting's entries sorted by their first range, and each side's ranges by file and start.
//! Paintings keep their order, which is theirs to choose. Loading a saved file and saving it
//! again writes the same bytes.
//!
//! # Checks
//!
//! [`validate`] runs these, each a function of that name here:
//!
//! * `relations_are_well_formed` - a record's sides and paths fit its relation; no pair twice.
//! * `referenced_files_exist` - every path a relation, entry or range names is a well-formed path
//!   of a file on that side.
//! * `added_and_removed_files_have_no_counterpart` - a file in an `added` or `removed` record is in
//!   no other record.
//! * `same_and_renamed_relations_are_one_to_one`, `split_and_merged_relations_share_a_file` - the
//!   relation agrees with how many counterparts the file has.
//! * `verdicts_are_on_pairs_without_trees` - a verdict is on a two-sided record, and not on two
//!   parsed files.
//! * `entry_sides_agree_with_operation` - a match has both sides, a delete only `before`, an insert
//!   only `after`.
//! * `entry_paths_resolve` - every entry's path resolves in its file's tree.
//! * `matched_pairs_are_recorded_once` - one pair of nodes has one entry.
//! * `removed_nodes_are_not_matched` - a node that is deleted (or inside a
//!   `delete_with_children`) is in no match, and likewise an inserted one.
//! * `painting_entries_are_well_formed` - a `Match` has both ends, a `Delete` only `before`, an
//!   `Insert` only `after`; every range is non-empty and inside a text file.
//! * `painted_ranges_do_not_overlap` - no two ranges of one painting claim the same byte of a
//!   file. This is `human_mapping`'s invariant 7, run per file.
//! * `solved_files_are_fully_marked` - a file whose every record is `solved` has no node the
//!   entries say nothing about.
//! * `single_file_invariants_hold_per_pair` - see below.
//!
//! An unsolved record is not a violation, and neither is a file no record mentions: both are
//! partial progress, and [`HumanChangeset::is_solved`] is false for them.
//!
//! # The single-file invariants
//!
//! `human_mapping::invariants` checks a truth of one file pair, where a node has at most one
//! partner. They are reused, not rewritten, through a **pair view**: the changeset truth as it
//! reads from inside one pair of files, as a [`HumanMapping`].
//!
//! For a `same` or `renamed` pair, the view holds the entries between the two files; a node whose
//! partners are all in other files is a `delete` or `insert` in the view, which is what it is to
//! this pair; and a painted `Match` whose other end is in another file is a `Delete` or `Insert`.
//! All of the invariants run on that view **except 7**, which is `painted_ranges_do_not_overlap`
//! here, run over the whole changeset.
//!
//! What does **not** carry over, because matching is many-to-many:
//!
//! * **At a node with several partners, or whose partner has several, none of them is judged.**
//!   The view keeps one partner and calls the others inserted or deleted, so every rule that reads
//!   a node's partner or its painting is false there by construction: 9 and 11 (a copy is
//!   "unmatched" in the view but rightly painted `Move`, or left unpainted beside its original),
//!   10, 22 and 27 (the original stays, identical and paired, and is also painted as the source of
//!   a copy elsewhere), 23 (a painted `Match` holds the partner the view dropped), and 3, 12, 13,
//!   17, 18, 20, 21, 24, 25, 26, 28 for the same reason. A violation with a site inside such a
//!   node is dropped; elsewhere in the pair every rule holds.
//! * **Between `split` or `merged` files, and between files no record relates, only 14 and 15
//!   run** (an `identical` pair is token-identical; a `match_but_not_identical` pair differs).
//!   Every other rule reads a painting against a tree, and "unpainted is unchanged and in place"
//!   has no one counterpart file to be in place in. The same two rules judge the partners a
//!   `same`/`renamed` view dropped.
//! * `human_mapping`'s multi-map `groups` and `embedded` layers have no changeset form yet: a
//!   several-partner node is several entries, and code in a string is not mapped across files.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use tree_sitter::Node;

use super::PathCache;
use super::human_content::Judgement;
use super::human_mapping::invariants::{
    ground_truth_invariant_violations_for, painted_ranges_do_not_overlap as overlaps_in_one_file,
};
use super::human_mapping::{
    HumanMapping, HumanMappingEntry, HumanOperation, HumanTextEntry, HumanTextMapping,
    HumanTextOperation, HumanTextSpan, HumanTextVerdict, NamedTextMapping, StoredOperation,
    path_refs, span_text, walk_identical_subtree,
};
use crate::code::Code;
use crate::code::language::to_treesitter;

/// The provenance file's name in a changeset directory.
pub const SOURCE_FILE_NAME: &str = "source.json";
/// The truth file's name in a changeset directory.
pub const TRUTH_FILE_NAME: &str = "human_changeset.json";
/// The datasets under `src/test/data/changesets/`: `handmade` (written for this repository) and
/// `changeset-pilot` (the first sampled draw, `src/test/data/changesets.csv`). A dataset has a
/// directory once a changeset of it is in the corpus.
pub const CHANGESET_DATASETS: &[&str] = &["handmade", "changeset-pilot"];
/// The most **changed** files a changeset of either kind may hold, a path both sides have
/// counting once. Untouched files are not counted.
pub const MAX_CHANGED_FILES: usize = 20;
/// The most untouched files a `commit` may hold: each is there because a signal points at it.
pub const MAX_COMMIT_UNTOUCHED_FILES: usize = 3;
/// The most files a side of a `folders` changeset may hold, unchanged ones included.
pub const MAX_TREE_FILES: usize = 200;
/// What a changeset can show, as `signals` names them: the sampler's signals
/// (`stats::changeset::Signal`, which defines each), in name order.
pub const SIGNALS: [&str; 9] = [
    "block-move",
    "copy",
    "cross-language",
    "directory-move",
    "moved-and-edited",
    "non-code-move",
    "rename-edited",
    "rename-unedited",
    "split-or-merge",
];
/// The one entry of `signals` for a changeset that shows none of [`SIGNALS`].
pub const CONTROL_SIGNAL: &str = "control";
/// `human_solver`'s triage states, as `status` spells them.
pub const STATUSES: [&str; 3] = ["SAMPLED", "PROMOTED", "REJECTED"];
/// Why a `folders` tree lacks something its directory held ([`Omitted::reason`]).
pub const OMITTED_REASONS: [&str; 3] = ["size", "symlink", "submodule"];

/// The fewest files of a fixture each loading thread is given: below twice this, one thread.
const PARALLEL_LOAD_FILES: usize = 16;

const BEFORE: usize = 0;
const AFTER: usize = 1;
const SIDE_NAMES: [&str; 2] = ["before", "after"];

// ─── Paths and directories ──────────────────────────────────────────────────────────────────

/// `Err` unless `path` is a repository-relative path as the JSON spells one: `/`-separated,
/// not absolute, no empty, `.` or `..` component, no backslash, NUL or drive prefix.
pub fn check_repo_path(path: &str) -> Result<()> {
    ensure!(!path.is_empty(), "an empty path");
    ensure!(
        !path.contains('\\') && !path.contains('\0'),
        "path {path:?} holds a backslash or a NUL"
    );
    ensure!(!path.starts_with('/'), "path {path:?} is absolute");
    for (index, component) in path.split('/').enumerate() {
        ensure!(
            !component.is_empty() && component != "." && component != "..",
            "path {path:?} has an empty, `.` or `..` component"
        );
        ensure!(
            !(index == 0 && component.len() == 2 && component.ends_with(':')),
            "path {path:?} starts with a drive"
        );
    }
    Ok(())
}

/// `src/test/data/changesets/`.
pub fn changesets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join("changesets")
}

/// Every changeset as `(dataset, name, directory)`, sorted by dataset and name. A directory under
/// the root that [`CHANGESET_DATASETS`] does not name is an error.
pub fn changeset_dirs() -> Result<Vec<(String, String, PathBuf)>> {
    let root = changesets_root();
    let mut found = Vec::new();
    for dataset in fs::read_dir(&root).with_context(|| format!("reading {root:?}"))? {
        let dataset = dataset?;
        let dataset_name = dataset.file_name().to_string_lossy().into_owned();
        ensure!(
            dataset.file_type()?.is_dir() && CHANGESET_DATASETS.contains(&dataset_name.as_str()),
            "{:?} is not one of the changeset datasets {CHANGESET_DATASETS:?}",
            dataset.path()
        );
        for case in fs::read_dir(dataset.path())? {
            let case = case?;
            ensure!(
                case.file_type()?.is_dir(),
                "{:?} is not a changeset directory",
                case.path()
            );
            found.push((
                dataset_name.clone(),
                case.file_name().to_string_lossy().into_owned(),
                case.path(),
            ));
        }
    }
    found.sort();
    Ok(found)
}

/// The directory of changeset `name` in whichever dataset holds it.
pub fn changeset_dir(name: &str) -> Option<PathBuf> {
    CHANGESET_DATASETS
        .iter()
        .map(|dataset| changesets_root().join(dataset).join(name))
        .find(|path| path.is_dir())
}

// ─── The two trees ──────────────────────────────────────────────────────────────────────────

/// One file of one side.
#[derive(Debug, Clone)]
pub struct ChangesetFile {
    /// Its path as the JSON spells it: `src/a.rs`.
    pub path: String,
    pub bytes: Vec<u8>,
    /// The file as text, parsed when its path names a language with a grammar. `None` for a file
    /// that does not decode as text.
    pub code: Option<Code>,
}

impl ChangesetFile {
    fn from_bytes(path: &str, bytes: Vec<u8>, parser: &mut tree_sitter::Parser) -> Result<Self> {
        let code = match crate::code::decode_text(&bytes) {
            None => None,
            Some(contents) => {
                let mut code = Code {
                    contents,
                    ..Default::default()
                };
                // The real path, never the fixture's: the language is the file's own.
                code.metadata.path = Some(PathBuf::from(path));
                crate::code::metadata::hermetic_expand(&mut code.metadata);
                // A language that is recognised but has no grammar (Markdown) is text without
                // a tree, as a file of no known language is.
                if code
                    .metadata
                    .language
                    .is_some_and(|language| to_treesitter(&language).is_some())
                {
                    code.parse(parser);
                    code.ensure_parsed()
                        .with_context(|| format!("parsing {path}"))?;
                }
                Some(code)
            }
        };
        Ok(Self {
            path: path.to_string(),
            bytes,
            code,
        })
    }

    /// The root of the file's tree, when it has one.
    pub fn root(&self) -> Option<Node<'_>> {
        Some(self.code.as_ref()?.ast.as_ref()?.root_node())
    }

    /// The file as text, when it is text.
    pub fn text(&self) -> Option<&str> {
        self.code.as_ref().map(|code| code.contents.as_str())
    }
}

/// The before and after trees of a changeset, by path.
#[derive(Debug, Clone, Default)]
pub struct ChangesetTrees {
    pub before: BTreeMap<String, ChangesetFile>,
    pub after: BTreeMap<String, ChangesetFile>,
}

impl ChangesetTrees {
    /// Reads `dir/before` and `dir/after`. A side without a directory is empty.
    ///
    /// The files are read and parsed on several threads once there are enough of them: a
    /// folder fixture holds some 200 a side, and parsing them one after another was most of the
    /// quarter of a second it took to open one.
    pub fn load(dir: &Path) -> Result<Self> {
        let mut found: Vec<(usize, String, PathBuf)> = Vec::new();
        for (side, name) in SIDE_NAMES.iter().enumerate() {
            let root = dir.join(name);
            if !root.exists() {
                continue;
            }
            let mut files = Vec::new();
            collect_files(&root, "", &mut files)?;
            found.extend(
                files
                    .into_iter()
                    .map(|(path, on_disk)| (side, path, on_disk)),
            );
        }
        let parse = |files: &[(usize, String, PathBuf)]| -> Vec<Result<ChangesetFile>> {
            let mut parser = tree_sitter::Parser::new();
            files
                .iter()
                .map(|(_, path, on_disk)| {
                    let bytes =
                        fs::read(on_disk).with_context(|| format!("reading {on_disk:?}"))?;
                    ChangesetFile::from_bytes(path, bytes, &mut parser)
                })
                .collect()
        };
        let workers = std::thread::available_parallelism()
            .map_or(1, |count| count.get())
            .min(found.len() / PARALLEL_LOAD_FILES)
            .max(1);
        let parsed: Vec<Result<ChangesetFile>> = if workers == 1 {
            parse(&found)
        } else {
            // Chunks in order, so the first error is the same one a single thread would meet.
            let chunk = found.len().div_ceil(workers);
            std::thread::scope(|scope| {
                let handles: Vec<_> = found
                    .chunks(chunk)
                    .map(|files| scope.spawn(|| parse(files)))
                    .collect();
                handles
                    .into_iter()
                    .flat_map(|handle| handle.join().expect("a parsing thread panicked"))
                    .collect()
            })
        };
        let mut sides = [BTreeMap::new(), BTreeMap::new()];
        for ((side, path, _), file) in found.iter().zip(parsed) {
            sides[*side].insert(path.clone(), file?);
        }
        let [before, after] = sides;
        Ok(Self { before, after })
    }

    /// The trees of in-memory files, `(path, bytes)` per side. For tests and tools that have no
    /// directory yet.
    pub fn from_files(before: &[(&str, &[u8])], after: &[(&str, &[u8])]) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();
        let mut sides = [BTreeMap::new(), BTreeMap::new()];
        for (files, given) in sides.iter_mut().zip([before, after]) {
            for (path, bytes) in given {
                check_repo_path(path)?;
                files.insert(
                    path.to_string(),
                    ChangesetFile::from_bytes(path, bytes.to_vec(), &mut parser)?,
                );
            }
        }
        let [before, after] = sides;
        Ok(Self { before, after })
    }

    fn side(&self, side: usize) -> &BTreeMap<String, ChangesetFile> {
        if side == BEFORE {
            &self.before
        } else {
            &self.after
        }
    }

    /// How many distinct paths the two sides hold.
    pub fn path_count(&self) -> usize {
        self.before
            .keys()
            .chain(self.after.keys())
            .collect::<BTreeSet<_>>()
            .len()
    }
}

/// Every file under `dir`, as `(JSON path, path on disk)`, depth first in name order.
fn collect_files(dir: &Path, prefix: &str, found: &mut Vec<(String, PathBuf)>) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("reading {dir:?}"))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name
            .to_str()
            .with_context(|| format!("{:?} is not UTF-8", entry.path()))?;
        // `file_type` does not follow links, so a link out of the fixture is never read.
        let file_type = entry.file_type()?;
        ensure!(
            !file_type.is_symlink(),
            "{:?} is a symbolic link",
            entry.path()
        );
        if file_type.is_dir() {
            collect_files(&entry.path(), &format!("{prefix}{name}/"), found)?;
            continue;
        }
        let real = name
            .strip_suffix(".test")
            .with_context(|| format!("{:?} does not end in `.test`", entry.path()))?;
        let path = format!("{prefix}{real}");
        check_repo_path(&path)?;
        found.push((path, entry.path()));
    }
    Ok(())
}

// ─── source.json ────────────────────────────────────────────────────────────────────────────

/// What a changeset is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangesetKind {
    /// One commit: the files it changes, the untouched files a signal points at, and what the
    /// VCS says happened.
    Commit,
    /// Two directory trees as they lie on disk, complete, with nothing from a VCS.
    Folders,
}

impl ChangesetKind {
    /// The kind as `source.json` and `changesets.csv` spell it.
    pub fn name(self) -> &'static str {
        match self {
            ChangesetKind::Commit => "commit",
            ChangesetKind::Folders => "folders",
        }
    }
}

/// `source.json`: where a changeset came from and, for a commit, what the VCS says about it.
/// The one definition of the file: the materializer writes it through this type, `human_solver`
/// reads and rewrites it through it, and a key it does not have is an error. See the module doc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesetSource {
    pub kind: ChangesetKind,
    /// The sampled repository; absent, with `commit` and `parent`, for a handmade changeset.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repository: String,
    /// The commit whose tree is `after/`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub commit: String,
    /// The commit whose tree is `before/`: `commit`'s parent, or for folders any earlier point.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent: String,
    /// Folders only: the sub-folder of the repository the two trees are, `.` for its root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Every phenomenon the changeset was drawn or written for, of [`SIGNALS`], in name order
    /// and none ranking above another; [`CONTROL_SIGNAL`] alone when there is none.
    pub signals: Vec<String>,
    /// The dataset it belongs to, which is also its parent directory once promoted.
    pub dataset: String,
    /// `human_solver`'s triage state, one of [`STATUSES`].
    pub status: String,
    pub files: Vec<SourceFile>,
    /// Folders only: what the two directories held and the trees here do not.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub omitted: Vec<Omitted>,
    /// Commits only, and always there for one; two folders have no VCS to ask, and no key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcs_hints: Option<Vec<VcsHint>>,
}

/// What a `folders` tree held on disk at a path and the fixture does not: the tree is known to
/// be incomplete exactly there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Omitted {
    /// `before` or `after`.
    pub side: String,
    /// Relative to the sub-folder, as every path of the fixture.
    pub path: String,
    /// One of [`OMITTED_REASONS`]: `size` (an unchanged file over the sampler's size cap),
    /// `symlink` or `submodule`.
    pub reason: String,
    /// Of the file or of the link's target path, in bytes; none for a submodule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    /// The git blob, or the submodule's commit.
    pub id: String,
}

/// One path of the fixture. Both keys, equal, when both sides have the path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// Byte-identical at this path on both sides. A commit holds such a file because a signal
    /// points at it (the source of a copy); folders hold every file that did not change.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub untouched: bool,
}

/// `git diff --name-status`'s letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HintStatus {
    /// Added.
    A,
    /// Deleted.
    D,
    /// Modified.
    M,
    /// Renamed, from `old_path`.
    R,
    /// Copied, from `old_path`.
    C,
}

/// One line of what the VCS reports. A hint, never truth.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VcsHint {
    pub status: HintStatus,
    /// The path the line is about: the after side's, or the before side's for `D`.
    pub path: String,
    /// Where an `R` or `C` came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    /// The VCS's similarity score for an `R` or `C`, in percent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub similarity: Option<u8>,
}

impl HintStatus {
    /// The status `git diff --name-status` spells `letter`; `None` for one the format has no
    /// name for (`T`, a type change, which a changeset never holds).
    pub fn from_letter(letter: char) -> Option<Self> {
        Some(match letter {
            'A' => HintStatus::A,
            'D' => HintStatus::D,
            'M' => HintStatus::M,
            'R' => HintStatus::R,
            'C' => HintStatus::C,
            _ => return None,
        })
    }
}

impl ChangesetSource {
    /// The hints, none for folders.
    pub fn hints(&self) -> &[VcsHint] {
        self.vcs_hints.as_deref().unwrap_or_default()
    }

    /// The paths that changed: every path of `files` that is not `untouched`.
    pub fn changed_paths(&self) -> BTreeSet<&str> {
        self.files
            .iter()
            .filter(|file| !file.untouched)
            .flat_map(|file| [file.before.as_deref(), file.after.as_deref()])
            .flatten()
            .collect()
    }

    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(SOURCE_FILE_NAME);
        let json = fs::read_to_string(&path).with_context(|| format!("reading {path:?}"))?;
        serde_json::from_str(&json).with_context(|| format!("parsing {path:?}"))
    }

    /// The file's bytes: pretty JSON and a final newline.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }
}

/// What `files` must be for `trees`: every path once, in path order, `untouched` for a path
/// that is byte-identical on both sides.
pub fn source_files(trees: &ChangesetTrees) -> Vec<SourceFile> {
    let paths: BTreeSet<&String> = trees.before.keys().chain(trees.after.keys()).collect();
    paths
        .into_iter()
        .map(|path| {
            let (before, after) = (trees.before.get(path), trees.after.get(path));
            SourceFile {
                before: before.map(|_| path.clone()),
                after: after.map(|_| path.clone()),
                untouched: matches!((before, after), (Some(b), Some(a)) if b.bytes == a.bytes),
            }
        })
        .collect()
}

/// Every way `source` disagrees with `trees` or with itself, in a stable order. Empty is a pass.
///
/// For either kind: `files` names every path of the trees once, in path order, and nothing
/// else; a file is
/// `untouched` exactly when it is on both sides with the same bytes; at most
/// [`MAX_CHANGED_FILES`] paths changed; `signals`, `status` and the provenance keys are
/// well-formed. Then by `kind`, as the module doc lists: a commit's hints and its cap on
/// untouched files ([`commit_violations`]), or a folder's root, tree cap and `omitted`
/// ([`folders_violations`]).
pub fn source_violations(source: &ChangesetSource, trees: &ChangesetTrees) -> Vec<String> {
    let mut violations = Vec::new();
    let mut listed: [BTreeSet<&str>; 2] = Default::default();
    for file in &source.files {
        let sides = [&file.before, &file.after];
        if let (Some(before), Some(after)) = (&file.before, &file.after)
            && before != after
        {
            violations.push(format!(
                "files pairs {before} with {after}: which path became which is not source.json's \
                 to say"
            ));
        }
        if file.before.is_none() && file.after.is_none() {
            violations.push("a files record names no path".to_string());
        }
        for (side, path) in sides.iter().enumerate() {
            let Some(path) = path else { continue };
            if let Err(error) = check_repo_path(path) {
                violations.push(format!("files: {error}"));
            } else if !trees.side(side).contains_key(path) {
                violations.push(format!(
                    "files names {path}, which {}/ lacks",
                    SIDE_NAMES[side]
                ));
            } else if !listed[side].insert(path) {
                violations.push(format!(
                    "files names {path} twice on the {} side",
                    SIDE_NAMES[side]
                ));
            }
        }
        let same_bytes = match (&file.before, &file.after) {
            (Some(before), Some(after)) => {
                match (trees.before.get(before), trees.after.get(after)) {
                    (Some(before), Some(after)) => Some(before.bytes == after.bytes),
                    _ => None,
                }
            }
            _ => Some(false),
        };
        let path = file.before.as_ref().or(file.after.as_ref());
        match (file.untouched, same_bytes, path) {
            (true, Some(false), Some(path)) => violations.push(format!(
                "{path} is marked untouched but is not the same on both sides"
            )),
            (false, Some(true), Some(path)) => violations.push(format!(
                "{path} is the same on both sides but not marked untouched"
            )),
            _ => {}
        }
    }
    for side in [BEFORE, AFTER] {
        for path in trees.side(side).keys() {
            if !listed[side].contains(path.as_str()) {
                violations.push(format!("files does not name {}/{path}", SIDE_NAMES[side]));
            }
        }
    }
    let path_of = |file: &SourceFile| file.before.clone().or(file.after.clone());
    if !source
        .files
        .windows(2)
        .all(|pair| path_of(&pair[0]) < path_of(&pair[1]))
    {
        violations.push("files is not in path order".to_string());
    }
    let changed = source.changed_paths();
    if changed.len() > MAX_CHANGED_FILES {
        violations.push(format!(
            "{} changed files, more than the {MAX_CHANGED_FILES} a changeset may hold",
            changed.len()
        ));
    }

    let known = |name: &String| SIGNALS.contains(&name.as_str());
    if source.signals.is_empty() {
        violations.push(format!(
            "signals is empty: a changeset with none is [\"{CONTROL_SIGNAL}\"]"
        ));
    } else if source.signals != [CONTROL_SIGNAL] {
        for name in source.signals.iter().filter(|name| !known(name)) {
            violations.push(format!(
                "signals names {name:?}, which is not one of {SIGNALS:?} (or \
                 \"{CONTROL_SIGNAL}\", alone)"
            ));
        }
        if !source.signals.windows(2).all(|pair| pair[0] < pair[1]) {
            violations.push("signals is not sorted by name, each once".to_string());
        }
    }
    if !STATUSES.contains(&source.status.as_str()) {
        violations.push(format!(
            "status {:?} is not one of {STATUSES:?}",
            source.status
        ));
    }
    let provenance = [&source.repository, &source.commit, &source.parent];
    if provenance.iter().any(|key| key.is_empty()) && !provenance.iter().all(|key| key.is_empty()) {
        violations.push(
            "repository, commit and parent go together: all three, or none for a changeset \
             nothing was sampled from"
                .to_string(),
        );
    }
    violations.extend(match source.kind {
        ChangesetKind::Commit => commit_violations(source, trees, &changed),
        ChangesetKind::Folders => folders_violations(source, trees),
    });
    violations
}

/// What only a `commit` is held to: no `root` and nothing `omitted`; at most
/// [`MAX_COMMIT_UNTOUCHED_FILES`] untouched files; `vcs_hints` is there, every hint's paths
/// exist on the side its status says, and the hints between them name exactly the `changed`
/// paths.
fn commit_violations(
    source: &ChangesetSource,
    trees: &ChangesetTrees,
    changed: &BTreeSet<&str>,
) -> Vec<String> {
    let mut violations = Vec::new();
    if source.root.is_some() {
        violations.push("a commit has no root: the paths are the repository's".to_string());
    }
    if !source.omitted.is_empty() {
        violations.push("a commit omits nothing: omitted is for folders".to_string());
    }
    let untouched = source.files.iter().filter(|file| file.untouched).count();
    if untouched > MAX_COMMIT_UNTOUCHED_FILES {
        violations.push(format!(
            "{untouched} untouched files, more than the {MAX_COMMIT_UNTOUCHED_FILES} a commit \
             may hold"
        ));
    }
    let Some(hints) = &source.vcs_hints else {
        violations
            .push("a commit has vcs_hints, an empty list when the VCS reports nothing".to_string());
        return violations;
    };
    let mut hinted: BTreeSet<&str> = BTreeSet::new();
    for hint in hints {
        let on = |side: usize, path: &str| trees.side(side).contains_key(path);
        let describe = format!("hint {:?} {}", hint.status, hint.path);
        hinted.insert(&hint.path);
        let takes_old_path = matches!(hint.status, HintStatus::R | HintStatus::C);
        match (&hint.old_path, takes_old_path) {
            (Some(old), true) => {
                if !on(BEFORE, old) {
                    violations.push(format!("{describe}: before/ lacks its old path {old}"));
                }
                if hint.status == HintStatus::R {
                    hinted.insert(old);
                    if on(AFTER, old) {
                        violations
                            .push(format!("{describe}: its old path {old} is still in after/"));
                    }
                }
            }
            (None, true) => violations.push(format!("{describe} has no old_path")),
            (Some(_), false) => violations.push(format!("{describe} has an old_path")),
            (None, false) => {}
        }
        if hint.similarity.is_some() != takes_old_path {
            violations.push(format!(
                "{describe}: a similarity goes with R and C, and only with them"
            ));
        }
        let (wants_before, wants_after) = match hint.status {
            HintStatus::A | HintStatus::R | HintStatus::C => (false, true),
            HintStatus::D => (true, false),
            HintStatus::M => (true, true),
        };
        if on(BEFORE, &hint.path) != wants_before && hint.status != HintStatus::C {
            violations.push(format!("{describe}: wrong for what before/ holds"));
        }
        if on(AFTER, &hint.path) != wants_after {
            violations.push(format!("{describe}: wrong for what after/ holds"));
        }
    }
    if hinted != *changed {
        let only_hinted: Vec<_> = hinted.difference(changed).collect();
        let only_changed: Vec<_> = changed.difference(&hinted).collect();
        violations.push(format!(
            "the hints and the changed files differ: hinted only {only_hinted:?}, changed only \
             {only_changed:?}"
        ));
    }
    violations
}

/// What only `folders` are held to: a `root` (`.` or a path); no `vcs_hints`; at most
/// [`MAX_TREE_FILES`] files a side; and `omitted` in path and side order, each entry once,
/// well-formed, and about a path its side's tree does not hold.
fn folders_violations(source: &ChangesetSource, trees: &ChangesetTrees) -> Vec<String> {
    let mut violations = Vec::new();
    match source.root.as_deref() {
        None => violations.push(
            "folders name their root: the sub-folder the trees are, `.` for the repository's"
                .to_string(),
        ),
        Some(".") => {}
        Some(root) => {
            if let Err(error) = check_repo_path(root) {
                violations.push(format!("root: {error}"));
            }
        }
    }
    if source.vcs_hints.is_some() {
        violations.push("folders have no vcs_hints: there is no VCS to ask".to_string());
    }
    for side in [BEFORE, AFTER] {
        let files = trees.side(side).len();
        if files > MAX_TREE_FILES {
            violations.push(format!(
                "{}/ holds {files} files, more than the {MAX_TREE_FILES} a side of a folder may",
                SIDE_NAMES[side]
            ));
        }
    }
    let key = |omitted: &Omitted| (omitted.path.clone(), omitted.side.clone());
    if !source
        .omitted
        .windows(2)
        .all(|pair| key(&pair[0]) < key(&pair[1]))
    {
        violations.push("omitted is not sorted by path and side, each once".to_string());
    }
    for omitted in &source.omitted {
        let describe = format!("omitted {}/{}", omitted.side, omitted.path);
        let Some(side) = SIDE_NAMES.iter().position(|name| *name == omitted.side) else {
            violations.push(format!("{describe}: the side is before or after"));
            continue;
        };
        if let Err(error) = check_repo_path(&omitted.path) {
            violations.push(format!("{describe}: {error}"));
        } else if trees.side(side).contains_key(&omitted.path) {
            violations.push(format!("{describe}: the tree holds that file"));
        }
        if !OMITTED_REASONS.contains(&omitted.reason.as_str()) {
            violations.push(format!(
                "{describe}: reason {:?} is not one of {OMITTED_REASONS:?}",
                omitted.reason
            ));
        }
        if omitted.bytes.is_some() == (omitted.reason == "submodule") {
            violations.push(format!(
                "{describe}: a size in bytes goes with a file or a link, and not with a \
                 submodule"
            ));
        }
        if omitted.id.is_empty() {
            violations.push(format!("{describe}: no id"));
        }
    }
    violations
}

// ─── human_changeset.json ───────────────────────────────────────────────────────────────────

/// What a pair of files is to each other. See the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileRelationKind {
    Same,
    Renamed,
    Split,
    Merged,
    Added,
    Removed,
}

/// One record of `files`: a pair of files and what they are to each other, or one file that was
/// added or removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRelation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    pub relation: FileRelationKind,
    /// Whether a human is done with this record: its files' nodes are all decided.
    pub solved: bool,
    /// For a pair without trees: what changed, as `human_content` judges a content pair.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Judgement>,
}

/// A node: its file and its path in that file's tree ([`super::path_for_node`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeRef {
    pub file: String,
    pub path: Vec<String>,
}

impl NodeRef {
    pub fn new(file: &str, path: &[&str]) -> Self {
        Self {
            file: file.to_string(),
            path: path.iter().map(|segment| segment.to_string()).collect(),
        }
    }
}

/// One decision about a node or a pair of nodes: `human_mapping`'s entry, each side in a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesetEntry {
    pub operation: HumanOperation,
    pub before: Option<NodeRef>,
    pub after: Option<NodeRef>,
}

impl ChangesetEntry {
    pub fn matched(operation: HumanOperation, before: NodeRef, after: NodeRef) -> Self {
        Self {
            operation,
            before: Some(before),
            after: Some(after),
        }
    }

    pub fn deleted(operation: HumanOperation, before: NodeRef) -> Self {
        Self {
            operation,
            before: Some(before),
            after: None,
        }
    }

    pub fn inserted(operation: HumanOperation, after: NodeRef) -> Self {
        Self {
            operation,
            before: None,
            after: Some(after),
        }
    }
}

/// A painted range of one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpan {
    pub file: String,
    pub range: HumanTextSpan,
}

/// One painted decision, as [`HumanTextEntry`], over ranges that name their files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesetTextEntry {
    pub operation: HumanTextOperation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<FileSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<FileSpan>,
}

impl ChangesetTextEntry {
    /// What the entry asserts, `Move` or `Update` derived for a `Match` from whether its ranges all
    /// read the same. `Err` when the entry is malformed for its operation or a range is outside
    /// its file.
    pub fn verdict(&self, trees: &ChangesetTrees) -> Result<HumanTextVerdict> {
        let mut texts = Vec::new();
        for (side, spans) in [&self.before, &self.after].into_iter().enumerate() {
            for span in spans {
                let text = trees
                    .side(side)
                    .get(&span.file)
                    .and_then(ChangesetFile::text)
                    .and_then(|text| span_text(text, span.range))
                    .with_context(|| {
                        format!(
                            "a range is outside the {} file {}",
                            SIDE_NAMES[side], span.file
                        )
                    })?;
                texts.push(text);
            }
        }
        let (has_before, has_after) = (!self.before.is_empty(), !self.after.is_empty());
        match self.operation {
            HumanTextOperation::Delete => {
                ensure!(
                    has_before && !has_after,
                    "a Delete has `before` ranges only"
                );
                Ok(HumanTextVerdict::Delete)
            }
            HumanTextOperation::Insert => {
                ensure!(
                    has_after && !has_before,
                    "an Insert has `after` ranges only"
                );
                Ok(HumanTextVerdict::Insert)
            }
            HumanTextOperation::Match => {
                ensure!(has_before && has_after, "a Match has both ends");
                Ok(if texts.windows(2).all(|pair| pair[0] == pair[1]) {
                    HumanTextVerdict::Move
                } else {
                    HumanTextVerdict::Update
                })
            }
        }
    }
}

/// One painting of the whole changeset, under a name as [`NamedTextMapping`]'s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesetTextMapping {
    pub name: String,
    pub entries: Vec<ChangesetTextEntry>,
}

/// The truth of one changeset, in memory: `human_changeset.json` with every
/// `identical_with_children` expanded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HumanChangeset {
    pub files: Vec<FileRelation>,
    pub entries: Vec<ChangesetEntry>,
    pub text_mappings: Vec<ChangesetTextMapping>,
}

/// [`ChangesetEntry`] as the file holds it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEntry {
    operation: StoredOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    before: Option<NodeRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<NodeRef>,
}

/// [`HumanChangeset`] as the file holds it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredChangeset {
    files: Vec<FileRelation>,
    #[serde(default)]
    entries: Vec<StoredEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    text_mappings: Vec<ChangesetTextMapping>,
}

/// Resolves [`NodeRef`]s against the trees, and says where a node is in its tree.
struct Resolver<'t> {
    trees: &'t ChangesetTrees,
    paths: [HashMap<&'t str, PathCache<'t>>; 2],
    preorder: [HashMap<&'t str, HashMap<usize, usize>>; 2],
}

impl<'t> Resolver<'t> {
    fn new(trees: &'t ChangesetTrees) -> Self {
        Self {
            trees,
            paths: Default::default(),
            preorder: Default::default(),
        }
    }

    fn file(&self, side: usize, path: &str) -> Result<&'t ChangesetFile> {
        self.trees
            .side(side)
            .get(path)
            .with_context(|| format!("no file {path} on the {} side", SIDE_NAMES[side]))
    }

    fn node(&mut self, side: usize, node: &NodeRef) -> Result<(&'t ChangesetFile, Node<'t>)> {
        let file = self.file(side, &node.file)?;
        let root = file.root().with_context(|| {
            format!("{} on the {} side has no tree", node.file, SIDE_NAMES[side])
        })?;
        let found = self.paths[side]
            .entry(file.path.as_str())
            .or_default()
            .resolve(root, &path_refs(&node.path))
            .with_context(|| format!("in {} on the {} side", node.file, SIDE_NAMES[side]))?;
        Ok((file, found))
    }

    /// Where `node` comes in a pre-order walk of its file; `usize::MAX` when it does not resolve.
    fn position(&mut self, side: usize, node: &NodeRef) -> usize {
        let Ok((file, found)) = self.node(side, node) else {
            return usize::MAX;
        };
        let index = self.preorder[side]
            .entry(file.path.as_str())
            .or_insert_with(|| {
                let mut index = HashMap::new();
                let mut stack = vec![found_root(found)];
                while let Some(node) = stack.pop() {
                    index.insert(node.id(), index.len());
                    let mut cursor = node.walk();
                    let children: Vec<Node> = node.children(&mut cursor).collect();
                    stack.extend(children.into_iter().rev());
                }
                index
            });
        index[&found.id()]
    }
}

fn found_root(mut node: Node) -> Node {
    while let Some(parent) = node.parent() {
        node = parent;
    }
    node
}

fn operation_rank(operation: HumanOperation) -> u8 {
    match operation {
        HumanOperation::Identical => 0,
        HumanOperation::Update => 1,
        HumanOperation::MatchButNotIdentical => 2,
        HumanOperation::Delete => 3,
        HumanOperation::DeleteWithChildren => 4,
        HumanOperation::Insert => 5,
        HumanOperation::InsertWithChildren => 6,
    }
}

fn is_match(operation: HumanOperation) -> bool {
    matches!(
        operation,
        HumanOperation::Identical | HumanOperation::Update | HumanOperation::MatchButNotIdentical
    )
}

fn is_delete(operation: HumanOperation) -> bool {
    matches!(
        operation,
        HumanOperation::Delete | HumanOperation::DeleteWithChildren
    )
}

type PairKey = (NodeRef, NodeRef);

impl HumanChangeset {
    /// The truth in `dir`, or `None` when nobody has started one.
    pub fn load(dir: &Path, trees: &ChangesetTrees) -> Result<Option<Self>> {
        let path = dir.join(TRUTH_FILE_NAME);
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path).with_context(|| format!("reading {path:?}"))?;
        Self::from_json(&json, trees)
            .with_context(|| format!("loading {path:?}"))
            .map(Some)
    }

    /// Parses the file form and expands each `identical_with_children` against `trees`.
    pub fn from_json(json: &str, trees: &ChangesetTrees) -> Result<Self> {
        let stored: StoredChangeset = serde_json::from_str(json)?;
        let mut resolver = Resolver::new(trees);
        let mut entries = Vec::with_capacity(stored.entries.len());
        for entry in stored.entries {
            if let Some(operation) = entry.operation.in_memory() {
                entries.push(ChangesetEntry {
                    operation,
                    before: entry.before,
                    after: entry.after,
                });
                continue;
            }
            let (Some(before), Some(after)) = (entry.before, entry.after) else {
                bail!("an identical_with_children entry needs both sides");
            };
            entries.extend(identical_subtree(&mut resolver, &before, &after)?);
        }
        Ok(Self {
            files: stored.files,
            entries,
            text_mappings: stored.text_mappings,
        })
    }

    /// One `Identical` entry for `before`/`after` and for every pair of their descendants, in
    /// pre-order: what a solver records when a human says two subtrees are the same code.
    pub fn identical_subtree(
        trees: &ChangesetTrees,
        before: &NodeRef,
        after: &NodeRef,
    ) -> Result<Vec<ChangesetEntry>> {
        identical_subtree(&mut Resolver::new(trees), before, after)
    }

    /// The file's bytes for this truth: canonical order, identical subtrees collapsed, pretty
    /// JSON and a final newline. See the module doc.
    pub fn to_json(&self, trees: &ChangesetTrees) -> Result<String> {
        let mut resolver = Resolver::new(trees);
        let mut files = self.files.clone();
        files.sort_by(|a, b| {
            (&a.before, &a.after, a.relation).cmp(&(&b.before, &b.after, b.relation))
        });

        // In canonical order a subtree's root comes before its descendants, so collapsing in that
        // order finds the largest identical subtrees first.
        let mut entries = self.entries.clone();
        sort_entries(&mut entries, &mut resolver);
        entries.dedup();
        let identical: HashSet<PairKey> = entries
            .iter()
            .filter(|entry| entry.operation == HumanOperation::Identical)
            .filter_map(|entry| Some((entry.before.clone()?, entry.after.clone()?)))
            .collect();
        let mut collapsed: HashSet<PairKey> = HashSet::new();
        let mut stored = Vec::new();
        for entry in entries {
            let mut operation = StoredOperation::from(entry.operation);
            if let (HumanOperation::Identical, Some(before), Some(after)) =
                (entry.operation, &entry.before, &entry.after)
            {
                if collapsed.contains(&(before.clone(), after.clone())) {
                    continue;
                }
                if let Some(subtree) = whole_subtree(&mut resolver, before, after, &identical) {
                    collapsed.extend(subtree);
                    operation = StoredOperation::IdenticalWithChildren;
                }
            }
            stored.push(StoredEntry {
                operation,
                before: entry.before,
                after: entry.after,
            });
        }

        let mut text_mappings = self.text_mappings.clone();
        for painting in &mut text_mappings {
            let start = |span: &FileSpan| {
                (
                    span.file.clone(),
                    span.range.start_row,
                    span.range.start_column,
                    span.range.end_row,
                    span.range.end_column,
                )
            };
            for entry in &mut painting.entries {
                entry.before.sort_by_key(start);
                entry.after.sort_by_key(start);
            }
            painting.entries.sort_by_key(|entry| {
                (
                    entry.before.first().map(start),
                    entry.after.first().map(start),
                )
            });
        }

        let stored = StoredChangeset {
            files,
            entries: stored,
            text_mappings,
        };
        Ok(serde_json::to_string_pretty(&stored)? + "\n")
    }

    /// Writes [`Self::to_json`] to `dir`.
    pub fn save(&self, dir: &Path, trees: &ChangesetTrees) -> Result<()> {
        let path = dir.join(TRUTH_FILE_NAME);
        fs::write(&path, self.to_json(trees)?).with_context(|| format!("writing {path:?}"))
    }

    /// Whether the whole changeset is solved: every record is `solved`, and every file of both
    /// sides is in a record. Says nothing about validity; see [`validate`].
    pub fn is_solved(&self, trees: &ChangesetTrees) -> bool {
        let related = |side: usize, path: &String| {
            self.files
                .iter()
                .any(|relation| [&relation.before, &relation.after][side].as_ref() == Some(path))
        };
        self.files.iter().all(|relation| relation.solved)
            && trees.before.keys().all(|path| related(BEFORE, path))
            && trees.after.keys().all(|path| related(AFTER, path))
    }
}

/// Sorts entries by file pair, then by where their nodes are in the two trees, then by operation
/// and path (which only entries that do not resolve need).
fn sort_entries(entries: &mut Vec<ChangesetEntry>, resolver: &mut Resolver) {
    let mut keyed: Vec<_> = std::mem::take(entries)
        .into_iter()
        .map(|entry| {
            let file = |node: &Option<NodeRef>| {
                node.as_ref()
                    .map(|node| node.file.clone())
                    .unwrap_or_default()
            };
            let mut position = |side: usize, node: &Option<NodeRef>| {
                node.as_ref()
                    .map_or(0, |node| resolver.position(side, node))
            };
            let key = (
                file(&entry.before),
                file(&entry.after),
                position(BEFORE, &entry.before),
                position(AFTER, &entry.after),
                operation_rank(entry.operation),
            );
            (key, entry)
        })
        .collect();
    keyed.sort_by(|(a_key, a), (b_key, b)| {
        a_key
            .cmp(b_key)
            .then_with(|| (&a.before, &a.after).cmp(&(&b.before, &b.after)))
    });
    entries.extend(keyed.into_iter().map(|(_, entry)| entry));
}

fn identical_subtree(
    resolver: &mut Resolver,
    before: &NodeRef,
    after: &NodeRef,
) -> Result<Vec<ChangesetEntry>> {
    let (_, before_node) = resolver.node(BEFORE, before)?;
    let (_, after_node) = resolver.node(AFTER, after)?;
    let mut entries = Vec::new();
    walk_identical_subtree(
        before_node,
        after_node,
        before.path.clone(),
        after.path.clone(),
        |before_path, after_path| {
            entries.push(ChangesetEntry {
                operation: HumanOperation::Identical,
                before: Some(NodeRef {
                    file: before.file.clone(),
                    path: before_path,
                }),
                after: Some(NodeRef {
                    file: after.file.clone(),
                    path: after_path,
                }),
            });
            true
        },
    )?;
    Ok(entries)
}

/// The pairs `identical_with_children` at `before`/`after` stands for, when the node has children
/// and every one of those pairs is in `identical`.
fn whole_subtree(
    resolver: &mut Resolver,
    before: &NodeRef,
    after: &NodeRef,
    identical: &HashSet<PairKey>,
) -> Option<Vec<PairKey>> {
    let (_, before_node) = resolver.node(BEFORE, before).ok()?;
    let (_, after_node) = resolver.node(AFTER, after).ok()?;
    if before_node.child_count() == 0 {
        return None;
    }
    let mut pairs = Vec::new();
    let complete = walk_identical_subtree(
        before_node,
        after_node,
        before.path.clone(),
        after.path.clone(),
        |before_path, after_path| {
            let pair = (
                NodeRef {
                    file: before.file.clone(),
                    path: before_path,
                },
                NodeRef {
                    file: after.file.clone(),
                    path: after_path,
                },
            );
            let known = identical.contains(&pair);
            pairs.push(pair);
            known
        },
    );
    complete.unwrap_or(false).then_some(pairs)
}

// ─── Validation ─────────────────────────────────────────────────────────────────────────────

/// One way a changeset's truth contradicts itself or its trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesetViolation {
    /// The check that found it, by the name the module doc lists.
    pub check: &'static str,
    /// For `single_file_invariants_hold_per_pair`: which of `human_mapping`'s invariants.
    pub invariant: Option<u8>,
    pub message: String,
    /// For `single_file_invariants_hold_per_pair`: the before and after file it was found
    /// between, so a tool (`human_solver`'s violation list) can open that pair.
    pub pair: Option<(String, String)>,
    /// Where in that pair to look: `(side, range)`, 0 = the before file, 1 = the after file.
    pub sites: Vec<(usize, HumanTextSpan)>,
}

impl std::fmt::Display for ChangesetViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.check, self.message)
    }
}

fn violation(check: &'static str, message: String) -> ChangesetViolation {
    ChangesetViolation {
        check,
        invariant: None,
        message,
        pair: None,
        sites: Vec::new(),
    }
}

/// A node as a key: its file's path and its id in that file's tree.
type NodeKey<'t> = (&'t str, usize);

/// One match entry with both nodes found.
struct ResolvedMatch<'t> {
    entry: usize,
    before: (&'t ChangesetFile, Node<'t>),
    after: (&'t ChangesetFile, Node<'t>),
}

impl<'t> ResolvedMatch<'t> {
    fn key(&self, side: usize) -> NodeKey<'t> {
        let (file, node) = if side == BEFORE {
            self.before
        } else {
            self.after
        };
        (file.path.as_str(), node.id())
    }
}

/// The entries resolved to nodes: `human_mapping::Caches` for a changeset, where a node has a
/// list of partners rather than one.
struct Resolved<'t> {
    matches: Vec<ResolvedMatch<'t>>,
    /// Per side, a node's matches as indices into `matches`.
    partners: [HashMap<NodeKey<'t>, Vec<usize>>; 2],
    /// Per side, the deleted (before) or inserted (after) nodes: entry index and whether the
    /// entry takes the node's children with it.
    removed: [HashMap<NodeKey<'t>, (usize, bool)>; 2],
    /// Entries whose sides fit their operation but do not resolve, with why.
    unresolved: Vec<(usize, String)>,
    /// Per side, the files an entry removes a node of with its children.
    removes_children: [HashSet<&'t str>; 2],
    /// Per side and file, the matches that name the file, as ascending indices into `matches`:
    /// a folder fixture has hundreds of pairs, and each reads only its own two files' entries.
    matches_of: [HashMap<&'t str, Vec<usize>>; 2],
    /// Per side and file, the entries with that side only, as ascending indices into the truth.
    one_sided: [HashMap<&'t str, Vec<usize>>; 2],
}

/// The indices of two ascending lists, ascending, each once.
fn merged(first: Option<&Vec<usize>>, second: Option<&Vec<usize>>) -> Vec<usize> {
    let mut all: Vec<usize> = first.into_iter().chain(second).flatten().copied().collect();
    all.sort_unstable();
    all.dedup();
    all
}

impl<'t> Resolved<'t> {
    fn new(truth: &HumanChangeset, trees: &'t ChangesetTrees) -> Self {
        let mut resolver = Resolver::new(trees);
        let mut resolved = Self {
            matches: Vec::new(),
            partners: Default::default(),
            removed: Default::default(),
            unresolved: Vec::new(),
            removes_children: Default::default(),
            matches_of: Default::default(),
            one_sided: Default::default(),
        };
        for (index, entry) in truth.entries.iter().enumerate() {
            let only_side = match (&entry.before, &entry.after) {
                (Some(node), None) => Some((BEFORE, node)),
                (None, Some(node)) => Some((AFTER, node)),
                _ => None,
            };
            if let Some((side, node)) = only_side
                && let Some((file, _)) = trees.side(side).get_key_value(&node.file)
            {
                resolved.one_sided[side]
                    .entry(file.as_str())
                    .or_default()
                    .push(index);
            }
            let outcome = match (&entry.before, &entry.after) {
                (Some(before), Some(after)) if is_match(entry.operation) => resolver
                    .node(BEFORE, before)
                    .and_then(|before| Ok((before, resolver.node(AFTER, after)?)))
                    .map(|(before, after)| {
                        let found = ResolvedMatch {
                            entry: index,
                            before,
                            after,
                        };
                        for side in [BEFORE, AFTER] {
                            resolved.partners[side]
                                .entry(found.key(side))
                                .or_default()
                                .push(resolved.matches.len());
                            resolved.matches_of[side]
                                .entry(found.key(side).0)
                                .or_default()
                                .push(resolved.matches.len());
                        }
                        resolved.matches.push(found);
                    }),
                (Some(node), None) | (None, Some(node))
                    if entry.before.is_some() == is_delete(entry.operation)
                        && !is_match(entry.operation) =>
                {
                    let side = if entry.before.is_some() {
                        BEFORE
                    } else {
                        AFTER
                    };
                    let with_children = matches!(
                        entry.operation,
                        HumanOperation::DeleteWithChildren | HumanOperation::InsertWithChildren
                    );
                    resolver.node(side, node).map(|(file, node)| {
                        resolved.removed[side]
                            .entry((file.path.as_str(), node.id()))
                            .or_insert((index, with_children));
                        if with_children {
                            resolved.removes_children[side].insert(file.path.as_str());
                        }
                    })
                }
                // `entry_sides_agree_with_operation` reports these.
                _ => Ok(()),
            };
            if let Err(error) = outcome {
                resolved.unresolved.push((index, format!("{error:#}")));
            }
        }
        resolved
    }

    /// The entry that removes `node`: its own, or an ancestor's that takes its children.
    fn removed_by(&self, side: usize, file: &'t str, node: Node<'t>) -> Option<usize> {
        if let Some((entry, _)) = self.removed[side].get(&(file, node.id())) {
            return Some(*entry);
        }
        if !self.removes_children[side].contains(file) {
            return None;
        }
        let mut current = node;
        while let Some(parent) = current.parent() {
            if let Some((entry, true)) = self.removed[side].get(&(file, parent.id())) {
                return Some(*entry);
            }
            current = parent;
        }
        None
    }

    /// The matches with an end in the before file `before` or in the after file `after`, as
    /// ascending indices into `matches`.
    fn touching(&self, before: &str, after: &str) -> Vec<usize> {
        merged(
            self.matches_of[BEFORE].get(before),
            self.matches_of[AFTER].get(after),
        )
    }

    /// Whether `node` has several partners, or a partner that has several.
    fn is_many_matched(&self, side: usize, key: NodeKey<'t>) -> bool {
        let Some(matches) = self.partners[side].get(&key) else {
            return false;
        };
        matches.len() > 1
            || matches
                .iter()
                .any(|&index| self.partners[1 - side][&self.matches[index].key(1 - side)].len() > 1)
    }
}

/// Every way `truth` contradicts itself or `trees`, in a stable order. Empty is a pass. The checks
/// are the module doc's list.
pub fn validate(truth: &HumanChangeset, trees: &ChangesetTrees) -> Vec<ChangesetViolation> {
    let resolved = Resolved::new(truth, trees);
    let mut violations = Vec::new();
    violations.extend(relations_are_well_formed(truth));
    violations.extend(referenced_files_exist(truth, trees));
    violations.extend(added_and_removed_files_have_no_counterpart(truth));
    violations.extend(same_and_renamed_relations_are_one_to_one(truth));
    violations.extend(split_and_merged_relations_share_a_file(truth));
    violations.extend(verdicts_are_on_pairs_without_trees(truth, trees));
    violations.extend(entry_sides_agree_with_operation(truth));
    violations.extend(entry_paths_resolve(&resolved));
    violations.extend(matched_pairs_are_recorded_once(truth, &resolved));
    violations.extend(removed_nodes_are_not_matched(truth, &resolved));
    violations.extend(painting_entries_are_well_formed(truth, trees));
    violations.extend(painted_ranges_do_not_overlap(truth, trees));
    violations.extend(solved_files_are_fully_marked(truth, trees, &resolved));
    violations.extend(single_file_invariants_hold_per_pair(
        truth, trees, &resolved,
    ));
    violations
}

fn describe_relation(relation: &FileRelation) -> String {
    format!(
        "{:?} {} -> {}",
        relation.relation,
        relation.before.as_deref().unwrap_or("(none)"),
        relation.after.as_deref().unwrap_or("(none)")
    )
}

/// How many counterparts the records give `path` on `side`, a file added or removed counting as
/// one (a pair recorded twice is `relations_are_well_formed`'s to report).
fn relation_count(truth: &HumanChangeset, side: usize, path: &str) -> usize {
    truth
        .files
        .iter()
        .filter(|relation| [&relation.before, &relation.after][side].as_deref() == Some(path))
        .map(|relation| [&relation.after, &relation.before][side])
        .collect::<HashSet<_>>()
        .len()
}

fn relations_are_well_formed(truth: &HumanChangeset) -> Vec<ChangesetViolation> {
    const CHECK: &str = "relations_are_well_formed";
    let mut violations = Vec::new();
    let mut seen = HashSet::new();
    for relation in &truth.files {
        let problem = match (relation.relation, &relation.before, &relation.after) {
            (FileRelationKind::Same, Some(before), Some(after)) => {
                (before != after).then_some("the two paths of a `same` record differ")
            }
            (FileRelationKind::Renamed, Some(before), Some(after)) => {
                (before == after).then_some("the two paths of a `renamed` record are equal")
            }
            (FileRelationKind::Split | FileRelationKind::Merged, Some(_), Some(_)) => None,
            (FileRelationKind::Added, None, Some(_)) => None,
            (FileRelationKind::Removed, Some(_), None) => None,
            (FileRelationKind::Added, ..) => Some("an `added` record has an `after` path only"),
            (FileRelationKind::Removed, ..) => Some("a `removed` record has a `before` path only"),
            _ => Some("the record needs both paths"),
        };
        if let Some(problem) = problem {
            violations.push(violation(
                CHECK,
                format!("{}: {problem}", describe_relation(relation)),
            ));
        }
        if !seen.insert((&relation.before, &relation.after)) {
            violations.push(violation(
                CHECK,
                format!(
                    "{}: the pair is recorded twice",
                    describe_relation(relation)
                ),
            ));
        }
    }
    violations
}

fn referenced_files_exist(
    truth: &HumanChangeset,
    trees: &ChangesetTrees,
) -> Vec<ChangesetViolation> {
    let mut named: BTreeSet<(usize, &str, &str)> = BTreeSet::new();
    for relation in &truth.files {
        for (side, path) in [&relation.before, &relation.after].into_iter().enumerate() {
            if let Some(path) = path {
                named.insert((side, path, "a relation"));
            }
        }
    }
    for entry in &truth.entries {
        for (side, node) in [&entry.before, &entry.after].into_iter().enumerate() {
            if let Some(node) = node {
                named.insert((side, &node.file, "an entry"));
            }
        }
    }
    for painting in &truth.text_mappings {
        for entry in &painting.entries {
            for (side, spans) in [&entry.before, &entry.after].into_iter().enumerate() {
                for span in spans {
                    named.insert((side, &span.file, "a painted range"));
                }
            }
        }
    }
    named
        .into_iter()
        .filter_map(|(side, path, what)| {
            let problem = match check_repo_path(path) {
                Err(error) => format!("{error}"),
                Ok(()) if !trees.side(side).contains_key(path) => {
                    format!("the {} side has no file {path}", SIDE_NAMES[side])
                }
                Ok(()) => return None,
            };
            Some(violation(
                "referenced_files_exist",
                format!("{what} names {path:?}: {problem}"),
            ))
        })
        .collect()
}

fn added_and_removed_files_have_no_counterpart(truth: &HumanChangeset) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for relation in &truth.files {
        let (side, path) = match (relation.relation, &relation.before, &relation.after) {
            (FileRelationKind::Added, _, Some(path)) => (AFTER, path),
            (FileRelationKind::Removed, Some(path), _) => (BEFORE, path),
            _ => continue,
        };
        if relation_count(truth, side, path) > 1 {
            violations.push(violation(
                "added_and_removed_files_have_no_counterpart",
                format!(
                    "{}: {path} is in another record too",
                    describe_relation(relation)
                ),
            ));
        }
    }
    violations
}

fn same_and_renamed_relations_are_one_to_one(truth: &HumanChangeset) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for relation in &truth.files {
        if !matches!(
            relation.relation,
            FileRelationKind::Same | FileRelationKind::Renamed
        ) {
            continue;
        }
        for (side, path) in [&relation.before, &relation.after].into_iter().enumerate() {
            if let Some(path) = path
                && relation_count(truth, side, path) > 1
            {
                violations.push(violation(
                    "same_and_renamed_relations_are_one_to_one",
                    format!(
                        "{}: {path} has another counterpart, which makes this a split or a merge",
                        describe_relation(relation)
                    ),
                ));
            }
        }
    }
    violations
}

fn split_and_merged_relations_share_a_file(truth: &HumanChangeset) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for relation in &truth.files {
        let (side, path, what) = match (relation.relation, &relation.before, &relation.after) {
            (FileRelationKind::Split, Some(path), _) => (BEFORE, path, "a split goes to"),
            (FileRelationKind::Merged, _, Some(path)) => (AFTER, path, "a merge comes from"),
            _ => continue,
        };
        if relation_count(truth, side, path) < 2 {
            violations.push(violation(
                "split_and_merged_relations_share_a_file",
                format!(
                    "{}: {what} at least two files, and {path} is in one record",
                    describe_relation(relation)
                ),
            ));
        }
    }
    violations
}

fn verdicts_are_on_pairs_without_trees(
    truth: &HumanChangeset,
    trees: &ChangesetTrees,
) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for relation in truth
        .files
        .iter()
        .filter(|relation| relation.verdict.is_some())
    {
        let has_tree = |side: usize, path: &Option<String>| {
            path.as_ref()
                .and_then(|path| trees.side(side).get(path))
                .is_some_and(|file| file.root().is_some())
        };
        let problem = if relation.before.is_none() || relation.after.is_none() {
            "a verdict judges a pair, and this record has one file"
        } else if has_tree(BEFORE, &relation.before) && has_tree(AFTER, &relation.after) {
            "both files have trees, so the entries are their truth, not a verdict"
        } else {
            continue;
        };
        violations.push(violation(
            "verdicts_are_on_pairs_without_trees",
            format!("{}: {problem}", describe_relation(relation)),
        ));
    }
    violations
}

fn describe_entry(index: usize, entry: &ChangesetEntry) -> String {
    let side = |node: &Option<NodeRef>| match node {
        Some(node) => format!("{}:{:?}", node.file, node.path),
        None => "(none)".to_string(),
    };
    format!(
        "entry {index} ({:?} {} -> {})",
        entry.operation,
        side(&entry.before),
        side(&entry.after)
    )
}

fn entry_sides_agree_with_operation(truth: &HumanChangeset) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for (index, entry) in truth.entries.iter().enumerate() {
        let wanted = match entry.operation {
            operation if is_match(operation) => (true, true),
            operation if is_delete(operation) => (true, false),
            _ => (false, true),
        };
        if (entry.before.is_some(), entry.after.is_some()) != wanted {
            let needs = match wanted {
                (true, true) => "a match has both sides",
                (true, false) => "a delete has a `before` side only",
                _ => "an insert has an `after` side only",
            };
            violations.push(violation(
                "entry_sides_agree_with_operation",
                format!("{}: {needs}", describe_entry(index, entry)),
            ));
        }
    }
    violations
}

fn entry_paths_resolve(resolved: &Resolved) -> Vec<ChangesetViolation> {
    resolved
        .unresolved
        .iter()
        .map(|(index, error)| violation("entry_paths_resolve", format!("entry {index}: {error}")))
        .collect()
}

fn matched_pairs_are_recorded_once(
    truth: &HumanChangeset,
    resolved: &Resolved,
) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    let mut seen: HashMap<(NodeKey, NodeKey), usize> = HashMap::new();
    for found in &resolved.matches {
        if let Some(first) = seen.insert((found.key(BEFORE), found.key(AFTER)), found.entry) {
            violations.push(violation(
                "matched_pairs_are_recorded_once",
                format!(
                    "{} pairs the nodes entry {first} already pairs",
                    describe_entry(found.entry, &truth.entries[found.entry])
                ),
            ));
        }
    }
    violations
}

fn removed_nodes_are_not_matched(
    truth: &HumanChangeset,
    resolved: &Resolved,
) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for found in &resolved.matches {
        for (side, (file, node)) in [found.before, found.after].into_iter().enumerate() {
            if let Some(remover) = resolved.removed_by(side, file.path.as_str(), node) {
                violations.push(violation(
                    "removed_nodes_are_not_matched",
                    format!(
                        "{} matches a node that {} {}",
                        describe_entry(found.entry, &truth.entries[found.entry]),
                        describe_entry(remover, &truth.entries[remover]),
                        ["deletes", "inserts"][side]
                    ),
                ));
            }
        }
    }
    violations
}

fn painting_entries_are_well_formed(
    truth: &HumanChangeset,
    trees: &ChangesetTrees,
) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for painting in &truth.text_mappings {
        for (index, entry) in painting.entries.iter().enumerate() {
            let mut problems = Vec::new();
            if let Err(error) = entry.verdict(trees) {
                problems.push(format!("{error:#}"));
            }
            for span in entry.before.iter().chain(&entry.after) {
                if span.range.is_empty() {
                    problems.push(format!("an empty range in {}", span.file));
                }
            }
            for problem in problems {
                violations.push(violation(
                    "painting_entries_are_well_formed",
                    format!(
                        "painting '{}' entry {index} ({:?}): {problem}",
                        painting.name, entry.operation
                    ),
                ));
            }
        }
    }
    violations
}

/// The ranges `painting` has in `file` on `side`, as a single-file painting of that side alone:
/// one entry per changeset entry, so "which entry" means the same in both.
fn painting_of_file(painting: &ChangesetTextMapping, side: usize, file: &str) -> NamedTextMapping {
    let entries = painting
        .entries
        .iter()
        .map(|entry| {
            let spans: Vec<HumanTextSpan> = [&entry.before, &entry.after][side]
                .iter()
                .filter(|span| span.file == file)
                .map(|span| span.range)
                .collect();
            let (before, after) = if side == BEFORE {
                (spans, Vec::new())
            } else {
                (Vec::new(), spans)
            };
            HumanTextEntry {
                operation: entry.operation,
                before,
                after,
            }
        })
        .collect();
    NamedTextMapping {
        name: painting.name.clone(),
        mapping: HumanTextMapping { entries },
    }
}

fn painted_ranges_do_not_overlap(
    truth: &HumanChangeset,
    trees: &ChangesetTrees,
) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    let nothing = Code::default();
    for painting in &truth.text_mappings {
        for side in [BEFORE, AFTER] {
            for (path, file) in trees.side(side) {
                let Some(code) = &file.code else { continue };
                let of_file = painting_of_file(painting, side, path);
                let (before, after) = if side == BEFORE {
                    (code, &nothing)
                } else {
                    (&nothing, code)
                };
                for found in overlaps_in_one_file(&of_file, before, after) {
                    violations.push(violation(
                        "painted_ranges_do_not_overlap",
                        format!("{path}: {}", found.message),
                    ));
                }
            }
        }
    }
    violations
}

fn solved_files_are_fully_marked(
    truth: &HumanChangeset,
    trees: &ChangesetTrees,
    resolved: &Resolved,
) -> Vec<ChangesetViolation> {
    let mut violations = Vec::new();
    for side in [BEFORE, AFTER] {
        for (path, file) in trees.side(side) {
            let mut relations = truth
                .files
                .iter()
                .filter(|relation| [&relation.before, &relation.after][side].as_ref() == Some(path))
                .peekable();
            if relations.peek().is_none() || !relations.all(|relation| relation.solved) {
                continue;
            }
            let Some(root) = file.root() else { continue };
            let mut unmarked = Vec::new();
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                let key = (path.as_str(), node.id());
                if let Some((_, true)) = resolved.removed[side].get(&key) {
                    continue;
                }
                if !resolved.partners[side].contains_key(&key)
                    && !resolved.removed[side].contains_key(&key)
                {
                    unmarked.push(node);
                }
                let mut cursor = node.walk();
                let children: Vec<Node> = node.children(&mut cursor).collect();
                stack.extend(children.into_iter().rev());
            }
            if let Some(first) = unmarked.first() {
                violations.push(violation(
                    "solved_files_are_fully_marked",
                    format!(
                        "{path} on the {} side is solved, but {} of its nodes have no entry; the \
                         first is {:?}",
                        SIDE_NAMES[side],
                        unmarked.len(),
                        super::path_for_node(*first)
                    ),
                ));
            }
        }
    }
    violations
}

/// The single-file invariants that judge one entry by its own two subtrees, which is all that
/// holds between files a painting cannot be read across. See the module doc.
const ENTRY_LOCAL_INVARIANTS: &[u8] = &[14, 15];

/// [`painted_ranges_do_not_overlap`] here: the single-file rule sees one pair at a time.
const INVARIANT_REPLACED_HERE: u8 = 7;

fn single_file_invariants_hold_per_pair(
    truth: &HumanChangeset,
    trees: &ChangesetTrees,
    resolved: &Resolved,
) -> Vec<ChangesetViolation> {
    const CHECK: &str = "single_file_invariants_hold_per_pair";
    let mut violations = Vec::new();
    let mut by_pair: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (index, found) in resolved.matches.iter().enumerate() {
        by_pair
            .entry((found.before.0.path.as_str(), found.after.0.path.as_str()))
            .or_default()
            .push(index);
    }

    let mut report = |before: &str,
                      after: &str,
                      mapping: &HumanMapping,
                      keep: &dyn Fn(u8) -> bool,
                      many_matched: Option<&[Vec<HumanTextSpan>; 2]>| {
        let (Some(before_code), Some(after_code)) = (
            trees.before.get(before).and_then(|file| file.code.as_ref()),
            trees.after.get(after).and_then(|file| file.code.as_ref()),
        ) else {
            return;
        };
        let found = match ground_truth_invariant_violations_for(mapping, before_code, after_code) {
            Ok(found) => found,
            Err(error) => {
                violations.push(violation(
                    CHECK,
                    format!("{before} -> {after}: the invariants do not run: {error:#}"),
                ));
                return;
            }
        };
        for found in found {
            let at_many_matched = many_matched.is_some_and(|spans| {
                found.sites.iter().any(|site| {
                    spans[site.side]
                        .iter()
                        .any(|span| spans_overlap(*span, site.span))
                })
            });
            if keep(found.invariant) && !at_many_matched {
                violations.push(ChangesetViolation {
                    check: CHECK,
                    invariant: Some(found.invariant),
                    message: format!("{before} -> {after}: {found}"),
                    pair: Some((before.to_string(), after.to_string())),
                    sites: found
                        .sites
                        .iter()
                        .map(|site| (site.side, site.span))
                        .collect(),
                });
            }
        }
    };

    // The pairs a painting can be read across: one file, before and after.
    let mut full_pairs: BTreeSet<(&str, &str)> = BTreeSet::new();
    for relation in &truth.files {
        let (Some(before), Some(after)) = (&relation.before, &relation.after) else {
            continue;
        };
        let one_to_one = matches!(
            relation.relation,
            FileRelationKind::Same | FileRelationKind::Renamed
        ) && relation_count(truth, BEFORE, before) == 1
            && relation_count(truth, AFTER, after) == 1;
        if one_to_one && full_pairs.insert((before, after)) {
            let (view, dropped) = pair_view(truth, resolved, before, after);
            let many_matched = many_matched_spans(resolved, before, after);
            report(
                before,
                after,
                &view,
                &|invariant| invariant != INVARIANT_REPLACED_HERE,
                Some(&many_matched),
            );
            // The partners the view dropped are still entries, and still judged as entries.
            by_pair.insert((before, after), dropped);
        }
    }

    for ((before, after), mut matches) in by_pair {
        // Each layer holds a node once, as a single-file mapping must.
        while !matches.is_empty() {
            let mut used: [HashSet<usize>; 2] = Default::default();
            let (layer, rest): (Vec<usize>, Vec<usize>) = matches.into_iter().partition(|&index| {
                let found = &resolved.matches[index];
                if used[BEFORE].contains(&found.before.1.id())
                    || used[AFTER].contains(&found.after.1.id())
                {
                    return false;
                }
                used[BEFORE].insert(found.before.1.id());
                used[AFTER].insert(found.after.1.id())
            });
            let mapping = HumanMapping {
                entries: layer
                    .iter()
                    .map(|&index| single_file_entry(&truth.entries[resolved.matches[index].entry]))
                    .collect(),
                ..Default::default()
            };
            report(
                before,
                after,
                &mapping,
                &|invariant| ENTRY_LOCAL_INVARIANTS.contains(&invariant),
                None,
            );
            matches = rest;
        }
    }
    violations
}

fn single_file_entry(entry: &ChangesetEntry) -> HumanMappingEntry {
    HumanMappingEntry {
        operation: entry.operation,
        before_path: entry.before.as_ref().map(|node| node.path.clone()),
        after_path: entry.after.as_ref().map(|node| node.path.clone()),
    }
}

fn spans_overlap(a: HumanTextSpan, b: HumanTextSpan) -> bool {
    (a.start_row, a.start_column) < (b.end_row, b.end_column)
        && (b.start_row, b.start_column) < (a.end_row, a.end_column)
}

/// Where the many-matched nodes of the pair's two files are, `[before, after]`.
fn many_matched_spans(resolved: &Resolved, before: &str, after: &str) -> [Vec<HumanTextSpan>; 2] {
    let mut spans: [Vec<HumanTextSpan>; 2] = Default::default();
    for index in resolved.touching(before, after) {
        let found = &resolved.matches[index];
        for (side, (file, node)) in [found.before, found.after].into_iter().enumerate() {
            if file.path == [before, after][side]
                && resolved.is_many_matched(side, (file.path.as_str(), node.id()))
            {
                spans[side].push(HumanTextSpan {
                    start_row: node.start_position().row,
                    start_column: node.start_position().column,
                    end_row: node.end_position().row,
                    end_column: node.end_position().column,
                });
            }
        }
    }
    spans
}

/// The changeset truth as it reads from inside the pair `before`/`after` (see the module doc),
/// and the matches between the two files it had to leave out because a node was already paired.
fn pair_view(
    truth: &HumanChangeset,
    resolved: &Resolved,
    before: &str,
    after: &str,
) -> (HumanMapping, Vec<usize>) {
    let files = [before, after];
    let mut entries = Vec::new();
    let mut dropped = Vec::new();
    let mut paired: [HashSet<usize>; 2] = Default::default();
    let touching = resolved.touching(before, after);
    for &index in &touching {
        let found = &resolved.matches[index];
        if found.before.0.path != before || found.after.0.path != after {
            continue;
        }
        if paired[BEFORE].contains(&found.before.1.id())
            || paired[AFTER].contains(&found.after.1.id())
        {
            dropped.push(index);
            continue;
        }
        paired[BEFORE].insert(found.before.1.id());
        paired[AFTER].insert(found.after.1.id());
        entries.push(single_file_entry(&truth.entries[found.entry]));
    }
    // A node whose partners are all elsewhere left this pair, or came into it.
    let mut gone: [HashSet<usize>; 2] = Default::default();
    for &index in &touching {
        let found = &resolved.matches[index];
        for (side, (file, node)) in [found.before, found.after].into_iter().enumerate() {
            if file.path != files[side]
                || paired[side].contains(&node.id())
                || !gone[side].insert(node.id())
            {
                continue;
            }
            let entry = &truth.entries[found.entry];
            entries.push(if side == BEFORE {
                HumanMappingEntry {
                    operation: HumanOperation::Delete,
                    before_path: entry.before.as_ref().map(|node| node.path.clone()),
                    after_path: None,
                }
            } else {
                HumanMappingEntry {
                    operation: HumanOperation::Insert,
                    before_path: None,
                    after_path: entry.after.as_ref().map(|node| node.path.clone()),
                }
            });
        }
    }
    let one_sided = merged(
        resolved.one_sided[BEFORE].get(before),
        resolved.one_sided[AFTER].get(after),
    );
    for entry in one_sided.into_iter().map(|index| &truth.entries[index]) {
        if !is_match(entry.operation) {
            entries.push(single_file_entry(entry));
        }
    }

    let text_mappings = truth
        .text_mappings
        .iter()
        .map(|painting| {
            let entries = painting
                .entries
                .iter()
                .filter_map(|entry| {
                    let of = |spans: &[FileSpan], file: &str| -> Vec<HumanTextSpan> {
                        spans
                            .iter()
                            .filter(|span| span.file == file)
                            .map(|span| span.range)
                            .collect()
                    };
                    let (before, after) = (of(&entry.before, before), of(&entry.after, after));
                    let operation = match (before.is_empty(), after.is_empty()) {
                        (true, true) => return None,
                        (false, false) => entry.operation,
                        (false, true) => HumanTextOperation::Delete,
                        (true, false) => HumanTextOperation::Insert,
                    };
                    Some(HumanTextEntry {
                        operation,
                        before,
                        after,
                    })
                })
                .collect();
            NamedTextMapping {
                name: painting.name.clone(),
                mapping: HumanTextMapping { entries },
            }
        })
        .collect();

    (
        HumanMapping {
            entries,
            text_mappings,
            ..Default::default()
        },
        dropped,
    )
}

// ─── A fixture, whole ───────────────────────────────────────────────────────────────────────

/// One changeset fixture, loaded.
pub struct Changeset {
    pub dir: PathBuf,
    pub source: ChangesetSource,
    pub trees: ChangesetTrees,
    /// `None` until someone has started solving it.
    pub truth: Option<HumanChangeset>,
}

impl Changeset {
    pub fn load(dir: &Path) -> Result<Self> {
        let trees = ChangesetTrees::load(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            source: ChangesetSource::load(dir)?,
            truth: HumanChangeset::load(dir, &trees)?,
            trees,
        })
    }

    pub fn load_by_name(name: &str) -> Result<Self> {
        let dir = changeset_dir(name).with_context(|| format!("no changeset called '{name}'"))?;
        Self::load(&dir)
    }
}

/// `src/test/data/samples/changesets/`: the samples nobody has promoted yet (git-ignored).
pub fn changeset_samples_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join("samples")
        .join("changesets")
}

/// What is wrong with the changeset directory `dir`, corpus fixture or sample, short of its
/// truth being unfinished: the inventory every fixture passes. `source.json` fits the trees and
/// its kind ([`source_violations`]); there is a `README.md` saying where the files are from and
/// under what licence, and it does not claim the changeset is unsolved (whether it is solved is
/// [`HumanChangeset::is_solved`]'s to say, and a sentence in a README goes stale); a truth in
/// progress may be unsolved, but never invalid. An error when the directory does not load.
pub fn fixture_violations(dir: &Path) -> Result<Vec<String>> {
    let changeset = Changeset::load(dir)?;
    let mut violations = source_violations(&changeset.source, &changeset.trees);
    match fs::read_to_string(dir.join("README.md")) {
        Err(_) => violations.push(
            "no README.md saying where the files are from and under what licence".to_string(),
        ),
        Ok(readme) => {
            if !readme.to_lowercase().contains("licen") {
                violations.push("README.md does not name a licence".to_string());
            }
            if readme.to_lowercase().contains("not solved") {
                violations.push(
                    "README.md says whether the changeset is solved, which goes stale:                      human_changeset.json says it"
                        .to_string(),
                );
            }
        }
    }
    if let Some(truth) = &changeset.truth {
        violations.extend(
            validate(truth, &changeset.trees)
                .iter()
                .map(ToString::to_string),
        );
    }
    Ok(violations)
}

/// What a solved changeset's stub asserts while there is no engine to grade: the fixture loads,
/// its `source.json` fits its trees, and its truth is whole, valid, and saved as
/// [`HumanChangeset::to_json`] writes it.
pub fn assert_solved_ground_truth(name: &str) -> Result<()> {
    let dir = changeset_dir(name).with_context(|| format!("no changeset called '{name}'"))?;
    assert_solved_fixture(&dir)
}

/// [`assert_solved_ground_truth`] of the changeset directory `dir`, wherever it is: what a stub
/// would assert of a fixture promoted there.
pub fn assert_solved_fixture(dir: &Path) -> Result<()> {
    let changeset = Changeset::load(dir)?;
    let name = dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let source = source_violations(&changeset.source, &changeset.trees);
    ensure!(
        source.is_empty(),
        "'{name}': source.json does not fit the trees:\n  {}",
        source.join("\n  ")
    );
    let truth = changeset
        .truth
        .as_ref()
        .with_context(|| format!("'{name}' has no {TRUTH_FILE_NAME}"))?;
    let violations = validate(truth, &changeset.trees);
    ensure!(
        violations.is_empty(),
        "'{name}': {} violation(s) in the truth:\n  {}",
        violations.len(),
        violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    ensure!(
        truth.is_solved(&changeset.trees),
        "'{name}' is not solved: a record is unsolved, or a file is in no record"
    );
    let on_disk = fs::read_to_string(changeset.dir.join(TRUTH_FILE_NAME))?;
    ensure!(
        on_disk == truth.to_json(&changeset.trees)?,
        "'{name}': {TRUTH_FILE_NAME} is not in the form saving writes"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trees(before: &[(&str, &str)], after: &[(&str, &str)]) -> ChangesetTrees {
        fn bytes<'a>(files: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a [u8])> {
            files
                .iter()
                .map(|(path, text)| (*path, text.as_bytes()))
                .collect()
        }
        ChangesetTrees::from_files(&bytes(before), &bytes(after)).unwrap()
    }

    fn relation(
        before: Option<&str>,
        after: Option<&str>,
        kind: FileRelationKind,
        solved: bool,
    ) -> FileRelation {
        FileRelation {
            before: before.map(str::to_string),
            after: after.map(str::to_string),
            relation: kind,
            solved,
            verdict: None,
        }
    }

    fn same(path: &str) -> FileRelation {
        relation(Some(path), Some(path), FileRelationKind::Same, true)
    }

    fn span(file: &str, row: usize, start: usize, end: usize) -> FileSpan {
        FileSpan {
            file: file.to_string(),
            range: HumanTextSpan {
                start_row: row,
                start_column: start,
                end_row: row,
                end_column: end,
            },
        }
    }

    fn painting(entries: Vec<ChangesetTextEntry>) -> ChangesetTextMapping {
        ChangesetTextMapping {
            name: "Only one solution".to_string(),
            entries,
        }
    }

    fn text_entry(
        operation: HumanTextOperation,
        before: Vec<FileSpan>,
        after: Vec<FileSpan>,
    ) -> ChangesetTextEntry {
        ChangesetTextEntry {
            operation,
            before,
            after,
        }
    }

    fn subtree(
        trees: &ChangesetTrees,
        before: (&str, &[&str]),
        after: (&str, &[&str]),
    ) -> Vec<ChangesetEntry> {
        HumanChangeset::identical_subtree(
            trees,
            &NodeRef::new(before.0, before.1),
            &NodeRef::new(after.0, after.1),
        )
        .unwrap()
    }

    fn roots(file: &str) -> ChangesetEntry {
        ChangesetEntry::matched(
            HumanOperation::MatchButNotIdentical,
            NodeRef::new(file, &[]),
            NodeRef::new(file, &[]),
        )
    }

    /// `g` moves from `a.rs` to `b.rs`, untouched.
    fn moved_trees() -> ChangesetTrees {
        trees(
            &[("a.rs", "fn f() {}\nfn g() {}\n"), ("b.rs", "fn h() {}\n")],
            &[("a.rs", "fn f() {}\n"), ("b.rs", "fn h() {}\nfn g() {}\n")],
        )
    }

    /// The whole truth of [`moved_trees`].
    fn moved_truth(trees: &ChangesetTrees) -> HumanChangeset {
        let mut entries = vec![roots("a.rs"), roots("b.rs")];
        entries.extend(subtree(
            trees,
            ("a.rs", &["function_item:1"]),
            ("a.rs", &["function_item:1"]),
        ));
        entries.extend(subtree(
            trees,
            ("b.rs", &["function_item:1"]),
            ("b.rs", &["function_item:1"]),
        ));
        entries.extend(subtree(
            trees,
            ("a.rs", &["function_item:2"]),
            ("b.rs", &["function_item:2"]),
        ));
        HumanChangeset {
            files: vec![same("a.rs"), same("b.rs")],
            entries,
            text_mappings: vec![painting(vec![text_entry(
                HumanTextOperation::Match,
                vec![span("a.rs", 1, 0, 9)],
                vec![span("b.rs", 1, 0, 9)],
            )])],
        }
    }

    fn checks(violations: &[ChangesetViolation]) -> Vec<&'static str> {
        let mut checks: Vec<_> = violations.iter().map(|found| found.check).collect();
        checks.dedup();
        checks
    }

    fn assert_only(truth: &HumanChangeset, trees: &ChangesetTrees, check: &str) {
        let violations = validate(truth, trees);
        assert_eq!(checks(&violations), [check], "{violations:#?}");
    }

    #[test]
    fn a_cross_file_move_is_valid_and_solved() {
        let trees = moved_trees();
        let truth = moved_truth(&trees);
        assert_eq!(validate(&truth, &trees), []);
        assert!(truth.is_solved(&trees));
    }

    #[test]
    fn repo_paths_cannot_leave_the_fixture() {
        for good in ["a.rs", "src/a.rs", "a b/c-d/.hidden", "src/..a"] {
            check_repo_path(good).unwrap();
        }
        for bad in [
            "",
            "/etc/passwd",
            "../a.rs",
            "src/../../a.rs",
            "src/./a.rs",
            "src//a.rs",
            "src/",
            "src\\a.rs",
            "C:/a.rs",
            "a\0b",
        ] {
            assert!(check_repo_path(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn a_directory_loads_with_real_paths_languages_and_binaries() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let write = |path: &str, bytes: &[u8]| -> Result<()> {
            let path = dir.path().join(path);
            fs::create_dir_all(path.parent().unwrap())?;
            Ok(fs::write(path, bytes)?)
        };
        write("before/src/deep/a.py.test", b"x = 1\n")?;
        write(
            "before/logo.png.test",
            &[0x89, b'P', b'N', b'G', 0xff, 0x00],
        )?;
        write("after/src/a.go.test", b"package a\n")?;
        let loaded = ChangesetTrees::load(dir.path())?;
        assert_eq!(
            loaded.before.keys().collect::<Vec<_>>(),
            ["logo.png", "src/deep/a.py"]
        );
        let language = |file: &ChangesetFile| file.code.as_ref().unwrap().metadata.language;
        assert_eq!(
            language(&loaded.before["src/deep/a.py"]),
            Some(crate::code::Language::Python)
        );
        assert_eq!(
            language(&loaded.after["src/a.go"]),
            Some(crate::code::Language::Go)
        );
        assert!(loaded.before["src/deep/a.py"].root().is_some());
        assert!(loaded.before["logo.png"].code.is_none());
        assert_eq!(loaded.path_count(), 3);

        // Only `.test` files belong under a side.
        write("after/notes.txt", b"hello")?;
        assert!(ChangesetTrees::load(dir.path()).is_err());
        Ok(())
    }

    #[test]
    fn a_side_without_a_directory_is_empty() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("after"))?;
        fs::write(dir.path().join("after/a.rs.test"), "fn f() {}\n")?;
        let loaded = ChangesetTrees::load(dir.path())?;
        assert!(loaded.before.is_empty());
        assert_eq!(loaded.after.len(), 1);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_is_refused() -> Result<()> {
        let outside = tempfile::tempdir()?;
        fs::write(outside.path().join("secret.rs.test"), "fn s() {}\n")?;
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("before"))?;
        std::os::unix::fs::symlink(
            outside.path().join("secret.rs.test"),
            dir.path().join("before/a.rs.test"),
        )?;
        assert!(ChangesetTrees::load(dir.path()).is_err());
        fs::remove_file(dir.path().join("before/a.rs.test"))?;
        std::os::unix::fs::symlink(outside.path(), dir.path().join("before/linked"))?;
        assert!(ChangesetTrees::load(dir.path()).is_err());
        Ok(())
    }

    #[test]
    fn every_referenced_file_exists_on_its_side() {
        let trees = moved_trees();
        let mut truth = moved_truth(&trees);
        truth
            .files
            .push(relation(None, Some("c.rs"), FileRelationKind::Added, true));
        truth.entries.push(ChangesetEntry::deleted(
            HumanOperation::Delete,
            NodeRef::new("../a.rs", &[]),
        ));
        truth.text_mappings[0].entries[0].after[0].file = "gone.rs".to_string();
        let violations = validate(&truth, &trees);
        let messages: Vec<_> = violations
            .iter()
            .filter(|found| found.check == "referenced_files_exist")
            .map(|found| found.message.as_str())
            .collect();
        assert_eq!(messages.len(), 3, "{violations:#?}");
        assert!(messages.iter().any(|message| message.contains("c.rs")));
        assert!(messages.iter().any(|message| message.contains("`..`")));
        assert!(messages.iter().any(|message| message.contains("gone.rs")));
    }

    #[test]
    fn every_entry_path_resolves_in_its_own_files_tree() {
        let trees = moved_trees();
        let mut truth = moved_truth(&trees);
        // `function_item:2` exists in after/b.rs and not in after/a.rs.
        truth.entries.push(ChangesetEntry::inserted(
            HumanOperation::InsertWithChildren,
            NodeRef::new("a.rs", &["function_item:2"]),
        ));
        assert_only(&truth, &trees, "entry_paths_resolve");
    }

    #[test]
    fn an_entrys_operation_agrees_with_its_sides() {
        let trees = moved_trees();
        for (operation, before, after) in [
            (HumanOperation::Identical, true, false),
            (HumanOperation::Update, false, true),
            (HumanOperation::Delete, true, true),
            (HumanOperation::DeleteWithChildren, false, true),
            (HumanOperation::Insert, true, false),
            (HumanOperation::InsertWithChildren, true, true),
        ] {
            let mut truth = moved_truth(&trees);
            let node = NodeRef::new("b.rs", &["function_item:1", "identifier:1"]);
            truth.entries.push(ChangesetEntry {
                operation,
                before: before.then(|| node.clone()),
                after: after.then(|| node.clone()),
            });
            assert_only(&truth, &trees, "entry_sides_agree_with_operation");
        }
    }

    #[test]
    fn an_added_or_removed_file_has_no_counterpart() {
        let trees = trees(&[("old.rs", "fn f() {}\n")], &[("new.rs", "fn f() {}\n")]);
        let mut truth = HumanChangeset {
            files: vec![
                relation(Some("old.rs"), None, FileRelationKind::Removed, false),
                relation(None, Some("new.rs"), FileRelationKind::Added, false),
            ],
            ..Default::default()
        };
        assert_eq!(validate(&truth, &trees), []);
        truth.files.push(relation(
            Some("old.rs"),
            Some("new.rs"),
            FileRelationKind::Split,
            false,
        ));
        let violations = validate(&truth, &trees);
        assert_eq!(
            violations
                .iter()
                .filter(|found| found.check == "added_and_removed_files_have_no_counterpart")
                .count(),
            2,
            "{violations:#?}"
        );
    }

    #[test]
    fn an_added_file_may_still_hold_a_copy_of_a_node() {
        let trees = trees(
            &[("a.rs", "fn f() {}\n")],
            &[("a.rs", "fn f() {}\n"), ("new.rs", "fn f() {}\n")],
        );
        let mut entries = subtree(&trees, ("a.rs", &[]), ("a.rs", &[]));
        entries.extend(subtree(
            &trees,
            ("a.rs", &["function_item:1"]),
            ("new.rs", &["function_item:1"]),
        ));
        entries.push(ChangesetEntry::inserted(
            HumanOperation::Insert,
            NodeRef::new("new.rs", &[]),
        ));
        let truth = HumanChangeset {
            files: vec![
                same("a.rs"),
                relation(None, Some("new.rs"), FileRelationKind::Added, true),
            ],
            entries,
            text_mappings: Vec::new(),
        };
        assert_eq!(validate(&truth, &trees), []);
        assert!(truth.is_solved(&trees));
    }

    #[test]
    fn relations_fit_their_kind() {
        let trees = moved_trees();
        let truth = |files: Vec<FileRelation>| HumanChangeset {
            files,
            ..Default::default()
        };
        use FileRelationKind::*;
        for bad in [
            relation(Some("a.rs"), Some("b.rs"), Same, false),
            relation(Some("a.rs"), Some("a.rs"), Renamed, false),
            relation(Some("a.rs"), None, Same, false),
            relation(Some("a.rs"), None, Added, false),
            relation(None, Some("a.rs"), Removed, false),
            relation(None, None, Split, false),
        ] {
            let violations = validate(&truth(vec![bad]), &trees);
            assert!(
                checks(&violations).contains(&"relations_are_well_formed"),
                "{violations:#?}"
            );
        }
        assert_only(
            &truth(vec![
                relation(Some("a.rs"), Some("b.rs"), Renamed, false),
                relation(Some("a.rs"), Some("b.rs"), Renamed, false),
            ]),
            &trees,
            "relations_are_well_formed",
        );
        // Two counterparts make a split, whatever the paths.
        assert_only(
            &truth(vec![
                relation(Some("a.rs"), Some("a.rs"), Same, false),
                relation(Some("a.rs"), Some("b.rs"), Split, false),
            ]),
            &trees,
            "same_and_renamed_relations_are_one_to_one",
        );
        assert_only(
            &truth(vec![relation(Some("a.rs"), Some("b.rs"), Merged, false)]),
            &trees,
            "split_and_merged_relations_share_a_file",
        );
        for good in [
            vec![
                relation(Some("a.rs"), Some("a.rs"), Split, false),
                relation(Some("a.rs"), Some("b.rs"), Split, false),
            ],
            vec![
                relation(Some("a.rs"), Some("b.rs"), Merged, false),
                relation(Some("b.rs"), Some("b.rs"), Merged, false),
            ],
            // Two files swapping contents.
            vec![
                relation(Some("a.rs"), Some("b.rs"), Renamed, false),
                relation(Some("b.rs"), Some("a.rs"), Renamed, false),
            ],
        ] {
            assert_eq!(validate(&truth(good), &trees), []);
        }
    }

    #[test]
    fn a_verdict_is_for_a_pair_without_trees() {
        let trees = ChangesetTrees::from_files(
            &[("a.rs", b"fn f() {}\n"), ("img/l.png", &[0xff, 0xfe, 0x00])],
            &[("a.rs", b"fn f() {}\n"), ("art/l.png", &[0xff, 0xfd, 0x00])],
        )
        .unwrap();
        let verdict: Judgement = serde_json::from_str("\"edited\"").unwrap();
        let mut picture = relation(
            Some("img/l.png"),
            Some("art/l.png"),
            FileRelationKind::Renamed,
            true,
        );
        picture.verdict = Some(verdict.clone());
        let mut truth = HumanChangeset {
            files: vec![same("a.rs"), picture],
            entries: subtree(&trees, ("a.rs", &[]), ("a.rs", &[])),
            text_mappings: Vec::new(),
        };
        assert_eq!(validate(&truth, &trees), []);
        assert!(truth.is_solved(&trees));
        let json = truth.to_json(&trees).unwrap();
        assert!(json.contains("\"verdict\": \"edited\""), "{json}");
        assert_eq!(HumanChangeset::from_json(&json, &trees).unwrap(), truth);

        truth.files[0].verdict = Some(verdict);
        assert_only(&truth, &trees, "verdicts_are_on_pairs_without_trees");
    }

    #[test]
    fn nodes_of_a_same_file_may_match_nodes_in_other_files() {
        // The move is exactly this: both files are `same`, and `g` crosses.
        let trees = moved_trees();
        let truth = moved_truth(&trees);
        assert!(truth.entries.iter().any(|entry| {
            entry.before.as_ref().map(|node| node.file.as_str()) == Some("a.rs")
                && entry.after.as_ref().map(|node| node.file.as_str()) == Some("b.rs")
        }));
        assert_eq!(validate(&truth, &trees), []);
    }

    #[test]
    fn one_node_may_match_several() {
        // `f` stays in a.rs and is copied to b.rs: one before node, two after nodes.
        let trees = trees(
            &[("a.rs", "fn f() {}\n"), ("b.rs", "fn h() {}\n")],
            &[("a.rs", "fn f() {}\n"), ("b.rs", "fn h() {}\nfn f() {}\n")],
        );
        let mut entries = vec![roots("b.rs")];
        entries.extend(subtree(&trees, ("a.rs", &[]), ("a.rs", &[])));
        entries.extend(subtree(
            &trees,
            ("b.rs", &["function_item:1"]),
            ("b.rs", &["function_item:1"]),
        ));
        entries.extend(subtree(
            &trees,
            ("a.rs", &["function_item:1"]),
            ("b.rs", &["function_item:2"]),
        ));
        let mut truth = HumanChangeset {
            files: vec![same("a.rs"), same("b.rs")],
            entries,
            text_mappings: vec![painting(vec![text_entry(
                HumanTextOperation::Match,
                vec![span("a.rs", 0, 0, 9)],
                vec![span("b.rs", 1, 0, 9)],
            )])],
        };
        assert_eq!(validate(&truth, &trees), []);
        assert!(truth.is_solved(&trees));

        // And several may match one: two identical functions folded into one.
        let folded = super::tests::trees(
            &[("a.rs", "fn f() {}\nfn f() {}\n")],
            &[("a.rs", "fn f() {}\n")],
        );
        let mut entries = vec![roots("a.rs")];
        for before in ["function_item:1", "function_item:2"] {
            entries.extend(subtree(
                &folded,
                ("a.rs", &[before]),
                ("a.rs", &["function_item:1"]),
            ));
        }
        truth = HumanChangeset {
            files: vec![same("a.rs")],
            entries,
            text_mappings: Vec::new(),
        };
        assert_eq!(validate(&truth, &folded), []);
    }

    #[test]
    fn a_removed_node_is_not_also_matched() {
        let trees = moved_trees();
        let g_name = &["function_item:2", "identifier:1"];
        for (check_side, entry) in [
            // `g` is matched into b.rs and deleted.
            (
                "deletes",
                ChangesetEntry::deleted(HumanOperation::Delete, NodeRef::new("a.rs", g_name)),
            ),
            // ... or sits inside a subtree that is deleted whole.
            (
                "deletes",
                ChangesetEntry::deleted(
                    HumanOperation::DeleteWithChildren,
                    NodeRef::new("a.rs", &["function_item:2"]),
                ),
            ),
            (
                "inserts",
                ChangesetEntry::inserted(HumanOperation::Insert, NodeRef::new("b.rs", g_name)),
            ),
            (
                "inserts",
                ChangesetEntry::inserted(
                    HumanOperation::InsertWithChildren,
                    NodeRef::new("b.rs", &["function_item:2"]),
                ),
            ),
        ] {
            let mut truth = moved_truth(&trees);
            // No painting: this is about the entries.
            truth.text_mappings.clear();
            truth.entries.push(entry);
            let violations: Vec<_> = validate(&truth, &trees)
                .into_iter()
                .filter(|found| found.check == "removed_nodes_are_not_matched")
                .collect();
            assert!(!violations.is_empty());
            assert!(
                violations
                    .iter()
                    .all(|found| found.message.ends_with(check_side)),
                "{violations:#?}"
            );
        }
    }

    #[test]
    fn a_pair_of_nodes_is_recorded_once() {
        let trees = moved_trees();
        let mut truth = moved_truth(&trees);
        let mut again = roots("a.rs");
        again.operation = HumanOperation::Identical;
        truth.entries.push(again);
        let violations = validate(&truth, &trees);
        assert!(
            checks(&violations).contains(&"matched_pairs_are_recorded_once"),
            "{violations:#?}"
        );
    }

    #[test]
    fn a_painted_range_lies_within_its_file_and_a_match_has_both_ends() {
        let trees = moved_trees();
        use HumanTextOperation::*;
        for (operation, before, after) in [
            // A Match (a move) with one end.
            (Match, vec![span("a.rs", 1, 0, 9)], vec![]),
            (Match, vec![], vec![span("b.rs", 1, 0, 9)]),
            (
                Delete,
                vec![span("a.rs", 1, 0, 9)],
                vec![span("b.rs", 1, 0, 9)],
            ),
            (Insert, vec![], vec![]),
            // Past the end of the row, past the end of the file, and empty.
            (Delete, vec![span("a.rs", 1, 0, 10)], vec![]),
            (Insert, vec![], vec![span("b.rs", 7, 0, 1)]),
            (Delete, vec![span("a.rs", 1, 3, 3)], vec![]),
        ] {
            let mut truth = moved_truth(&trees);
            truth.text_mappings = vec![painting(vec![text_entry(operation, before, after)])];
            let violations = validate(&truth, &trees);
            assert!(
                checks(&violations).contains(&"painting_entries_are_well_formed"),
                "{operation:?}: {violations:#?}"
            );
        }
        // A binary file has no text to paint.
        let binary = ChangesetTrees::from_files(&[("l.png", &[0xff, 0x00])], &[]).unwrap();
        let truth = HumanChangeset {
            text_mappings: vec![painting(vec![text_entry(
                Delete,
                vec![span("l.png", 0, 0, 1)],
                vec![],
            )])],
            ..Default::default()
        };
        assert_only(&truth, &binary, "painting_entries_are_well_formed");
    }

    #[test]
    fn a_match_is_a_move_or_an_update_by_its_text() {
        let trees = moved_trees();
        let moved = text_entry(
            HumanTextOperation::Match,
            vec![span("a.rs", 1, 0, 9)],
            vec![span("b.rs", 1, 0, 9)],
        );
        assert_eq!(moved.verdict(&trees).unwrap(), HumanTextVerdict::Move);
        let updated = text_entry(
            HumanTextOperation::Match,
            vec![span("a.rs", 0, 0, 9)],
            vec![span("b.rs", 1, 0, 9)],
        );
        assert_eq!(updated.verdict(&trees).unwrap(), HumanTextVerdict::Update);
    }

    #[test]
    fn ranges_of_one_painting_do_not_overlap_within_a_file() {
        let trees = moved_trees();
        let mut truth = moved_truth(&trees);
        truth.text_mappings[0].entries.push(text_entry(
            HumanTextOperation::Delete,
            vec![span("a.rs", 1, 3, 4)],
            vec![],
        ));
        let violations = validate(&truth, &trees);
        assert!(
            checks(&violations).contains(&"painted_ranges_do_not_overlap"),
            "{violations:#?}"
        );

        // The same columns in another file, or in another painting, claim other bytes.
        let mut truth = moved_truth(&trees);
        let elsewhere = truth.text_mappings[0].clone();
        truth.text_mappings.push(ChangesetTextMapping {
            name: "Another reading".to_string(),
            ..elsewhere
        });
        assert_eq!(validate(&truth, &trees), []);
    }

    #[test]
    fn unsolved_relations_are_partial_progress() {
        let trees = moved_trees();
        let whole = moved_truth(&trees);

        // Nothing decided yet but which files go together.
        let mut started = HumanChangeset {
            files: whole.files.clone(),
            ..Default::default()
        };
        for relation in &mut started.files {
            relation.solved = false;
        }
        assert_eq!(validate(&started, &trees), []);
        assert!(!started.is_solved(&trees));

        // One file done, the other not: still valid, still not solved.
        let mut half = whole.clone();
        half.files[1].solved = false;
        half.entries
            .retain(|entry| entry.before.as_ref().unwrap().file != "b.rs");
        assert_eq!(validate(&half, &trees), []);
        assert!(!half.is_solved(&trees));

        // Every record solved, but a file is in none.
        let mut missing = whole.clone();
        missing.files.pop();
        assert!(!missing.is_solved(&trees));
        assert!(whole.is_solved(&trees));
    }

    #[test]
    fn a_solved_file_has_no_undecided_node() {
        let trees = moved_trees();
        let mut truth = moved_truth(&trees);
        truth.text_mappings.clear();
        truth.entries.retain(|entry| {
            entry.before.as_ref().unwrap().path
                != ["function_item:1", "identifier:1"].map(String::from)
                || entry.before.as_ref().unwrap().file != "a.rs"
        });
        let violations = validate(&truth, &trees);
        assert_eq!(checks(&violations), ["solved_files_are_fully_marked"]);
        // Both sides of a.rs lose the entry.
        assert_eq!(violations.len(), 2, "{violations:#?}");
        assert!(violations[0].message.contains("1 of its nodes"));
    }

    #[test]
    fn the_single_file_invariants_judge_a_pair_and_a_cross_file_entry() {
        // Invariant 14 between files no record relates: `g` "identical" to a different function.
        let trees = moved_trees();
        let mut truth = moved_truth(&trees);
        truth.text_mappings.clear();
        for entry in &mut truth.entries {
            if let (Some(before), Some(after)) = (&entry.before, &mut entry.after)
                && before.file == "a.rs"
                && after.file == "b.rs"
            {
                after.path[0] = "function_item:1".to_string();
            }
        }
        let violations = validate(&truth, &trees);
        assert!(
            violations
                .iter()
                .any(|found| found.invariant == Some(14) && found.message.contains("a.rs -> b.rs")),
            "{violations:#?}"
        );

        // And a painting rule inside a `same` pair: a painted range ending in trailing whitespace
        // is invariant 1.
        let trees = super::tests::trees(&[("a.rs", "fn f() {}  \n")], &[("a.rs", "fn f() {}\n")]);
        let truth = HumanChangeset {
            files: vec![same("a.rs")],
            entries: subtree(&trees, ("a.rs", &[]), ("a.rs", &[])),
            text_mappings: vec![painting(vec![text_entry(
                HumanTextOperation::Delete,
                vec![span("a.rs", 0, 8, 11)],
                vec![],
            )])],
        };
        let violations = validate(&truth, &trees);
        assert!(
            violations
                .iter()
                .any(|found| found.check == "single_file_invariants_hold_per_pair"),
            "{violations:#?}"
        );
    }

    #[test]
    fn saving_is_deterministic_and_collapses_identical_subtrees() {
        let trees = moved_trees();
        let truth = moved_truth(&trees);
        let json = truth.to_json(&trees).unwrap();

        // Three whole subtrees and the two roots, whatever order the entries were in.
        let stored: serde_json::Value = serde_json::from_str(&json).unwrap();
        let operations: Vec<&str> = stored["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["operation"].as_str().unwrap())
            .collect();
        assert_eq!(
            operations,
            [
                "match_but_not_identical",
                "identical_with_children",
                "identical_with_children",
                "match_but_not_identical",
                "identical_with_children",
            ]
        );
        assert_eq!(stored["entries"][2]["before"]["file"], "a.rs");
        assert_eq!(stored["entries"][2]["after"]["file"], "b.rs");
        assert!(json.ends_with("}\n"));

        let mut shuffled = truth.clone();
        shuffled.entries.reverse();
        shuffled.files.reverse();
        let last = shuffled.entries[0].clone();
        shuffled.entries.push(last);
        assert_eq!(shuffled.to_json(&trees).unwrap(), json);

        // Loading expands what saving collapsed, and saving that writes the same bytes.
        let loaded = HumanChangeset::from_json(&json, &trees).unwrap();
        assert_eq!(loaded.entries.len(), truth.entries.len());
        assert_eq!(loaded.to_json(&trees).unwrap(), json);
        assert_eq!(validate(&loaded, &trees), []);
    }

    #[test]
    fn unknown_keys_in_the_truth_are_refused() {
        let trees = moved_trees();
        let json = r#"{ "files": [], "entries": [], "vcs_hints": [] }"#;
        assert!(HumanChangeset::from_json(json, &trees).is_err());
        let json = r#"{ "files": [ { "after": "a.rs", "relation": "added", "solved": false,
                        "hint": "A" } ] }"#;
        assert!(HumanChangeset::from_json(json, &trees).is_err());
    }

    fn hint(status: HintStatus, path: &str) -> VcsHint {
        VcsHint {
            status,
            path: path.to_string(),
            old_path: None,
            similarity: None,
        }
    }

    fn commit_source(trees: &ChangesetTrees, vcs_hints: Vec<VcsHint>) -> ChangesetSource {
        ChangesetSource {
            kind: ChangesetKind::Commit,
            repository: String::new(),
            commit: String::new(),
            parent: String::new(),
            root: None,
            signals: vec!["rename-unedited".to_string()],
            dataset: "handmade".to_string(),
            status: "PROMOTED".to_string(),
            files: source_files(trees),
            omitted: Vec::new(),
            vcs_hints: Some(vcs_hints),
        }
    }

    #[test]
    fn source_json_is_held_to_the_trees() {
        let trees = trees(
            &[
                ("a.rs", "fn f() {}\n"),
                ("u.rs", "fn u() {}\n"),
                ("old.rs", "fn o() {}\n"),
            ],
            &[
                ("a.rs", "fn g() {}\n"),
                ("u.rs", "fn u() {}\n"),
                ("new.rs", "fn o() {}\n"),
            ],
        );
        let good = commit_source(
            &trees,
            vec![
                hint(HintStatus::M, "a.rs"),
                VcsHint {
                    old_path: Some("old.rs".to_string()),
                    similarity: Some(100),
                    ..hint(HintStatus::R, "new.rs")
                },
            ],
        );
        assert_eq!(source_violations(&good, &trees), Vec::<String>::new());
        let json = good.to_json().unwrap();
        assert!(!json.contains("repository"), "{json}");
        assert!(
            !json.contains("root") && !json.contains("omitted"),
            "{json}"
        );
        assert_eq!(
            serde_json::from_str::<ChangesetSource>(&json).unwrap(),
            good
        );
        // The file has one definition: a key it does not have is refused, `stratum` included.
        let stale = json.replacen("\"signals\"", "\"stratum\": \"copy\",\n  \"signals\"", 1);
        assert!(serde_json::from_str::<ChangesetSource>(&stale).is_err());

        let broken = |change: &dyn Fn(&mut ChangesetSource)| {
            let mut source = good.clone();
            change(&mut source);
            source_violations(&source, &trees)
        };
        fn hints(source: &mut ChangesetSource) -> &mut Vec<VcsHint> {
            source.vcs_hints.as_mut().unwrap()
        }
        // A file missing from `files`, a wrong `untouched`, a hint about a path that is not
        // there, a hint missing, and a rename without where it came from.
        assert!(!broken(&|source| drop(source.files.pop())).is_empty());
        assert!(!broken(&|source| source.files[0].untouched = true).is_empty());
        assert!(!broken(&|source| source.files[3].untouched = false).is_empty());
        assert!(!broken(&|source| hints(source)[0].path = "b.rs".to_string()).is_empty());
        assert!(!broken(&|source| drop(hints(source).remove(0))).is_empty());
        assert!(!broken(&|source| hints(source)[1].old_path = None).is_empty());
        assert!(!broken(&|source| hints(source)[0].status = HintStatus::A).is_empty());
        assert!(!broken(&|source| hints(source).push(hint(HintStatus::M, "u.rs"))).is_empty());
        // What a commit has and has not, and the keys every kind is held to.
        assert!(!broken(&|source| source.vcs_hints = None).is_empty());
        assert!(!broken(&|source| source.root = Some(".".to_string())).is_empty());
        assert!(!broken(&|source| source.signals.clear()).is_empty());
        assert!(!broken(&|source| source.signals = vec!["rename unedited".to_string()]).is_empty());
        assert!(
            !broken(&|source| {
                source.signals = vec!["rename-unedited".to_string(), "copy".to_string()]
            })
            .is_empty()
        );
        assert!(
            !broken(&|source| { source.signals = vec!["control".to_string(), "copy".to_string()] })
                .is_empty()
        );
        assert!(!broken(&|source| source.status = "DONE".to_string()).is_empty());
        assert!(!broken(&|source| source.commit = "5f44ea07".to_string()).is_empty());
        assert!(broken(&|source| source.signals = vec!["control".to_string()]).is_empty());
    }

    /// `count` files `f<n>.rs`, the first `changed` of them differing between the sides.
    fn many_files(count: usize, changed: usize) -> ChangesetTrees {
        let side = |after: bool| -> Vec<(String, String)> {
            (0..count)
                .map(|index| {
                    let body = if after && index < changed { "1" } else { "0" };
                    (format!("f{index:03}.rs"), format!("fn f() {{ {body} }}\n"))
                })
                .collect()
        };
        fn refs(files: &[(String, String)]) -> Vec<(&str, &[u8])> {
            files
                .iter()
                .map(|(path, text)| (path.as_str(), text.as_bytes()))
                .collect()
        }
        let (before, after) = (side(false), side(true));
        ChangesetTrees::from_files(&refs(&before), &refs(&after)).unwrap()
    }

    fn modified_hints(trees: &ChangesetTrees) -> Vec<VcsHint> {
        source_files(trees)
            .iter()
            .filter(|file| !file.untouched)
            .map(|file| hint(HintStatus::M, file.before.as_ref().unwrap()))
            .collect()
    }

    #[test]
    fn the_file_cap_is_on_changed_files_and_goes_by_kind() {
        let folders = |trees: &ChangesetTrees| ChangesetSource {
            kind: ChangesetKind::Folders,
            root: Some("src".to_string()),
            vcs_hints: None,
            ..commit_source(trees, Vec::new())
        };
        // Twenty changed files beside the untouched ones a signal may point at: a commit.
        let trees = many_files(
            MAX_CHANGED_FILES + MAX_COMMIT_UNTOUCHED_FILES,
            MAX_CHANGED_FILES,
        );
        let commit = commit_source(&trees, modified_hints(&trees));
        assert_eq!(source_violations(&commit, &trees), Vec::<String>::new());
        // One more changed file is too many for either kind.
        let trees = many_files(MAX_CHANGED_FILES + 1, MAX_CHANGED_FILES + 1);
        let commit = commit_source(&trees, modified_hints(&trees));
        assert_eq!(source_violations(&commit, &trees).len(), 1);
        assert_eq!(source_violations(&folders(&trees), &trees).len(), 1);
        // A whole tree of unchanged files is what folders hold, and not what a commit does.
        let trees = many_files(MAX_TREE_FILES, 2);
        assert_eq!(
            source_violations(&folders(&trees), &trees),
            Vec::<String>::new()
        );
        let commit = commit_source(&trees, modified_hints(&trees));
        assert_eq!(source_violations(&commit, &trees).len(), 1);
        // And a tree has its own cap, a side.
        let trees = many_files(MAX_TREE_FILES + 1, 2);
        assert_eq!(source_violations(&folders(&trees), &trees).len(), 2);
    }

    #[test]
    fn folders_have_a_root_no_hints_and_nothing_omitted_in_their_trees() {
        let trees = trees(
            &[("a.rs", "fn f() {}\n"), ("u.rs", "fn u() {}\n")],
            &[("a.rs", "fn g() {}\n"), ("u.rs", "fn u() {}\n")],
        );
        let omitted = |side: &str, path: &str, reason: &str, bytes: Option<u64>| Omitted {
            side: side.to_string(),
            path: path.to_string(),
            reason: reason.to_string(),
            bytes,
            id: "0123abcd".to_string(),
        };
        let good = ChangesetSource {
            kind: ChangesetKind::Folders,
            repository: "example".to_string(),
            commit: "c".repeat(40),
            parent: "p".repeat(40),
            root: Some(".".to_string()),
            signals: vec![CONTROL_SIGNAL.to_string()],
            dataset: "changeset-pilot".to_string(),
            status: "SAMPLED".to_string(),
            files: source_files(&trees),
            omitted: vec![
                omitted("before", "big.bin", "size", Some(2_000_000)),
                omitted("after", "link", "symlink", Some(4)),
                omitted("before", "link", "symlink", Some(4)),
                omitted("after", "vendor/lib", "submodule", None),
            ],
            vcs_hints: None,
        };
        assert_eq!(source_violations(&good, &trees), Vec::<String>::new());
        let json = good.to_json().unwrap();
        assert!(!json.contains("vcs_hints"), "{json}");
        assert_eq!(
            serde_json::from_str::<ChangesetSource>(&json).unwrap(),
            good
        );

        let broken = |change: &dyn Fn(&mut ChangesetSource)| {
            let mut source = good.clone();
            change(&mut source);
            source_violations(&source, &trees)
        };
        assert!(!broken(&|source| source.root = None).is_empty());
        assert!(!broken(&|source| source.root = Some("../up".to_string())).is_empty());
        assert!(!broken(&|source| source.vcs_hints = Some(Vec::new())).is_empty());
        // Every file that did not change is marked, with no signal needed to point at it.
        assert!(!broken(&|source| source.files[1].untouched = false).is_empty());
        // An omitted path is no file of its side; and each entry is well-formed, once, in order.
        assert!(!broken(&|source| source.omitted[0].path = "a.rs".to_string()).is_empty());
        assert!(!broken(&|source| source.omitted[0].side = "both".to_string()).is_empty());
        assert!(!broken(&|source| source.omitted[0].reason = "generated".to_string()).is_empty());
        assert!(!broken(&|source| source.omitted[0].bytes = None).is_empty());
        assert!(!broken(&|source| source.omitted[3].bytes = Some(1)).is_empty());
        assert!(!broken(&|source| source.omitted.swap(1, 2)).is_empty());
        assert!(!broken(&|source| source.omitted[2] = source.omitted[1].clone()).is_empty());
    }

    // ─── The fixtures on disk ───────────────────────────────────────────────────────────────

    /// What `git diff --name-status -M -z` reports between the two trees, in git's order: each
    /// side is written into a temporary repository's index and the two trees compared. `None`
    /// when there is no `git` to run.
    fn git_vcs_hints(trees: &ChangesetTrees) -> Result<Option<Vec<VcsHint>>> {
        let scratch = tempfile::tempdir()?;
        let repository = scratch.path().join("repository");
        fs::create_dir(&repository)?;
        // Nothing of the machine's git configuration may reach the answer.
        let no_config = scratch.path().join("gitconfig");
        fs::write(&no_config, "")?;
        let git = |arguments: &[&str]| -> Result<Option<Vec<u8>>> {
            let output = std::process::Command::new("git")
                .current_dir(&repository)
                .env("GIT_CONFIG_GLOBAL", &no_config)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .args(["-c", "core.autocrlf=false", "-c", "core.safecrlf=false"])
                .args(arguments)
                .output();
            let output = match output {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                other => other?,
            };
            ensure!(
                output.status.success(),
                "git {arguments:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(Some(output.stdout))
        };
        if git(&["init", "--quiet"])?.is_none() {
            return Ok(None);
        }
        let mut tree_ids = Vec::new();
        for side in [&trees.before, &trees.after] {
            for entry in fs::read_dir(&repository)? {
                let entry = entry?;
                if entry.file_name() != ".git" {
                    if entry.file_type()?.is_dir() {
                        fs::remove_dir_all(entry.path())?;
                    } else {
                        fs::remove_file(entry.path())?;
                    }
                }
            }
            for (path, file) in side {
                let on_disk = repository.join(path);
                fs::create_dir_all(on_disk.parent().unwrap())?;
                fs::write(on_disk, &file.bytes)?;
            }
            git(&["add", "--all"])?;
            let id = git(&["write-tree"])?.context("git went away")?;
            tree_ids.push(String::from_utf8(id)?.trim().to_string());
        }
        let output = git(&[
            "diff",
            "--name-status",
            "-M",
            "-z",
            &tree_ids[0],
            &tree_ids[1],
        ])?
        .context("git went away")?;
        let output = String::from_utf8(output)?;
        let mut fields = output.split('\0').filter(|field| !field.is_empty());
        let mut hints = Vec::new();
        while let Some(status) = fields.next() {
            let mut next = || fields.next().context("git's output ends inside a record");
            let (letter, score) = status.split_at(1);
            let Some(status) = letter.chars().next().and_then(HintStatus::from_letter) else {
                bail!("git reports a status {letter:?} the format has no name for");
            };
            let old_path = match status {
                HintStatus::R | HintStatus::C => Some(next()?.to_string()),
                _ => None,
            };
            hints.push(VcsHint {
                status,
                path: next()?.to_string(),
                similarity: old_path.is_some().then(|| score.parse()).transpose()?,
                old_path,
            });
        }
        Ok(Some(hints))
    }

    /// The dev helper behind the handmade set's `source.json` files: rewrites each one's `files`
    /// and `vcs_hints` from its trees, keeping what a person wrote (`signals`, `status`). Run it
    /// after changing a handmade tree:
    /// `cargo test --all-features write_handmade_sources -- --ignored`.
    #[test]
    #[ignore = "rewrites src/test/data/changesets/handmade/*/source.json"]
    fn write_handmade_sources() -> Result<()> {
        for (dataset, name, dir) in changeset_dirs()? {
            if dataset != "handmade" {
                continue;
            }
            let trees = ChangesetTrees::load(&dir)?;
            let mut source = ChangesetSource::load(&dir)?;
            source.files = source_files(&trees);
            source.vcs_hints = Some(git_vcs_hints(&trees)?.context("this needs git")?);
            source.dataset = dataset;
            fs::write(dir.join(SOURCE_FILE_NAME), source.to_json()?)?;
            println!("{name}: {:?}", source.vcs_hints);
        }
        Ok(())
    }

    #[test]
    fn handmade_hints_are_what_git_reports() -> Result<()> {
        for (dataset, name, dir) in changeset_dirs()? {
            if dataset != "handmade" {
                continue;
            }
            let changeset = Changeset::load(&dir)?;
            let Some(hints) = git_vcs_hints(&changeset.trees)? else {
                eprintln!("no git on PATH: the hints are not compared");
                return Ok(());
            };
            assert_eq!(changeset.source.kind, ChangesetKind::Commit, "{name}");
            assert_eq!(changeset.source.hints(), hints, "{name}");
        }
        Ok(())
    }

    /// The stub of changeset `name`, `src/test/fixtures/changesets/<name, `-` as `_`>.rs`.
    fn stub_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("test")
            .join("fixtures")
            .join("changesets")
            .join(format!("{}.rs", name.replace('-', "_")))
    }

    #[test]
    fn every_changeset_loads_and_its_source_fits_its_trees() -> Result<()> {
        let mut names = HashSet::new();
        let all = changeset_dirs()?;
        assert!(!all.is_empty());
        for (dataset, name, dir) in all {
            assert!(
                names.insert(name.clone()),
                "'{name}' is in two datasets; a name is unique across them"
            );
            let changeset = Changeset::load(&dir)?;
            assert_eq!(changeset.source.dataset, dataset, "{name}");
            assert_eq!(changeset.source.status, "PROMOTED", "{name}");
            assert_eq!(fixture_violations(&dir)?, Vec::<String>::new(), "{name}");
        }
        Ok(())
    }

    /// The same inventory over the samples nobody has promoted yet, which are git-ignored and
    /// so not there on every machine: `cargo test --all-features --lib
    /// every_changeset_sample_loads -- --ignored` after `materialize_test_diffs --changesets`.
    /// `CHANGESET_SAMPLES_DIR` names another directory of samples (a materializer's
    /// `--output-dir`).
    #[test]
    #[ignore = "reads the git-ignored src/test/data/samples/changesets/"]
    fn every_changeset_sample_loads_and_its_source_fits_its_trees() -> Result<()> {
        let root = std::env::var_os("CHANGESET_SAMPLES_DIR")
            .map_or_else(changeset_samples_root, PathBuf::from);
        let mut failures = Vec::new();
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
            .with_context(|| format!("reading {root:?}"))?
            .map(|entry| Ok(entry?.path()))
            .collect::<Result<_>>()?;
        dirs.sort();
        for dir in &dirs {
            let name = dir.file_name().unwrap().to_string_lossy().into_owned();
            match fixture_violations(dir) {
                Ok(violations) => {
                    failures.extend(violations.iter().map(|found| format!("{name}: {found}")))
                }
                Err(error) => failures.push(format!("{name}: {error:#}")),
            }
            if let Ok(source) = ChangesetSource::load(dir) {
                *kinds.entry(source.kind.name()).or_default() += 1;
                if !CHANGESET_DATASETS.contains(&source.dataset.as_str()) {
                    failures.push(format!(
                        "{name}: dataset {:?} is not one of {CHANGESET_DATASETS:?}, so the \
                         sample cannot be promoted",
                        source.dataset
                    ));
                }
            }
        }
        println!("{} samples under {root:?}: {kinds:?}", dirs.len());
        assert!(
            failures.is_empty(),
            "{} problem(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
        Ok(())
    }

    #[test]
    fn solved_changesets_have_a_stub_and_only_they_do() -> Result<()> {
        let mut stubs: BTreeSet<PathBuf> = fs::read_dir(stub_path("x").parent().unwrap())?
            .map(|entry| Ok(entry?.path()))
            .collect::<Result<_>>()?;
        for (_, name, dir) in changeset_dirs()? {
            let changeset = Changeset::load(&dir)?;
            let solved = changeset
                .truth
                .as_ref()
                .is_some_and(|truth| truth.is_solved(&changeset.trees));
            assert_eq!(
                stubs.remove(&stub_path(&name)),
                solved,
                "'{name}': a stub is for a solved changeset, and every solved one has one"
            );
        }
        assert!(stubs.is_empty(), "stubs without a changeset: {stubs:?}");
        Ok(())
    }

    #[test]
    fn the_worked_examples_hold_what_they_are_for() -> Result<()> {
        // The intact move: one cross-file entry between two `same` files, painted as a move.
        let moved = Changeset::load_by_name("rust-function-moved-intact")?;
        let truth = moved.truth.as_ref().unwrap();
        let crossing = |entry: &&ChangesetEntry| match (&entry.before, &entry.after) {
            (Some(before), Some(after)) => before.file != after.file,
            _ => false,
        };
        assert!(truth.entries.iter().any(|entry| crossing(&entry)));
        for painting in &truth.text_mappings {
            for entry in &painting.entries {
                assert_eq!(entry.verdict(&moved.trees)?, HumanTextVerdict::Move);
            }
        }
        // Its painting is held to the tree through the pair view: unpainted, the function that
        // left `geometry.rs` is a removed leaf nobody painted (invariant 11).
        let mut unpainted = truth.clone();
        for painting in &mut unpainted.text_mappings {
            painting.entries.clear();
        }
        let violations = validate(&unpainted, &moved.trees);
        assert!(
            !violations.is_empty() && violations.iter().all(|found| found.invariant == Some(11)),
            "{violations:#?}"
        );

        // The copy: `round_to` in the untouched file has two partners.
        let copied = Changeset::load_by_name("rust-function-copied")?;
        let truth = copied.truth.as_ref().unwrap();
        let original = NodeRef::new("src/units.rs", &["function_item:1"]);
        let partners: Vec<&str> = truth
            .entries
            .iter()
            .filter(|entry| entry.before.as_ref() == Some(&original))
            .map(|entry| entry.after.as_ref().unwrap().file.as_str())
            .collect();
        assert_eq!(partners, ["src/report.rs", "src/units.rs"]);
        let untouched: Vec<_> = copied
            .source
            .files
            .iter()
            .filter(|file| file.untouched)
            .collect();
        assert_eq!(untouched.len(), 1);
        assert_eq!(untouched[0].before.as_deref(), Some("src/units.rs"));
        Ok(())
    }
}
