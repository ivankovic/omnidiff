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
//! Changeset sampling: what a multi-file fixture holds and which signals it carries, for
//! `sample_test_diffs --changesets` and `materialize_test_diffs --changesets`.
//!
//! The fixture format, `source.json` and the limits of each kind are specified in
//! `test::helper::human_changeset`; what is sampled here is held to them
//! (`source_violations`). A changeset is of one of two kinds ([`ChangesetKind`]; [`KIND_COMMIT`]
//! and [`KIND_FOLDERS`] in the CSV). A *commit* is every file one commit changes,
//! [`MIN_FILES`]..=[`MAX_FILES`] of them, plus at most [`MAX_UNTOUCHED_SOURCES`] untouched
//! files a signal points at, with the VCS's hints beside it. *Folders* are two complete
//! directory trees as they would lie on disk, with no VCS, no hints and no notion of a commit.
//!
//! # What a commit holds
//!
//! [`load_changes`] reads the tree diff without rename detection, so a file is added, removed or
//! modified at one path, and drops:
//! - paths [`anomalous_paths::is_anomalous`] drops, lock files ([`LOCK_FILES`]) and text
//!   [`crate::stats::is_generated`] calls generated: they are not code somebody wrote. The path
//!   is asked about with a leading `/`, as its patterns for the repository root (`/.github/`) are
//!   written; a tree diff's paths have none;
//! - symbolic links and submodules, on either side: a link's "content" is a path and a submodule
//!   has none, so neither is a file to diff. A changeset never holds one, and nothing follows one;
//! - mode-only changes.
//!
//! A commit is skipped when fewer than two or more than twenty files are left, when a file is
//! larger than [`MAX_FILE_BYTES`], or when it changes more than [`MAX_CHANGED_LINES`] lines
//! (added plus removed, a whole added or removed file counting every line): nobody solves a
//! larger one by hand.
//!
//! # What folders hold
//!
//! Two points in a repository's history - the tree before the first of `--range-length`
//! consecutive single-parent commits and the tree after the last - and one sub-folder of it,
//! complete on both sides, unchanged files included ([`choose_folder`], [`load_folder`]).
//! Nothing is dropped by its path: a lock file or a vendored directory is there on disk too.
//!
//! The sub-folder is the directory, the repository root included, that holds the most changed
//! *code* files among those that qualify - then the most changed files, then the deepest. A
//! code file is one whose path names a language with a tree-sitter grammar
//! ([`is_code_path`]): what a person solving the fixture maps node by node. A directory
//! qualifies when
//! - it holds [`MIN_FILES`]..=[`MAX_FILES`] changed files, none of them over
//!   [`MAX_FILE_BYTES`], and at least one of them is code;
//! - no component of its path starts with a dot (`.github`, `.changes`, `.vscode`): what lives
//!   there is configuration and release bookkeeping, many small files changing together, which
//!   won nearly one pilot folder in eight under the first rule, "the most changed files";
//! - it holds at most `--max-tree-files` files on either side ([`MAX_TREE_FILES`] by default,
//!   and at most).
//!
//! The pair is skipped when no directory qualifies, or when the one chosen changes more than
//! [`MAX_CHANGED_LINES`] lines. Because the rule takes the directory that keeps the most of
//! what changed, twenty commits apart it rarely takes one with two or three changed files:
//! the `2-3` bucket of folders stays nearly empty by construction (`--range-length` is the
//! knob for that, not the choice of directory).
//!
//! What a tree on disk would hold and a fixture does not is listed, not passed over
//! ([`Omitted`]): an unchanged file over [`MAX_FILE_BYTES`] (with its size and blob id), a
//! symbolic link, a submodule. The tree is known to be incomplete exactly there. Binary files
//! under the cap are held as they are. A changed path with a link or submodule on one side is
//! a file added or removed on the other.
//!
//! # Signals
//!
//! [`classify`] is a pure function of the changed files' blobs, the VCS's rename hints and an
//! index of the untouched files. No signal uses the VCS's file pairing but `rename-edited`, which
//! is defined by it, and which folders therefore never carry: they are classified with no hints,
//! over the whole sub-folder, its unchanged files being the untouched ones. A changeset carries
//! every signal that fires, in no order of importance: a directory renamed with a function moved
//! out of one of its files is a `directory-move`, a `rename-edited` and a `block-move` at once,
//! and a fixture of it shows all three.
//!
//! Lines are compared after trimming whitespace, and blank lines, lines without a letter or digit
//! (braces, `});`), lines that are one of [`TRIVIAL_WORDS`], imports ([`IMPORT_PREFIXES`]) and
//! lines that are nothing but one string (Go's imports) are ignored. A *block* is
//! [`BLOCK_LINES`] consecutive such lines holding at least [`MIN_BLOCK_CHARS`] characters. A file
//! *loses* a block when the line diff removes every line of it and the block is nowhere in the
//! file afterwards, and *gains* one the same way round, so a block the diff merely realigned is
//! neither. A block is *boilerplate*, and ignored, when more than [`BOILERPLATE_FILES`] files of
//! the changeset lose it, or gain it, or more than that many untouched files hold it: licence
//! headers and import runs are everywhere, and say nothing about where code went.
//!
//! - `rename-unedited`: a removed path's blob is an added path's blob, and it is text.
//! - `non-code-move`: the same, and the blob is content ([`content::sniff`]) or not text at all.
//! - `rename-edited`: the VCS reports a rename with similarity below 100 between a removed and an
//!   added path of the changeset.
//! - `directory-move`: at least [`DIRECTORY_MOVE_FILES`] removed paths reappear as added paths
//!   under one directory substitution (the same trailing components, another prefix).
//! - `block-move`: a block one file loses is gained by another. Not between two paths the VCS
//!   reports as a rename, where the blocks only went where their file went; a removed file whose
//!   blocks land in an added one that the VCS does not pair does count, since nothing else says
//!   where it went.
//! - `split-or-merge`: one file's lost blocks are gained by two or more files, or two or more
//!   files' lost blocks by one, renames included (a file renamed with a function left behind).
//! - `copy`: a block a file gains, that no file lost, is held before and after by at most
//!   [`BOILERPLATE_FILES`] other files, changed or untouched ([`Untouched`]); at least
//!   [`COPY_BLOCKS`] such blocks from one source. Untouched sources are recorded, the
//!   [`MAX_UNTOUCHED_SOURCES`] with the most blocks, so the fixture holds them.
//! - `moved-and-edited`: no block of file A is gained by file B, yet A's removed lines and B's
//!   added lines, each at least [`MOVED_EDITED_LINES`] long, share tokens: the Jaccard similarity
//!   of their [`SHINGLE_TOKENS`]-token shingles is at least [`MOVED_EDITED_SIMILARITY`]. Shingles a
//!   file both removes and adds are left out first, so one edit made to two files is not a move.
//! - `cross-language`: a removed and an added file share a stem and are of different programming
//!   languages, and are not one blob (`Foo.java` renamed `Foo.kt` and translated in the next
//!   commit is a rename here and an edit there; folders spanning the two hold the translation).
//! - `control` ([`CONTROL`]): none of the above.
//!
//! # `changesets.csv`
//!
//! One [`Row`] a changeset. `signals` holds every signal that fired, sorted by name and
//! `;`-separated, or `control`; `untouched_paths` the untouched files a `copy` points at, likewise
//! separated. `files` counts the changed files, which also decide `bucket` ([`file_bucket`]).
//! `root` is the sub-folder of a `folders` row, `.` for the repository root.
//!
//! The CSV is the sampler's list, and the materializer's input. A fixture's own `source.json`
//! says the same in the format's terms: `signals` as a list, and the untouched files as
//! `files[].untouched`, which for folders is every file that did not change. `human_solver`
//! sets `status` in both when it promotes a sample.

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::rc::Rc;

