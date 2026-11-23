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
//! The library API: [`Diff`], re-exported as `omnidiff::Diff`, and the types it hands out.
//!
//! Everything else in the crate is the binary's internals made visible; this module is the part
//! meant for other crates, and it is shaped so that it can stay put while those change:
//!
//! - **One owned result, views computed on first use.** A [`Diff`] owns both sides and the
//!   engine's node mapping. The ranges of a preset, the definitions of a side and the definition
//!   changes are each built the first time they are asked for and kept, as `similar`'s `ops()`
//!   and imara-diff 0.2's owned `Diff` do; nothing borrows the caller's strings or runs callbacks.
//!   Before this module, `diff_strings` returned node ids into two trees it had already dropped,
//!   and nothing set `diff::Diff::text`, so a library caller got neither the nodes nor the ranges.
//! - **No internal types in signatures.** Positions are byte ranges plus 0-based (row, byte
//!   column) [`Position`]s, never `tree_sitter` points or `TextRange`s; every enum and every
//!   struct with public fields is `#[non_exhaustive]`, so adding a case is not a breaking change.
//! - **Nothing is refused.** A pair without a grammar, or over the parse size cap, is diffed line
//!   by line ([`Diff::is_structural`] is then false and there are no definitions), the way the
//!   viewer shows it.
//! - **The engine is untouched.** This module only reads the node mapping and the painted
//!   ranges; nothing here feeds back into matching, so the quality gates cannot move because of it.
//!
//! Definitions (functions, types, modules, impls...) and what happened to each are in
//! [`definitions`]; their qualified names in `qualified_name`.

mod changes;
pub mod definitions;
#[cfg(feature = "git")]
pub mod git;
mod qualified_name;

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::code::{Code, Language, decode_text, language::language_for_path_and_content};
use crate::diff::{
    NodeCache,
    text::{
        RangeMatch, RenderOptions, TextDiff, TextOperation, plain_text_line_diff,
        ranges_for_options,
    },
    text_range::{SourceColumn, SourceRow, SourceText, TextRange},
};

pub use definitions::{ChangeKind, Definition, DefinitionChange, DefinitionKind};
use qualified_name::{FileSystem, ProjectFiles};

/// One side of a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Before,
    After,
}

impl Side {
    fn index(self) -> usize {
        match self {
            Side::Before => 0,
            Side::After => 1,
        }
    }
}

/// How much of a change [`Diff::ranges`] reports, as the viewer's `--minimal` and `--maximal`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Preset {
    /// Only what changed: no whitespace, no punctuation that merely shifted. What a reviewer
    /// needs, and what [`DefinitionChange::kind`] judges "modified" by.
    #[default]
    Minimal,
    /// Everything that changed, including re-indentation and changed trailing whitespace.
    Maximal,
}

impl Preset {
    fn options(self) -> RenderOptions {
        match self {
            Preset::Minimal => RenderOptions::MINIMAL,
            Preset::Maximal => RenderOptions::MAXIMAL,
        }
    }

    fn index(self) -> usize {
        match self {
            Preset::Minimal => 0,
            Preset::Maximal => 1,
        }
    }
}

/// A point in a side's text: 0-based row, and a 0-based column in **bytes** from the row's start
/// (tree-sitter's unit, not characters or screen cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

/// A right-open stretch of a side's text, as a byte range and as positions. An end at the end of
/// a row is written `(row + 1, 0)`. An empty span marks a place, such as where an insertion sits
/// on the side that does not have it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub bytes: Range<usize>,
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// What happened to a stretch of text.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    /// Only in the after side.
    Insert,
    /// Only in the before side.
    Delete,
    /// The same construct, with different text.
    Update,
    /// The same text, somewhere else.
    Move,
}

/// One changed stretch of one side's text ([`Diff::ranges`]).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextChange {
    pub operation: Operation,
    /// The stretch, in the side it was asked for.
    pub span: Span,
    /// For a [`Operation::Move`], where the same text is in the other side.
    pub other: Option<Span>,
}