use anyhow::{Result, bail};
use git2::{DiffOptions, FileMode, ObjectType, Oid, Patch, Repository, Tree};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};

use crate::anomalous_paths;
use crate::code::language::language_for_path;
use crate::code::{Language, decode_text};
use crate::diff::content;
use crate::test::helper::human_changeset as format;
pub use crate::test::helper::human_changeset::{ChangesetKind, Omitted};

/// The fewest changed files that make a changeset worth sampling.
pub const MIN_FILES: usize = 2;
/// The format's cap on changed files; the untouched files a signal adds are not counted.
pub const MAX_FILES: usize = format::MAX_CHANGED_FILES;
/// The format's cap on a side of a folder, and `--max-tree-files`'s default and limit.
pub const MAX_TREE_FILES: usize = format::MAX_TREE_FILES;
/// Added plus removed lines of a changeset.
pub const MAX_CHANGED_LINES: usize = 3_000;
/// The size `stats::expand_from_code` refuses to parse; `sample_test_diffs`'s cap for code.
pub const MAX_FILE_BYTES: usize = 1024 * 1024;
/// N: the lines of a block.
pub const BLOCK_LINES: usize = 4;
pub const MIN_BLOCK_CHARS: usize = 80;
pub const BOILERPLATE_FILES: usize = 2;
/// Blocks one source must share with one file for `copy`, nine lines in a row or as many in
/// pieces: in the pilot, at one to three blocks nearly every `copy` was a new test file opening
/// like its neighbour (the same six lines of set-up), not code somebody copied.
pub const COPY_BLOCKS: usize = 6;
/// The untouched `copy` sources a commit records: the format's cap on a commit's untouched files.
pub const MAX_UNTOUCHED_SOURCES: usize = format::MAX_COMMIT_UNTOUCHED_FILES;
/// K: the lines each side of a `moved-and-edited` pair must lose and gain.
pub const MOVED_EDITED_LINES: usize = 8;
pub const SHINGLE_TOKENS: usize = 3;
/// The fewest shingles a side of a `moved-and-edited` pair needs: below it, a ratio is noise.
pub const MIN_SHINGLES: usize = 30;
pub const MOVED_EDITED_SIMILARITY: f64 = 0.4;
pub const DIRECTORY_MOVE_FILES: usize = 3;
/// The largest untouched file indexed as a `copy` source.
pub const UNTOUCHED_MAX_BYTES: usize = 256 * 1024;
/// The most paths a repository's untouched-file index holds; a larger tree's rest is not searched.
pub const UNTOUCHED_MAX_PATHS: usize = 20_000;
/// A block this many indexed paths have held is boilerplate without looking further.
const INDEX_PATHS_PER_BLOCK: usize = 32;

/// Lines that are structure, not content, once punctuation is dropped and case folded.
pub const TRIVIAL_WORDS: &[&str] = &[
    "begin", "break", "continue", "default", "do", "done", "else", "end", "endif", "esac", "fi",
    "finally", "override", "pass", "return", "then", "try",
];

/// Dependency lock files: generated, and often thousands of lines a change.
pub const LOCK_FILES: &[&str] = &[
    "Cargo.lock",
    "Gemfile.lock",
    "Pipfile.lock",
    "bun.lock",
    "bun.lockb",
    "composer.lock",
    "flake.lock",
    "go.sum",
    "npm-shrinkwrap.json",
    "package-lock.json",
    "packages.lock.json",
    "pnpm-lock.yaml",
    "poetry.lock",
    "uv.lock",
    "yarn.lock",
];

/// What a changeset can show. Declared in the order of the names, which is the order a
/// changeset's signals are listed in; none ranks above another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Signal {
    BlockMove,
    Copy,
    CrossLanguage,
    DirectoryMove,
    MovedAndEdited,
    NonCodeMove,
    RenameEdited,
    RenameUnedited,
    SplitOrMerge,
}

/// What a changeset with no signal is called, in `signals` and as a sampling cell.
pub const CONTROL: &str = format::CONTROL_SIGNAL;
/// A changeset that is one commit, with the VCS's hints, as the `kind` column spells it.
pub const KIND_COMMIT: &str = "commit";
/// A changeset that is two directory trees, with nothing from a VCS.
pub const KIND_FOLDERS: &str = "folders";

/// The kind a `kind` column names.
pub fn kind_of(name: &str) -> Option<ChangesetKind> {
    [ChangesetKind::Commit, ChangesetKind::Folders]
        .into_iter()
        .find(|kind| kind.name() == name)
}

impl Signal {
    pub const ALL: [Signal; 9] = [
        Signal::BlockMove,
        Signal::Copy,
        Signal::CrossLanguage,
        Signal::DirectoryMove,
        Signal::MovedAndEdited,
        Signal::NonCodeMove,
        Signal::RenameEdited,
        Signal::RenameUnedited,
        Signal::SplitOrMerge,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Signal::BlockMove => "block-move",
            Signal::Copy => "copy",
            Signal::CrossLanguage => "cross-language",
            Signal::DirectoryMove => "directory-move",
            Signal::MovedAndEdited => "moved-and-edited",
            Signal::NonCodeMove => "non-code-move",
            Signal::RenameEdited => "rename-edited",
            Signal::RenameUnedited => "rename-unedited",
            Signal::SplitOrMerge => "split-or-merge",
        }
    }
}

/// The file-count buckets a signal is sampled in.
pub const FILE_BUCKETS: [&str; 3] = ["2-3", "4-8", "9-20"];

/// Which of [`FILE_BUCKETS`] a changeset of `files` changed files falls into.
pub fn file_bucket(files: usize) -> &'static str {
    match files {
        0..=3 => FILE_BUCKETS[0],
        4..=8 => FILE_BUCKETS[1],
        _ => FILE_BUCKETS[2],
    }
}

/// One side of a changed file.
#[derive(Debug, Clone)]
pub struct Blob {
    pub id: Oid,
    pub bytes: Vec<u8>,
}

impl Blob {
    /// A blob of `bytes`, with the id git gives them.
    pub fn of(bytes: &[u8]) -> Blob {
        Blob {
            id: Oid::hash_object(ObjectType::Blob, bytes).unwrap_or(Oid::ZERO_SHA1),
            bytes: bytes.to_vec(),
        }
    }
}

/// A path the changeset changes: added (`after` only), removed (`before` only) or modified.
#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub before: Option<Blob>,
    pub after: Option<Blob>,
}

/// What the VCS says happened to a path (`git diff --name-status -M`). Data, never truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsHint {
    /// `A`, `D`, `M`, `R`, `C` or `T`.
    pub status: char,
    pub path: String,
    /// Where a renamed or copied file came from.
    pub old_path: Option<String>,
    /// Of a rename or copy, in percent.
    pub similarity: Option<u8>,
}

impl VcsHint {
    /// The hint as `source.json` holds one; `None` for a status the format has no name for (a
    /// type change, `T`: a link or a submodule, which a changeset never holds).
    pub fn to_source(&self) -> Option<format::VcsHint> {
        Some(format::VcsHint {
            status: format::HintStatus::from_letter(self.status)?,
            path: self.path.clone(),
            old_path: self.old_path.clone(),
            similarity: self.similarity,
        })
    }
}

/// The untouched files of the before tree that hold a block.
pub enum Holders {
    Paths(Vec<String>),
    /// More than anyone would call a copy source.
    Boilerplate,
}

/// The before tree's untouched files, as `copy` sources and as the measure of boilerplate.
pub trait Untouched {
    fn holders(&self, block: u64) -> Holders;
}

/// No untouched files.
pub struct NoUntouched;

impl Untouched for NoUntouched {
    fn holders(&self, _block: u64) -> Holders {
        Holders::Paths(Vec::new())
    }
}

/// Untouched files given as text: what a test has, and the unchanged files of a folder.
#[derive(Default)]
pub struct UntouchedTexts {
    by_block: FxHashMap<u64, Vec<String>>,
}

impl UntouchedTexts {
    pub fn new<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> UntouchedTexts {
        let mut by_block: FxHashMap<u64, Vec<String>> = FxHashMap::default();
        for (path, text) in files {
            for block in blocks_of(text) {
                by_block.entry(block).or_default().push(path.to_string());
            }
        }
        UntouchedTexts { by_block }
    }
}

impl Untouched for UntouchedTexts {
    fn holders(&self, block: u64) -> Holders {
        match self.by_block.get(&block) {
            Some(paths) if paths.len() > BOILERPLATE_FILES => Holders::Boilerplate,
            Some(paths) => Holders::Paths(paths.clone()),
            None => Holders::Paths(Vec::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    /// Every signal that fired, sorted; none for a control.
    pub signals: Vec<Signal>,
    /// Untouched files a `copy` points at, for the fixture to hold.
    pub untouched: Vec<String>,
    /// What each signal saw, one line each, for a person checking it.
    pub evidence: Vec<String>,
}

fn hash_of(value: impl Hash) -> u64 {
    let mut hasher = FxHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

/// How a line that only names a dependency starts. Two files that need the same things list
/// them in the same order, and a new file's first lines are its neighbour's.
pub const IMPORT_PREFIXES: &[&str] = &[
    "#import ",
    "#include ",
    "@import ",
    "extern crate ",
    "from ",
    "import ",
    "namespace ",
    "package ",
    "use ",
    "using ",
];

/// True for a line that is structure: blank, punctuation only, one of [`TRIVIAL_WORDS`], an
/// import ([`IMPORT_PREFIXES`]) or nothing but one string.
fn is_trivial(trimmed: &str) -> bool {
    if IMPORT_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return true;
    }
    // A lone string, as a line of Go's `import (...)` is, alias and all.
    let literal = trimmed.strip_suffix(',').unwrap_or(trimmed);
    let literal = match literal.split_once(' ') {
        Some((alias, rest)) if rest.starts_with('"') && !alias.contains('"') => rest,
        _ => literal,
    };
    if literal.len() >= 2
        && literal.starts_with('"')
        && literal.ends_with('"')
        && !literal[1..literal.len() - 1].contains('"')
    {
        return true;
    }
    let word: String = trimmed
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    word.is_empty() || TRIVIAL_WORDS.contains(&word.as_str())
}

/// A line that counts: its number in the file (from 1) and its trimmed text.
fn significant_lines(text: &str) -> Vec<(u32, &str)> {
    text.split('\n')
        .enumerate()
        .map(|(index, line)| (index as u32 + 1, line.trim()))
        .filter(|(_, line)| !is_trivial(line))
        .collect()
}

/// The hash of each block of `lines`, with whether `changed` holds every line of it.
fn blocks(lines: &[(u32, &str)], changed: &FxHashSet<u32>) -> Vec<(u64, bool)> {
    lines
        .windows(BLOCK_LINES)
        .filter(|window| {
            window.iter().map(|(_, line)| line.len()).sum::<usize>() >= MIN_BLOCK_CHARS
        })
        .map(|window| {
            let texts: Vec<&str> = window.iter().map(|(_, line)| *line).collect();
            (
                hash_of(&texts),
                window.iter().all(|(number, _)| changed.contains(number)),
            )
        })
        .collect()
}

/// Every block of `text`.
pub fn blocks_of(text: &str) -> FxHashSet<u64> {
    blocks(&significant_lines(text), &FxHashSet::default())
        .into_iter()
        .map(|(hash, _)| hash)
        .collect()
}

/// The shingles of the lines of `lines` that `changed` holds.
fn shingles(lines: &[(u32, &str)], changed: &FxHashSet<u32>) -> FxHashSet<u64> {
    let tokens: Vec<&str> = lines
        .iter()
        .filter(|(number, _)| changed.contains(number))
        .flat_map(|(_, line)| {
            line.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .filter(|token| !token.is_empty())
        })
        .collect();
    tokens.windows(SHINGLE_TOKENS).map(hash_of).collect()
}

/// The lines a plain line diff of `before` and `after` removes and adds, by number from 1.
fn line_diff(before: &str, after: &str) -> (FxHashSet<u32>, FxHashSet<u32>) {
    let mut removed = FxHashSet::default();
    let mut added = FxHashSet::default();
    let mut options = DiffOptions::new();
    options.context_lines(0).force_text(true);
    let Ok(patch) = Patch::from_buffers(
        before.as_bytes(),
        None,
        after.as_bytes(),
        None,
        Some(&mut options),
    ) else {
        return (removed, added);
    };
    for hunk in 0..patch.num_hunks() {
        for line in 0..patch.num_lines_in_hunk(hunk).unwrap_or(0) {
            let Ok(line) = patch.line_in_hunk(hunk, line) else {
                continue;
            };
            match (line.origin(), line.old_lineno(), line.new_lineno()) {
                ('-', Some(number), _) => {
                    removed.insert(number);
                }
                ('+', _, Some(number)) => {
                    added.insert(number);
                }
                _ => {}
            }
        }
    }
    (removed, added)
}

/// What the block and token signals need of one changed text file.
#[derive(Default)]
struct Analysis {
    /// Every block of each side.
    before_blocks: FxHashSet<u64>,
    after_blocks: FxHashSet<u64>,
    lost: FxHashSet<u64>,
    gained: FxHashSet<u64>,
    /// Significant lines the diff removes and adds.
    removed_lines: usize,
    added_lines: usize,
    /// Shingles of the removed lines that are not also in the added lines, and the reverse.
    lost_shingles: FxHashSet<u64>,
    gained_shingles: FxHashSet<u64>,
    /// Every line the diff removes or adds, trivial ones too.
    changed_lines: usize,
}

fn text_of(side: &Option<Blob>) -> Option<&str> {
    match side {
        Some(blob) => std::str::from_utf8(&blob.bytes).ok(),
        None => Some(""),
    }
}

/// `None` for a file with a side that is not UTF-8 text.
fn analyze(file: &ChangedFile) -> Option<Analysis> {
    let before = text_of(&file.before)?;
    let after = text_of(&file.after)?;
    let all_of = |text: &str| -> FxHashSet<u32> { (1..=text.split('\n').count() as u32).collect() };
    let (removed, added) = match (&file.before, &file.after) {
        (Some(_), Some(_)) => line_diff(before, after),
        (Some(_), None) => (all_of(before), FxHashSet::default()),
        (None, _) => (FxHashSet::default(), all_of(after)),
    };
    let lines_of = |side: &Option<Blob>, text: &str| match side {
        // A final newline ends the last line; it does not start another.
        Some(_) => text.lines().count(),
        None => 0,
    };
    let changed_lines = match (&file.before, &file.after) {
        (Some(_), Some(_)) => removed.len() + added.len(),
        _ => lines_of(&file.before, before) + lines_of(&file.after, after),
    };
    let before_lines = significant_lines(before);
    let after_lines = significant_lines(after);
    let before_all = blocks(&before_lines, &removed);
    let after_all = blocks(&after_lines, &added);
    let before_blocks: FxHashSet<u64> = before_all.iter().map(|(hash, _)| *hash).collect();
    let after_blocks: FxHashSet<u64> = after_all.iter().map(|(hash, _)| *hash).collect();
    let lost = before_all
        .iter()
        .filter(|(hash, changed)| *changed && !after_blocks.contains(hash))
        .map(|(hash, _)| *hash)
        .collect();
    let gained = after_all
        .iter()
        .filter(|(hash, changed)| *changed && !before_blocks.contains(hash))
        .map(|(hash, _)| *hash)
        .collect();
    let removed_shingles = shingles(&before_lines, &removed);
    let added_shingles = shingles(&after_lines, &added);
    Some(Analysis {
        lost,
        gained,
        removed_lines: before_lines
            .iter()
            .filter(|(number, _)| removed.contains(number))
            .count(),
        added_lines: after_lines
            .iter()
            .filter(|(number, _)| added.contains(number))
            .count(),
        lost_shingles: removed_shingles
            .difference(&added_shingles)
            .copied()
            .collect(),
        gained_shingles: added_shingles
            .difference(&removed_shingles)
            .copied()
            .collect(),
        before_blocks,
        after_blocks,
        changed_lines,
    })
}

/// Added plus removed lines over `files`; a file that is not text counts none.
pub fn changed_lines(files: &[ChangedFile]) -> usize {
    files
        .iter()
        .filter_map(analyze)
        .map(|analysis| analysis.changed_lines)
        .sum()
}

/// The language most changed files are in, by path; the first by name on a tie, `Other` when no
/// file has one.
pub fn main_language(files: &[ChangedFile]) -> String {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    for file in files {
        if let Some(language) = language_for_path(Path::new(&file.path)) {
            *counts.entry(language.to_string()).or_default() += 1;
        }
    }
    let mut counts: Vec<(String, usize)> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
        .into_iter()
        .next()
        .map_or("Other".to_string(), |(language, _)| language)
}

/// True for bytes OmniDiff diffs as text: not content of a family, and decodable.
fn is_code(bytes: &[u8]) -> bool {
    content::sniff(bytes).is_none() && decode_text(bytes).is_some()
}

/// Removed and added paths holding one blob, each path in one pair: `(removed, added)` indexes.
fn exact_renames(files: &[ChangedFile]) -> Vec<(usize, usize)> {
    let mut removed: FxHashMap<Oid, Vec<usize>> = FxHashMap::default();
    for (index, file) in files.iter().enumerate() {
        if let (Some(before), None) = (&file.before, &file.after)
            && !before.bytes.is_empty()
        {
            removed.entry(before.id).or_default().push(index);
        }
    }
    let mut pairs = Vec::new();
    for (index, file) in files.iter().enumerate() {
        if let (None, Some(after)) = (&file.before, &file.after)
            && let Some(sources) = removed.get_mut(&after.id)
            && !sources.is_empty()
        {
            pairs.push((sources.remove(0), index));
        }
    }
    pairs
}

/// The directory substitutions that move at least [`DIRECTORY_MOVE_FILES`] files: a removed and
/// an added path with the same trailing components and different prefixes, by prefix pair.
pub fn directory_moves(removed: &[&str], added: &[&str]) -> Vec<(String, String, usize)> {
    let mut moves: FxHashMap<(String, String), FxHashSet<&str>> = FxHashMap::default();
    for old in removed {
        let old_parts: Vec<&str> = old.split('/').collect();
        for new in added {
            let new_parts: Vec<&str> = new.split('/').collect();
            let shared = old_parts
                .iter()
                .rev()
                .zip(new_parts.iter().rev())
                .take_while(|(a, b)| a == b)
                .count();
            if shared == 0 || old == new {
                continue;
            }
            let from = old_parts[..old_parts.len() - shared].join("/");
            let to = new_parts[..new_parts.len() - shared].join("/");
            moves.entry((from, to)).or_default().insert(old);
        }
    }
    let mut moves: Vec<(String, String, usize)> = moves
        .into_iter()
        .filter(|(_, files)| files.len() >= DIRECTORY_MOVE_FILES)
        .map(|((from, to), files)| (from, to, files.len()))
        .collect();
    moves.sort();
    moves
}

/// Languages that describe data or a document: a `deploy.js` replaced by a `deploy.yml` was not
/// translated.
const DATA_LANGUAGES: &[&str] = &["CSS", "HTML", "JSON", "MarkDown", "XML", "YAML"];

/// Removed and added paths with one stem and different programming languages.
pub fn cross_language_pairs<'a>(removed: &[&'a str], added: &[&'a str]) -> Vec<(&'a str, &'a str)> {
    let stem = |path: &str| {
        Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
    };
    let mut pairs = Vec::new();
    for old in removed {
        let Some(old_language) = language_for_path(Path::new(old)) else {
            continue;
        };
        let is_data = |language: Language| DATA_LANGUAGES.contains(&language.to_string().as_str());
        if is_data(old_language) {
            continue;
        }
        for new in added {
            if let Some(new_language) = language_for_path(Path::new(new))
                && new_language != old_language
                && !is_data(new_language)
                && stem(old) == stem(new)
            {
                pairs.push((*old, *new));
            }
        }
    }
    pairs
}

/// The signals of a changeset (see the module documentation).
pub fn classify(
    files: &[ChangedFile],
    hints: &[VcsHint],
    untouched: &dyn Untouched,
) -> Classification {
    let mut fired: FxHashSet<Signal> = FxHashSet::default();
    let mut evidence = Vec::new();
    let index_of: FxHashMap<&str, usize> = files
        .iter()
        .enumerate()
        .map(|(index, file)| (file.path.as_str(), index))
        .collect();
    let is_removed = |file: &ChangedFile| file.before.is_some() && file.after.is_none();
    let is_added = |file: &ChangedFile| file.before.is_none() && file.after.is_some();
    let removed: Vec<&str> = files
        .iter()
        .filter(|file| is_removed(file))
        .map(|file| file.path.as_str())
        .collect();
    let added: Vec<&str> = files
        .iter()
        .filter(|file| is_added(file))
        .map(|file| file.path.as_str())
        .collect();

    let exact = exact_renames(files);
    let mut renamed: FxHashSet<usize> = FxHashSet::default();
    for (old, new) in &exact {
        renamed.extend([*old, *new]);
        let code = files[*new]
            .after
            .as_ref()
            .is_some_and(|blob| is_code(&blob.bytes));
        let signal = if code {
            Signal::RenameUnedited
        } else {
            Signal::NonCodeMove
        };
        fired.insert(signal);
        evidence.push(format!(
            "{}: {} -> {}",
            signal.name(),
            files[*old].path,
            files[*new].path
        ));
    }

    // The one signal the VCS defines.
    let mut hinted: FxHashSet<(usize, usize)> = FxHashSet::default();
    for hint in hints {
        let Some(old_path) = &hint.old_path else {
            continue;
        };
        if hint.status != 'R' || hint.similarity.is_none_or(|similarity| similarity >= 100) {
            continue;
        }
        if let (Some(old), Some(new)) = (
            index_of.get(old_path.as_str()),
            index_of.get(hint.path.as_str()),
        ) && is_removed(&files[*old])
            && is_added(&files[*new])
        {
            hinted.insert((*old, *new));
            fired.insert(Signal::RenameEdited);
            evidence.push(format!(
                "rename-edited: {} -> {} ({}%)",
                old_path,
                hint.path,
                hint.similarity.unwrap_or(0)
            ));
        }
    }

    for (from, to, count) in directory_moves(&removed, &added) {
        fired.insert(Signal::DirectoryMove);
        evidence.push(format!("directory-move: {from}/ -> {to}/ ({count} files)"));
    }

    for (old, new) in cross_language_pairs(&removed, &added) {
        // The file under a new extension, not yet translated: a rename, and found as one.
        if exact.contains(&(index_of[old], index_of[new])) {
            continue;
        }
        fired.insert(Signal::CrossLanguage);
        evidence.push(format!("cross-language: {old} -> {new}"));
    }

    // Blocks: a file renamed as it is has moved whole, and none of its blocks is looked at.
    let analyses: Vec<Option<Analysis>> = files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            if renamed.contains(&index) {
                None
            } else {
                analyze(file)
            }
        })
        .collect();
    let mut losers: FxHashMap<u64, Vec<usize>> = FxHashMap::default();
    let mut gainers: FxHashMap<u64, usize> = FxHashMap::default();
    for (index, analysis) in analyses.iter().enumerate() {
        let Some(analysis) = analysis else { continue };
        for block in &analysis.lost {
            losers.entry(*block).or_default().push(index);
        }
        for block in &analysis.gained {
            *gainers.entry(*block).or_default() += 1;
        }
    }
    // Asked once a block: an index answers from the repository.
    let mut holders_of: FxHashMap<u64, Option<Vec<String>>> = FxHashMap::default();
    let mut holders = |block: u64| -> Option<Vec<String>> {
        holders_of
            .entry(block)
            .or_insert_with(|| match untouched.holders(block) {
                Holders::Paths(paths) if paths.len() <= BOILERPLATE_FILES => Some(paths),
                _ => None,
            })
            .clone()
    };