/// How big a [`Diff`] is, in lines ([`Diff::stats`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Stats {
    inserted: usize,
    deleted: usize,
    updated: usize,
    moved: usize,
}

impl Stats {
    /// Lines only in the after side.
    pub fn inserted(&self) -> usize {
        self.inserted
    }

    /// Lines only in the before side.
    pub fn deleted(&self) -> usize {
        self.deleted
    }

    /// Lines of the after side whose construct is on both sides, with different text.
    pub fn updated(&self) -> usize {
        self.updated
    }

    /// Lines of the after side that are the same text somewhere else in the before side.
    pub fn moved(&self) -> usize {
        self.moved
    }

    /// Whether nothing changed but whitespace the preset ignores, or nothing at all.
    pub fn is_empty(&self) -> bool {
        *self == Stats::default()
    }
}

impl std::ops::Add for Stats {
    type Output = Stats;

    fn add(self, other: Stats) -> Stats {
        Stats {
            inserted: self.inserted + other.inserted,
            deleted: self.deleted + other.deleted,
            updated: self.updated + other.updated,
            moved: self.moved + other.moved,
        }
    }
}

impl std::iter::Sum for Stats {
    fn sum<I: Iterator<Item = Stats>>(iter: I) -> Stats {
        iter.fold(Stats::default(), |a, b| a + b)
    }
}

/// Why [`Diff::files`] could not diff a pair.
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// A file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A file is not text: not UTF-8, and not UTF-16 or UTF-32 with a byte order mark.
    NotText { path: PathBuf },
    /// A git operation failed: the repository could not be opened, a revision did not resolve,
    /// an object could not be read.
    #[cfg(feature = "git")]
    Git(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Error::NotText { path } => write!(f, "{} is not text", path.display()),
            #[cfg(feature = "git")]
            Error::Git(source) => write!(f, "git: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::NotText { .. } => None,
            #[cfg(feature = "git")]
            Error::Git(source) => Some(&**source),
        }
    }
}

/// The difference between two versions of a file.
///
/// ```
/// use omnidiff::{Diff, Language};
///
/// let before = "fn parse() {}\nfn lex() {}\n";
/// let after = "fn parse() { todo!() }\nfn lex() {}\nfn peek() {}\n";
/// let diff = Diff::new(before, after, Language::Rust);
///
/// let changes: Vec<String> = diff
///     .changed_functions()
///     .map(|c| format!("{} {}", c.kind(), c.qualified_name()))
///     .collect();
/// assert_eq!(changes, ["modified parse", "added peek"]);
/// ```
pub struct Diff {
    sides: [Code; 2],
    paths: [Option<PathBuf>; 2],
    /// Where each side's project files (`go.mod`, `__init__.py`) are read for qualified names:
    /// the filesystem, or the git tree the side came from.
    projects: [Arc<dyn ProjectFiles + Send + Sync>; 2],
    /// The engine's result; `None` when either side has no syntax tree.
    engine: Option<crate::diff::Diff>,
    /// Per [`Preset`]: each side's ranges, filtered for it.
    ranges: [OnceLock<[Vec<RangeMatch>; 2]>; 2],
    definitions: OnceLock<definitions::Definitions>,
}

impl std::fmt::Debug for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Diff")
            .field("language", &self.language())
            .field("paths", &self.paths)
            .field("structural", &self.is_structural())
            .finish_non_exhaustive()
    }
}

impl Diff {
    /// Diffs two versions of a program given as text, both in `language`.
    ///
    /// Names of definitions are relative to the file ([`DefinitionChange::qualified_name`]);
    /// [`Diff::with_paths`] says where the file sits, for a full name.
    pub fn new(before: &str, after: &str, language: Language) -> Self {
        let (before, after) = crate::code::build_sides(
            crate::code::worth_building_in_parallel(before, after),
            || Code::from_string(before, &language),
            || Code::from_string(after, &language),
        );
        Self::from_codes(before, after)
    }