    let mut moves: FxHashMap<(usize, usize), usize> = FxHashMap::default();
    let mut copies: FxHashMap<(String, usize), usize> = FxHashMap::default();
    for (target, analysis) in analyses.iter().enumerate() {
        let Some(analysis) = analysis else { continue };
        for block in &analysis.gained {
            if gainers[block] > BOILERPLATE_FILES {
                continue;
            }
            let Some(untouched_holders) = holders(*block) else {
                continue;
            };
            let sources: Vec<usize> = losers
                .get(block)
                .map(|sources| {
                    sources
                        .iter()
                        .copied()
                        .filter(|source| *source != target)
                        .collect()
                })
                .unwrap_or_default();
            if !sources.is_empty() {
                if sources.len() <= BOILERPLATE_FILES {
                    for source in sources {
                        *moves.entry((source, target)).or_default() += 1;
                    }
                }
                continue;
            }
            // Nobody lost it: a copy, if few enough files hold it before and after.
            let language = language_for_path(Path::new(&files[target].path));
            let kept: Vec<&str> = analyses
                .iter()
                .enumerate()
                .filter(|(source, _)| *source != target)
                .filter_map(|(source, analysis)| {
                    let analysis = analysis.as_ref()?;
                    (analysis.before_blocks.contains(block)
                        && analysis.after_blocks.contains(block))
                    .then_some(files[source].path.as_str())
                })
                .collect();
            let elsewhere: Vec<&String> = untouched_holders
                .iter()
                .filter(|path| !index_of.contains_key(path.as_str()))
                .collect();
            if kept.len() + elsewhere.len() > BOILERPLATE_FILES {
                continue;
            }
            for source in kept {
                *copies.entry((source.to_string(), target)).or_default() += 1;
            }
            for source in elsewhere {
                if language.is_some() && language_for_path(Path::new(source)) == language {
                    *copies.entry((source.clone(), target)).or_default() += 1;
                }
            }
        }
    }

    let mut out_degree: FxHashMap<usize, usize> = FxHashMap::default();
    let mut in_degree: FxHashMap<usize, usize> = FxHashMap::default();
    let mut sorted_moves: Vec<(&(usize, usize), &usize)> = moves.iter().collect();
    sorted_moves.sort();
    for ((source, target), count) in sorted_moves {
        *out_degree.entry(*source).or_default() += 1;
        *in_degree.entry(*target).or_default() += 1;
        // The file renamed, as the VCS has it: its blocks went where it went.
        if hinted.contains(&(*source, *target)) {
            continue;
        }
        fired.insert(Signal::BlockMove);
        evidence.push(format!(
            "block-move: {} -> {} ({count} blocks)",
            files[*source].path, files[*target].path
        ));
    }
    for (degree, word) in [(&out_degree, "split"), (&in_degree, "merge")] {
        let mut several: Vec<(&usize, &usize)> =
            degree.iter().filter(|(_, count)| **count >= 2).collect();
        several.sort();
        for (file, count) in several {
            fired.insert(Signal::SplitOrMerge);
            evidence.push(format!(
                "split-or-merge: {word} of {} ({count} files)",
                files[*file].path
            ));
        }
    }

    let mut copies: Vec<((String, usize), usize)> = copies
        .into_iter()
        .filter(|(_, count)| *count >= COPY_BLOCKS)
        .collect();
    copies.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    // The cap on files is on changed ones, so every changeset has room for its sources.
    let room = MAX_UNTOUCHED_SOURCES;
    let mut sources: Vec<String> = Vec::new();
    for ((source, target), count) in copies {
        if !index_of.contains_key(source.as_str()) && !sources.contains(&source) {
            // `;` separates the paths in `changesets.csv`.
            if sources.len() >= room || source.contains(';') {
                continue;
            }
            sources.push(source.clone());
        }
        fired.insert(Signal::Copy);
        evidence.push(format!(
            "copy: {source} -> {} ({count} blocks)",
            files[target].path
        ));
    }
    sources.sort();

    for (source, from) in analyses.iter().enumerate() {
        let Some(from) = from else { continue };
        if from.removed_lines < MOVED_EDITED_LINES || from.lost_shingles.len() < MIN_SHINGLES {
            continue;
        }
        for (target, to) in analyses.iter().enumerate() {
            let Some(to) = to else { continue };
            if source == target
                || to.added_lines < MOVED_EDITED_LINES
                || to.gained_shingles.len() < MIN_SHINGLES
                || moves.contains_key(&(source, target))
            {
                continue;
            }
            let shared = from.lost_shingles.intersection(&to.gained_shingles).count();
            let union = from.lost_shingles.len() + to.gained_shingles.len() - shared;
            let similarity = shared as f64 / union as f64;
            if similarity >= MOVED_EDITED_SIMILARITY {
                fired.insert(Signal::MovedAndEdited);
                evidence.push(format!(
                    "moved-and-edited: {} -> {} ({similarity:.2})",
                    files[source].path, files[target].path
                ));
            }
        }
    }

    let mut signals: Vec<Signal> = fired.into_iter().collect();
    signals.sort();
    Classification {
        signals,
        untouched: sources,
        evidence,
    }
}

/// Why a commit is not a changeset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Skip {
    TooFewFiles,
    TooManyFiles,
    FileTooLarge,
    TooManyLines,
    NoFolder,
}

impl Skip {
    pub fn name(self) -> &'static str {
        match self {
            Skip::TooFewFiles => "fewer than 2 files",
            Skip::TooManyFiles => "more than 20 files",
            Skip::FileTooLarge => "a file over 1 MiB",
            Skip::TooManyLines => "over 3000 changed lines",
            Skip::NoFolder => "no sub-folder under the caps",
        }
    }
}

fn is_regular(mode: FileMode) -> bool {
    matches!(
        mode,
        FileMode::Blob | FileMode::BlobGroupWritable | FileMode::BlobExecutable
    )
}

/// True for a repository path a fixture can hold under its own directory: relative, and nothing
/// but plain components (no `..`, `.`, `.git` or empty one, no backslash). Git refuses most of
/// the rest itself; a fixture is written from a repository nobody here vouches for.
pub fn is_safe_path(path: &str) -> bool {
    !path.contains(['\\', '\0'])
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".." | ".git"))
}

/// [`anomalous_paths::is_anomalous`] of a path relative to the repository root.
fn is_anomalous(path: &str) -> bool {
    anomalous_paths::is_anomalous(Path::new(&format!("/{path}")))
}

fn is_lock_file(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .is_some_and(|name| LOCK_FILES.iter().any(|lock| name == *lock))
}