    /// Reads and diffs two files, each in the language its name and contents say. An empty side
    /// (git's `/dev/null` for an added or deleted file) takes the other side's language, so it
    /// reads as a whole-file insertion or deletion.
    pub fn files(before: impl AsRef<Path>, after: impl AsRef<Path>) -> Result<Self, Error> {
        let (before, after) = (before.as_ref(), after.as_ref());
        let read = |path: &Path| -> Result<String, Error> {
            let bytes = std::fs::read(path).map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
            decode_text(&bytes).ok_or_else(|| Error::NotText {
                path: path.to_path_buf(),
            })
        };
        let (before_text, after_text) = (read(before)?, read(after)?);
        Ok(Self::from_texts(&before_text, &after_text, before, after))
    }

    /// Two sides read from anywhere (files, a git object store, an editor's buffer), named by
    /// `before_path` and `after_path`, which pick each side's language as [`Diff::files`] does
    /// and place the file for qualified names. The paths need not exist.
    ///
    /// ```
    /// use omnidiff::{Diff, Operation, Side};
    ///
    /// let diff = Diff::from_texts("a = 1\n", "a = 1\nb = 2\n", "x.py".as_ref(), "x.py".as_ref());
    /// assert_eq!(diff.language(), omnidiff::Language::Python);
    /// let after = diff.line_operations(Side::After, Default::default());
    /// assert_eq!(after[..2], [None, Some(Operation::Insert)]);
    /// ```
    pub fn from_texts(
        before_text: &str,
        after_text: &str,
        before_path: &Path,
        after_path: &Path,
    ) -> Self {
        let detected = [
            language_for_path_and_content(before_path, before_text),
            language_for_path_and_content(after_path, after_text),
        ];
        let language_of = |text: &str, own: Option<Language>, other: Option<Language>| {
            if text.is_empty() { own.or(other) } else { own }.unwrap_or(Language::Unknown)
        };
        let before_language = language_of(before_text, detected[0], detected[1]);
        let after_language = language_of(after_text, detected[1], detected[0]);
        let (before, after) = crate::code::build_sides(
            crate::code::worth_building_in_parallel(before_text, after_text),
            || Code::from_string(before_text, &before_language),
            || Code::from_string(after_text, &after_language),
        );
        Self::from_codes(before, after).with_paths(before_path, after_path)
    }

    fn from_codes(before: Code, after: Code) -> Self {
        let engine = (before.ast.is_some() && after.ast.is_some())
            .then(|| crate::diff::Diff::from_code(&before, &after));
        Self {
            sides: [before, after],
            paths: [None, None],
            projects: [Arc::new(FileSystem), Arc::new(FileSystem)],
            engine,
            ranges: [OnceLock::new(), OnceLock::new()],
            definitions: OnceLock::new(),
        }
    }

    /// Where each side's file sits, so definitions get their full qualified name: the Rust
    /// crate path from `src/`, the Python package from its `__init__.py` files, the Go import
    /// path from `go.mod`. A path that does not exist on disk still places the file by its
    /// components. Changes nothing else about the diff.
    pub fn with_paths(mut self, before: impl AsRef<Path>, after: impl AsRef<Path>) -> Self {
        self.paths = [
            Some(before.as_ref().to_path_buf()),
            Some(after.as_ref().to_path_buf()),
        ];
        self.definitions = OnceLock::new();
        self
    }

    /// Reads each side's project files from `before` and `after` instead of the filesystem.
    #[cfg(feature = "git")]
    pub(crate) fn with_project_files(
        mut self,
        before: Arc<dyn ProjectFiles + Send + Sync>,
        after: Arc<dyn ProjectFiles + Send + Sync>,
    ) -> Self {
        self.projects = [before, after];
        self.definitions = OnceLock::new();
        self
    }

    pub(crate) fn project_files(&self, side: Side) -> &dyn ProjectFiles {
        &*self.projects[side.index()]
    }

    /// The language the diff was read in: the after side's, or the before side's when the after
    /// side has none.
    pub fn language(&self) -> Language {
        [&self.sides[1], &self.sides[0]]
            .into_iter()
            .find_map(|code| code.metadata.language.filter(|l| *l != Language::Unknown))
            .unwrap_or(Language::Unknown)
    }