/// The files `after` changes from `before` that a changeset holds (see the module documentation),
/// sorted by path, or why the commit is not one. The changed-line cap is the caller's
/// ([`changed_lines`]): a fixture is materialised from a row that already passed it.
pub fn load_changes(
    repo: &Repository,
    before: &Tree,
    after: &Tree,
) -> Result<std::result::Result<Vec<ChangedFile>, Skip>> {
    let diff = repo.diff_tree_to_tree(Some(before), Some(after), None)?;
    let mut candidates = Vec::new();
    for delta in diff.deltas() {
        let (old, new) = (delta.old_file(), delta.new_file());
        let (has_old, has_new) = (!old.id().is_zero(), !new.id().is_zero());
        if (has_old && !is_regular(old.mode())) || (has_new && !is_regular(new.mode())) {
            continue;
        }
        if has_old && has_new && old.id() == new.id() {
            continue;
        }
        let Some(path) = new.path().or(old.path()) else {
            continue;
        };
        let Some(text) = path.to_str() else { continue };
        if is_anomalous(text) || is_lock_file(text) || !is_safe_path(text) {
            continue;
        }
        candidates.push((
            text.to_string(),
            has_old.then(|| old.id()),
            has_new.then(|| new.id()),
        ));
    }
    if candidates.len() < MIN_FILES {
        return Ok(Err(Skip::TooFewFiles));
    }
    // Generated files are only dropped below, by content, so the count may still fall; but not
    // by this much, and reading hundreds of blobs to find out would be the walk's whole cost.
    if candidates.len() > 4 * MAX_FILES {
        return Ok(Err(Skip::TooManyFiles));
    }
    let mut files = Vec::new();
    for (path, old, new) in candidates {
        let side = |id: Option<Oid>| -> Result<Option<Blob>> {
            let Some(id) = id else { return Ok(None) };
            let blob = repo.find_blob(id)?;
            Ok(Some(Blob {
                id,
                bytes: blob.content().to_vec(),
            }))
        };
        let (before, after) = (side(old)?, side(new)?);
        let sides = [&before, &after];
        if sides.iter().any(|side| {
            side.as_ref()
                .is_some_and(|blob| blob.bytes.len() > MAX_FILE_BYTES)
        }) {
            return Ok(Err(Skip::FileTooLarge));
        }
        let generated = sides.iter().any(|side| {
            side.as_ref()
                .and_then(|blob| std::str::from_utf8(&blob.bytes).ok())
                .is_some_and(crate::stats::is_generated)
        });
        if !generated {
            files.push(ChangedFile {
                path,
                before,
                after,
            });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(match files.len() {
        0..MIN_FILES => Err(Skip::TooFewFiles),
        MIN_FILES..=MAX_FILES => Ok(files),
        _ => Err(Skip::TooManyFiles),
    })
}

/// The bytes of the regular file at `path` in `tree`; an error for a link, a submodule or a
/// directory.
pub fn regular_blob(repo: &Repository, tree: &Tree, path: &str) -> Result<Vec<u8>> {
    if !is_safe_path(path) {
        bail!("{path:?} is not a path a fixture can hold");
    }
    let entry = tree.get_path(Path::new(path))?;
    if entry.kind() != Some(ObjectType::Blob) || !matches!(entry.filemode(), 0o100644 | 0o100755) {
        bail!("{path:?} is not a regular file");
    }
    Ok(repo.find_blob(entry.id())?.content().to_vec())
}

/// Two complete trees of one sub-folder (see the module documentation), paths relative to it.
pub struct Folder {
    /// The sub-folder's path in the repository; empty for the root.
    pub root: String,
    pub changed: Vec<ChangedFile>,
    /// Files with one blob on both sides, sorted by path.
    pub unchanged: Vec<(String, Vec<u8>)>,
    pub omitted: Vec<Omitted>,
}

impl Folder {
    /// The sub-folder as `changesets.csv` and `source.json` name it: `.` for the root.
    pub fn root_name(&self) -> &str {
        if self.root.is_empty() {
            "."
        } else {
            &self.root
        }
    }

    /// The unchanged files that are text, as the untouched files of a classification.
    pub fn untouched(&self) -> UntouchedTexts {
        UntouchedTexts::new(self.unchanged.iter().filter_map(|(path, bytes)| {
            std::str::from_utf8(bytes)
                .ok()
                .map(|text| (path.as_str(), text))
        }))
    }
}

/// The tree at `root` (empty for `tree` itself); `None` when `tree` has no such directory.
fn subtree<'repo>(repo: &'repo Repository, tree: &Tree<'repo>, root: &str) -> Option<Tree<'repo>> {
    if root.is_empty() {
        return Some(tree.clone());
    }
    let entry = tree.get_path(Path::new(root)).ok()?;
    repo.find_tree(entry.id()).ok()
}

/// Every entry under `tree` that is not a directory: its id and file mode, by path from `prefix`.
fn entries_under(
    repo: &Repository,
    tree: &Tree,
    prefix: &str,
    found: &mut std::collections::BTreeMap<String, (Oid, i32)>,
) {
    for entry in tree.iter() {
        let Ok(name) = entry.name() else { continue };
        let path = format!("{prefix}{name}");
        if entry.kind() == Some(ObjectType::Tree) {
            if let Ok(inner) = repo.find_tree(entry.id()) {
                entries_under(repo, &inner, &format!("{path}/"), found);
            }
        } else {
            found.insert(path, (entry.id(), entry.filemode()));
        }
    }
}

/// How many files `tree` holds, counted no further than past `limit`.
fn count_files(repo: &Repository, tree: &Tree, limit: usize) -> usize {
    let mut count = 0;
    for entry in tree.iter() {
        if entry.kind() == Some(ObjectType::Tree) {
            if let Ok(inner) = repo.find_tree(entry.id()) {
                count += count_files(repo, &inner, limit.saturating_sub(count));
            }
        } else {
            count += 1;
        }
        if count > limit {
            break;
        }
    }
    count
}

fn is_regular_mode(mode: i32) -> bool {
    matches!(mode, 0o100644 | 0o100664 | 0o100755)
}

/// True for a path in a language OmniDiff parses with a tree-sitter grammar: a file a person
/// solving a changeset maps node by node, not line by line.
pub fn is_code_path(path: &str) -> bool {
    language_for_path(Path::new(path))
        .is_some_and(|language| crate::code::language::to_treesitter(&language).is_some())
}

/// True for a directory a folders fixture may be: the root, or a path none of whose components
/// starts with a dot.
fn is_folder_candidate(directory: &str) -> bool {
    directory.is_empty()
        || (is_safe_path(directory) && !directory.split('/').any(|part| part.starts_with('.')))
}

/// The sub-folder a folders fixture of `before` and `after` holds, by the rule in the module
/// documentation; [`Skip::NoFolder`] when no directory qualifies.
pub fn choose_folder(
    repo: &Repository,
    before: &Tree,
    after: &Tree,
    max_tree_files: usize,
) -> Result<std::result::Result<String, Skip>> {
    let diff = repo.diff_tree_to_tree(Some(before), Some(after), None)?;
    let odb = repo.odb()?;
    // Each changed file's path, and whether a side of it is over the size cap.
    let mut changed: Vec<(String, bool)> = Vec::new();
    for delta in diff.deltas() {
        let (old, new) = (delta.old_file(), delta.new_file());
        let regular = |file: &git2::DiffFile| !file.id().is_zero() && is_regular(file.mode());
        let (has_old, has_new) = (regular(&old), regular(&new));
        if !(has_old || has_new) || (has_old && has_new && old.id() == new.id()) {
            continue;
        }
        let Some(path) = new.path().or(old.path()).and_then(Path::to_str) else {
            continue;
        };
        let too_large = |present: bool, id: Oid| {
            present
                && odb
                    .read_header(id)
                    .is_ok_and(|(size, _)| size > MAX_FILE_BYTES)
        };
        changed.push((
            path.to_string(),
            too_large(has_old, old.id()) || too_large(has_new, new.id()),
        ));
    }
    /// What a directory's changed files come to; the root is "".
    #[derive(Default)]
    struct Changed {
        files: usize,
        code: usize,
        too_large: bool,
    }
    let mut directories: FxHashMap<&str, Changed> = FxHashMap::default();
    for (path, too_large) in &changed {
        let code = is_code_path(path);
        let mut directory = path.as_str();
        loop {
            directory = directory.rsplit_once('/').map_or("", |(parent, _)| parent);
            let entry = directories.entry(directory).or_default();
            entry.files += 1;
            entry.code += usize::from(code);
            entry.too_large |= too_large;
            if directory.is_empty() {
                break;
            }
        }
    }
    let depth = |directory: &str| match directory {
        "" => 0,
        _ => directory.split('/').count(),
    };
    let mut candidates: Vec<(&str, Changed)> = directories
        .into_iter()
        .filter(|(directory, changed)| {
            (MIN_FILES..=MAX_FILES).contains(&changed.files)
                && changed.code > 0
                && !changed.too_large
                && is_folder_candidate(directory)
        })
        .collect();
    candidates.sort_by(|a, b| {
        (b.1.code, b.1.files, depth(b.0))
            .cmp(&(a.1.code, a.1.files, depth(a.0)))
            .then_with(|| a.0.cmp(b.0))
    });
    let max_tree_files = max_tree_files.min(MAX_TREE_FILES);
    for (directory, _) in candidates {
        let fits = |tree: &Tree| {
            subtree(repo, tree, directory)
                .is_none_or(|tree| count_files(repo, &tree, max_tree_files) <= max_tree_files)
        };
        if fits(before) && fits(after) {
            return Ok(Ok(directory.to_string()));
        }
    }
    Ok(Err(Skip::NoFolder))
}

/// The complete trees of `root` in `before` and `after` (see the module documentation). An
/// error for a path a fixture cannot hold, or a changed file over [`MAX_FILE_BYTES`], which
/// [`choose_folder`] never picks a folder with.
pub fn load_folder(repo: &Repository, before: &Tree, after: &Tree, root: &str) -> Result<Folder> {
    if !(root.is_empty() || is_safe_path(root)) {
        bail!("{root:?} is not a folder a fixture can hold");
    }
    let mut sides = [
        std::collections::BTreeMap::new(),
        std::collections::BTreeMap::new(),
    ];
    for (tree, found) in [before, after].into_iter().zip(sides.iter_mut()) {
        if let Some(tree) = subtree(repo, tree, root) {
            entries_under(repo, &tree, "", found);
        }
    }
    let [old, new] = sides;
    let odb = repo.odb()?;
    let mut folder = Folder {
        root: root.to_string(),
        changed: Vec::new(),
        unchanged: Vec::new(),
        omitted: Vec::new(),
    };
    // What a side holds at a path that is not a regular file is listed, and is no file there.
    let mut regular = |side: &'static str, path: &str, entry: Option<&(Oid, i32)>| {
        let (id, mode) = entry?;
        if is_regular_mode(*mode) {
            return Some(*id);
        }
        let submodule = *mode == 0o160000;
        folder.omitted.push(Omitted {
            side: side.to_string(),
            path: path.to_string(),
            reason: if submodule { "submodule" } else { "symlink" }.to_string(),
            bytes: odb.read_header(*id).ok().map(|(size, _)| size as u64),
            id: id.to_string(),
        });
        None
    };
    let paths: std::collections::BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    let mut files: Vec<(&String, Option<Oid>, Option<Oid>)> = Vec::new();
    for path in paths {
        if !is_safe_path(path) {
            bail!("{path:?} is not a path a fixture can hold");
        }
        let before = regular("before", path, old.get(path));
        let after = regular("after", path, new.get(path));
        if before.is_some() || after.is_some() {
            files.push((path, before, after));
        }
    }
    for (path, before, after) in files {
        let size = |id: Oid| odb.read_header(id).map_or(0, |(size, _)| size);
        let bytes = |id: Oid| -> Result<Vec<u8>> { Ok(repo.find_blob(id)?.content().to_vec()) };
        if before == after {
            let id = before.unwrap_or(Oid::ZERO_SHA1);
            if size(id) > MAX_FILE_BYTES {
                for side in ["before", "after"] {
                    folder.omitted.push(Omitted {
                        side: side.to_string(),
                        path: path.clone(),
                        reason: "size".to_string(),
                        bytes: Some(size(id) as u64),
                        id: id.to_string(),
                    });
                }
            } else {
                folder.unchanged.push((path.clone(), bytes(id)?));
            }
            continue;
        }
        let side = |id: Option<Oid>| -> Result<Option<Blob>> {
            let Some(id) = id else { return Ok(None) };
            if size(id) > MAX_FILE_BYTES {
                bail!("{path:?} changes and is over the size cap");
            }
            Ok(Some(Blob {
                id,
                bytes: bytes(id)?,
            }))
        };
        folder.changed.push(ChangedFile {
            path: path.clone(),
            before: side(before)?,
            after: side(after)?,
        });
    }
    folder
        .omitted
        .sort_by(|a, b| (&a.path, &a.side).cmp(&(&b.path, &b.side)));
    Ok(folder)
}

/// What `git diff --name-status -M -z` prints.
pub fn parse_name_status(output: &[u8]) -> Vec<VcsHint> {
    let mut fields = output
        .split(|byte| *byte == 0)
        .map(|field| String::from_utf8_lossy(field).into_owned());
    let mut hints = Vec::new();
    while let Some(status) = fields.next() {
        let Some(letter) = status.chars().next() else {
            continue;
        };
        let similarity = status[1..].parse::<u8>().ok();
        let Some(first) = fields.next() else { break };
        if matches!(letter, 'R' | 'C') {
            let Some(path) = fields.next() else { break };
            hints.push(VcsHint {
                status: letter,
                path,
                old_path: Some(first),
                similarity,
            });
        } else {
            hints.push(VcsHint {
                status: letter,
                path: first,
                old_path: None,
                similarity: None,
            });
        }
    }
    hints
}

/// Git's own account of `parent..commit`, renames detected (`-M`, git's default threshold).
pub fn vcs_hints(repo: &Repository, parent: Oid, commit: Oid) -> Result<Vec<VcsHint>> {
    let output = std::process::Command::new("git")
        .arg("--git-dir")
        .arg(repo.path())
        .args(["diff", "--name-status", "-M", "-z", "--no-ext-diff"])
        .arg(parent.to_string())
        .arg(commit.to_string())
        .arg("--")
        .output()?;
    if !output.status.success() {
        bail!(
            "git diff --name-status failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(parse_name_status(&output.stdout))
}

/// The blocks of a repository's files, by path, built up tree by tree as a walk goes back through
/// history: which paths have ever held a block, to be checked against one tree ([`TreeView`]).
/// Only files with a language, in UTF-8 and at most [`UNTOUCHED_MAX_BYTES`] long are indexed.
#[derive(Default)]
pub struct BlockIndex {
    paths: Vec<String>,
    path_ids: FxHashMap<String, u32>,
    by_block: FxHashMap<u64, Vec<u32>>,
    /// A directory already indexed with this content, and a file.
    seen_trees: FxHashSet<(String, Oid)>,
    seen_blobs: FxHashSet<(u32, Oid)>,
    blob_blocks: RefCell<FxHashMap<Oid, Rc<FxHashSet<u64>>>>,
}

impl BlockIndex {
    /// Indexes what `tree` holds that no tree before it did.
    pub fn add_tree(&mut self, repo: &Repository, tree: &Tree) {
        self.add_directory(repo, tree, "");
    }

    fn add_directory(&mut self, repo: &Repository, tree: &Tree, prefix: &str) {
        if !self.seen_trees.insert((prefix.to_string(), tree.id())) {
            return;
        }
        for entry in tree.iter() {
            let Ok(name) = entry.name() else { continue };
            let path = format!("{prefix}{name}");
            match entry.kind() {
                Some(ObjectType::Tree) => {
                    if let Ok(subtree) = repo.find_tree(entry.id()) {
                        self.add_directory(repo, &subtree, &format!("{path}/"));
                    }
                }
                Some(ObjectType::Blob) if matches!(entry.filemode(), 0o100644 | 0o100755) => {
                    if language_for_path(Path::new(&path)).is_none() || is_anomalous(&path) {
                        continue;
                    }
                    let id = match self.path_ids.get(&path) {
                        Some(id) => *id,
                        None if self.paths.len() >= UNTOUCHED_MAX_PATHS => continue,
                        None => {
                            let id = self.paths.len() as u32;
                            self.paths.push(path.clone());
                            self.path_ids.insert(path, id);
                            id
                        }
                    };
                    if !self.seen_blobs.insert((id, entry.id())) {
                        continue;
                    }
                    let Ok(blob) = repo.find_blob(entry.id()) else {
                        continue;
                    };
                    if blob.content().len() > UNTOUCHED_MAX_BYTES {
                        continue;
                    }
                    let Ok(text) = std::str::from_utf8(blob.content()) else {
                        continue;
                    };
                    for block in blocks_of(text) {
                        let holders = self.by_block.entry(block).or_default();
                        if holders.len() < INDEX_PATHS_PER_BLOCK && !holders.contains(&id) {
                            holders.push(id);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// The index as `tree`'s untouched files: every indexed path but `changed`.
    pub fn view<'a>(
        &'a self,
        repo: &'a Repository,
        tree: &'a Tree<'a>,
        changed: &'a [ChangedFile],
    ) -> TreeView<'a> {
        TreeView {
            index: self,
            repo,
            tree,
            changed: changed.iter().map(|file| file.path.as_str()).collect(),
        }
    }

    fn blocks_of_blob(&self, repo: &Repository, id: Oid) -> Rc<FxHashSet<u64>> {
        let mut cache = self.blob_blocks.borrow_mut();
        if cache.len() > 4096 {
            cache.clear();
        }
        cache
            .entry(id)
            .or_insert_with(|| {
                let blocks = repo
                    .find_blob(id)
                    .ok()
                    .and_then(|blob| std::str::from_utf8(blob.content()).ok().map(blocks_of))
                    .unwrap_or_default();
                Rc::new(blocks)
            })
            .clone()
    }
}

/// A [`BlockIndex`] seen from one tree of the repository.
pub struct TreeView<'a> {
    index: &'a BlockIndex,
    repo: &'a Repository,
    tree: &'a Tree<'a>,
    changed: FxHashSet<&'a str>,
}

impl Untouched for TreeView<'_> {
    fn holders(&self, block: u64) -> Holders {
        let Some(candidates) = self.index.by_block.get(&block) else {
            return Holders::Paths(Vec::new());
        };
        if candidates.len() >= INDEX_PATHS_PER_BLOCK {
            return Holders::Boilerplate;
        }
        let mut paths = Vec::new();
        for id in candidates {
            let path = &self.index.paths[*id as usize];
            if self.changed.contains(path.as_str()) {
                continue;
            }
            // The path held the block in some tree; whether in this one is the blob's say.
            let Ok(entry) = self.tree.get_path(Path::new(path)) else {
                continue;
            };
            if entry.kind() == Some(ObjectType::Blob)
                && self
                    .index
                    .blocks_of_blob(self.repo, entry.id())
                    .contains(&block)
            {
                paths.push(path.clone());
                if paths.len() > BOILERPLATE_FILES {
                    return Holders::Boilerplate;
                }
            }
        }
        Holders::Paths(paths)
    }
}

/// A row of `changesets.csv` (see the module documentation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The fixture's directory under `samples/changesets/` ([`fixture_name`]).
    pub name: String,
    /// [`KIND_COMMIT`] or [`KIND_FOLDERS`].
    pub kind: String,
    pub repository: String,
    /// The after tree's commit.
    pub commit: String,
    /// The before tree's commit: `commit`'s parent, or for folders any earlier point in history.
    pub parent: String,
    /// The sub-folder of a `folders` row, `.` for the repository root; empty for a commit.
    pub root: String,
    /// Every signal that fired, sorted and `;`-separated, or `control`.
    pub signals: String,
    /// The untouched files a signal points at, `;`-separated.
    pub untouched_paths: String,
    /// Changed files.
    pub files: usize,
    pub changed_lines: usize,
    pub main_language: String,
    pub bucket: String,
    pub dataset: String,
    /// `SAMPLED`, then `PROMOTED` or `REJECTED` once triaged.
    pub status: String,
    pub comment: String,
}

pub const CSV_HEADER: [&str; 15] = [
    "name",
    "kind",
    "repository",
    "commit",
    "parent",
    "root",
    "signals",
    "untouched_paths",
    "files",
    "changed_lines",
    "main_language",
    "bucket",
    "dataset",
    "status",
    "comment",
];

/// The `signals` column of a classification.
pub fn signals_column(classification: &Classification) -> String {
    if classification.signals.is_empty() {
        return CONTROL.to_string();
    }
    let names: Vec<&str> = classification.signals.iter().map(|s| s.name()).collect();
    names.join(";")
}

impl Row {
    /// The untouched files the fixture holds because a signal points at them.
    pub fn untouched(&self) -> Vec<&str> {
        self.untouched_paths
            .split(';')
            .filter(|path| !path.is_empty())
            .collect()
    }

    /// What the row counts towards: each of its signals, or [`CONTROL`].
    pub fn signal_names(&self) -> Vec<&str> {
        self.signals
            .split(';')
            .filter(|name| !name.is_empty())
            .collect()
    }
}

/// `commit-x-<repository>-<commit>` or `folders-x-<repository>-<parent>-<commit>`: lower case,
/// the repository without a trailing `.git`, commits abbreviated to 8 characters, as
/// `materialize_test_diffs` names a single-file sample. No signal is in the name: a changeset
/// has several, and a person triaging it may find another.
pub fn fixture_name(kind: &str, repository: &str, parent: &str, commit: &str) -> String {
    let repository = repository.strip_suffix(".git").unwrap_or(repository);
    let short = |id: &str| id[..8.min(id.len())].to_string();
    if kind == KIND_FOLDERS {
        format!(
            "{kind}-x-{}-{}-{}",
            repository.to_lowercase(),
            short(parent),
            short(commit)
        )
    } else {
        format!("{kind}-x-{}-{}", repository.to_lowercase(), short(commit))
    }
}

/// The rows of `path`; none when there is no such file.
pub fn read_rows(path: &Path) -> Result<Vec<Row>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut reader = csv::Reader::from_path(path)?;
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let field = |index: usize| record.get(index).unwrap_or("").to_string();
        rows.push(Row {
            name: field(0),
            kind: field(1),
            repository: field(2),
            commit: field(3),
            parent: field(4),
            root: field(5),
            signals: field(6),
            untouched_paths: field(7),
            files: field(8).parse().unwrap_or(0),
            changed_lines: field(9).parse().unwrap_or(0),
            main_language: field(10),
            bucket: field(11),
            dataset: field(12),
            status: field(13),
            comment: field(14),
        });
    }
    Ok(rows)
}

/// Writes `rows` to `path`, sorted by kind, repository, commit and parent.
pub fn write_rows(path: &Path, mut rows: Vec<Row>) -> Result<()> {
    rows.sort_by(|a, b| {
        (&a.kind, &a.repository, &a.commit, &a.parent).cmp(&(
            &b.kind,
            &b.repository,
            &b.commit,
            &b.parent,
        ))
    });
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record(CSV_HEADER)?;
    for row in &rows {
        writer.write_record([
            &row.name,
            &row.kind,
            &row.repository,
            &row.commit,
            &row.parent,
            &row.root,
            &row.signals,
            &row.untouched_paths,
            &row.files.to_string(),
            &row.changed_lines.to_string(),
            &row.main_language,
            &row.bucket,
            &row.dataset,
            &row.status,
            &row.comment,
        ])?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests;