    /// One side's text.
    pub fn text(&self, side: Side) -> &str {
        &self.sides[side.index()].contents
    }

    /// The path given for a side, by [`Diff::files`] or [`Diff::with_paths`].
    pub fn path(&self, side: Side) -> Option<&Path> {
        self.paths[side.index()].as_deref()
    }

    /// Whether both sides were parsed and matched node by node. False for a language without a
    /// grammar, a file over the size cap, or a pair whose sides are in no common language:
    /// those are diffed line by line, and have no definitions.
    pub fn is_structural(&self) -> bool {
        self.engine.as_ref().is_some_and(|e| e.ast.is_some())
    }

    /// Whether the two sides differ at all, whitespace included.
    pub fn has_changes(&self) -> bool {
        self.sides[0].contents != self.sides[1].contents
    }

    /// Every changed stretch of `side`'s text under `preset`, in document order. Unchanged text
    /// gets no entry.
    pub fn ranges(&self, side: Side, preset: Preset) -> Vec<TextChange> {
        let source = SourceText::new(self.text(side));
        let other = SourceText::new(self.text(match side {
            Side::Before => Side::After,
            Side::After => Side::Before,
        }));
        self.painted(preset)[side.index()]
            .iter()
            .filter(|range| !range.source.is_empty())
            .filter_map(|range| {
                let operation = match range.operation {
                    TextOperation::Insert => Operation::Insert,
                    TextOperation::Delete => Operation::Delete,
                    TextOperation::Update => Operation::Update,
                    TextOperation::Move => Operation::Move,
                    _ => return None,
                };
                Some(TextChange {
                    operation,
                    span: span_of(&range.source, &source)?,
                    // The engine places an insert, delete or update's destination for its own
                    // alignment; only a move's is the text it came from or went to.
                    other: (operation == Operation::Move)
                        .then(|| span_of(&range.destination, &other))
                        .flatten(),
                })
            })
            .collect()
    }

    /// What happened to each line of `side`, one entry per line of its text (the empty one after
    /// a final newline included): the operation of a change touching the line, or `None` for a
    /// line left as it was. Lines, not spans: a line holding an updated token is an updated
    /// line. For a gutter, a status bar, a line-based renderer.
    pub fn line_operations(&self, side: Side, preset: Preset) -> Vec<Option<Operation>> {
        crate::diff::text::line_operations(
            &self.painted(preset)[side.index()],
            self.text(side).split('\n').count(),
        )
        .into_iter()
        .map(|op| match op {
            TextOperation::Insert => Some(Operation::Insert),
            TextOperation::Delete => Some(Operation::Delete),
            TextOperation::Update => Some(Operation::Update),
            TextOperation::Move => Some(Operation::Move),
            _ => None,
        })
        .collect()
    }

    /// How big the diff is, in lines: deleted lines counted on the before side, inserted,
    /// updated and moved ones on the after side, so that a line on both counts once.
    ///
    /// ```
    /// use omnidiff::{Diff, Language};
    ///
    /// let diff = Diff::new("a\nb\n", "a\nc\nd\n", Language::Unknown);
    /// let stats = diff.stats(Default::default());
    /// assert_eq!((stats.deleted(), stats.inserted()), (1, 2));
    /// ```
    pub fn stats(&self, preset: Preset) -> Stats {
        let [before, after] = self.painted(preset);
        let counts = crate::diff::text::change_counts(
            self.text(Side::Before),
            self.text(Side::After),
            before,
            after,
        );
        Stats {
            inserted: counts.insertions,
            deleted: counts.deletions,
            updated: counts.updates,
            moved: counts.moves,
        }
    }

    /// Each side's painted ranges under `preset`, built once.
    fn painted(&self, preset: Preset) -> &[Vec<RangeMatch>; 2] {
        self.ranges[preset.index()].get_or_init(|| {
            let options = preset.options();
            let [before, after] = &self.sides;
            let (before_ranges, after_ranges) =
                match self.engine.as_ref().and_then(|e| e.ast.as_ref()) {
                    // The range builder walks the trees recursively, so it runs on the pipeline's own
                    // big-stack thread, as the engine does.
                    Some(ast) => crate::diff::on_pipeline_stack(|| {
                        let node_cache = NodeCache::build(before, after);
                        let text =
                            TextDiff::from_with_options(before, after, ast, &node_cache, options);
                        (text.all(0), text.all(1))
                    }),
                    None => plain_text_line_diff(&before.contents, &after.contents),
                };
            [
                ranges_for_options(&before_ranges, &before.contents, options),
                ranges_for_options(&after_ranges, &after.contents, options),
            ]
        })
    }

    fn definitions(&self) -> &definitions::Definitions {
        self.definitions
            .get_or_init(|| definitions::Definitions::build(self))
    }

    /// Every definition in `side`, outermost first, in document order: functions, methods,
    /// types, modules, impl blocks and the like (see [`DefinitionKind`]). Empty for a pair that
    /// is not [structural](Diff::is_structural).
    pub fn definitions_of(&self, side: Side) -> &[Definition] {
        &self.definitions().sides[side.index()]
    }

    /// The innermost definition of `side` around byte `offset`, as git's `@@ ... @@ fn foo()`
    /// hunk header names it.
    pub fn enclosing_definition(&self, side: Side, offset: usize) -> Option<&Definition> {
        self.definitions_of(side)
            .iter()
            .filter(|d| d.span().bytes.start <= offset && offset < d.span().bytes.end)
            .max_by_key(|d| d.depth())
    }

    /// Every definition that changed, nested ones included, in document order (the after
    /// side's, with removed definitions where they were).
    pub fn changed_definitions(&self) -> impl Iterator<Item = &DefinitionChange> {
        self.definitions()
            .changes
            .iter()
            .filter(|c| c.kind() != ChangeKind::Unchanged)
    }

    /// The changed definitions directly in the file, not inside another definition. A changed
    /// method shows as its class or impl block, modified.
    pub fn changed_top_level_definitions(&self) -> impl Iterator<Item = &DefinitionChange> {
        self.changed_definitions().filter(|c| c.is_top_level())
    }

    /// Changed functions and methods.
    pub fn changed_functions(&self) -> impl Iterator<Item = &DefinitionChange> {
        self.changed_definitions()
            .filter(|c| c.definition_kind().is_function())
    }

    /// Changed types: structs, enums, unions, classes, interfaces, traits and type aliases.
    pub fn changed_types(&self) -> impl Iterator<Item = &DefinitionChange> {
        self.changed_definitions()
            .filter(|c| c.definition_kind().is_type())
    }

    /// Changed modules, namespaces and packages declared in the file.
    pub fn changed_modules(&self) -> impl Iterator<Item = &DefinitionChange> {
        self.changed_definitions()
            .filter(|c| c.definition_kind() == DefinitionKind::Module)
    }

    /// Every definition, changed or not, paired across the sides.
    pub fn all_definitions(&self) -> impl Iterator<Item = &DefinitionChange> {
        self.definitions().changes.iter()
    }

    pub(crate) fn code(&self, side: Side) -> &Code {
        &self.sides[side.index()]
    }

    pub(crate) fn engine_ast(&self) -> Option<&crate::diff::ASTDiff> {
        self.engine.as_ref().and_then(|e| e.ast.as_ref())
    }

    pub(crate) fn painted_minimal(&self) -> &[Vec<RangeMatch>; 2] {
        self.painted(Preset::Minimal)
    }
}

/// `range` of `text` as a [`Span`]; `None` for a position the text does not have.
fn span_of(range: &TextRange, text: &SourceText) -> Option<Span> {
    let byte = |row: usize, column: usize| {
        text.byte_index(SourceRow::from_raw(row), SourceColumn::from_raw(column))
            .map(|offset| offset.get())
    };
    let start = byte(range.start_row, range.start_column)?;
    let end = byte(range.end_row, range.end_column)?;
    Some(Span {
        bytes: start..end.max(start),
        start: Position {
            row: range.start_row,
            column: range.start_column,
        },
        end: Position {
            row: range.end_row,
            column: range.end_column,
        },
    })
}

#[cfg(test)]
mod tests;
