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
//! The changeset session: building the truth of a whole commit, where code may move between
//! files (`test::helper::human_changeset` is the format and its checks; issue #63).
//!
//! `human_solver <name>` opens it when `<name>` is a directory under
//! `src/test/data/changesets/<dataset>/` or `src/test/data/samples/changesets/`. The tool assumes
//! nothing about which file became which: that is part of the truth, and the human says it.
//!
//! # Three views
//!
//! * **The file board** ([`board_key`], [`draw_board`]) is home: before files left, after files
//!   right, each with its language, its line count, how many of its nodes nothing is said about,
//!   and its relations. A relation from `human_changeset.json` is truth. Where the truth names no
//!   relation for a file, one is **proposed** from `source.json`'s `vcs_hints` and from equal
//!   paths, drawn dim with a `?`: a proposal is never saved and never read by anything until the
//!   human confirms it.
//! * **The pair view** is the code session itself (`run_case_session`: both trees, the `t`
//!   painting view, every key) on any before file and any after file, related or not. While it is
//!   open `App::mapping` is a *projection* of the changeset onto that pair ([`project`]), and
//!   after every edit it is written back and projected again ([`after_key`]), so the changeset
//!   is the one copy of the truth. This is the embedded-layer arrangement (`LayerFrame`) again.
//! * **The leftover pool** ([`pool_rows`]) lists, across the changeset, the outermost subtrees
//!   that have no partner, and pairs those whose full subtree hash (`code::hash`) is equal as
//!   candidates. A candidate is a suggestion: `a` accepts one, and nothing else matches it.
//!
//! # What a pair's projection holds
//!
//! The entries between the two files, one partner a node (the first on file); every one-sided
//! entry of the two files; and for a node whose partners are all *elsewhere* - another file, or a
//! second partner in this pair - a stand-in `delete` or `insert`, so that every single-file code
//! path sees the node as decided and leaves it alone. The panel draws such a node `~` with where
//! its partners are, instead of `-`/`+`, and a matched node with further partners carries `+>`.
//! Stand-ins are never written back ([`write_back`]); [`guard`] puts one back if a key
//! overwrote it, which is what makes a second partner impossible by accident - `m` on a
//! matched-elsewhere node changes nothing, and `+` is the key that adds a partner.
//!
//! Of the paintings, the projection holds the ranges that lie wholly in the two files. A range
//! whose other end is in a third file is not this pair's to edit: it is drawn over the text view
//! with its real verdict (`App::layer_paint`, the channel embedded layers use) and edited in the
//! pair view of its own two files. The keystroke check for overlapping ranges does not see such a
//! range; `validate` does, on save.
//!
//! # Decisions
//!
//! * **Seeding** (`S`) runs the single-file engine on the open pair and is offered only for a
//!   `same` or `renamed` pair with nothing recorded: a match between unrelated files would be a
//!   cross-file match from an engine, which is the thing this truth exists to grade. It takes
//!   the engine's matches and painted matches and nothing else: what a single-file diff calls
//!   deleted or inserted may be in another file, so it stays undecided for the human. (A seeded
//!   delete would also be painted `Delete`, which the pair's invariants accept even after the
//!   node is matched to another file - a wrong painting nothing would report.)
//! * **The pool counts a deleted or inserted subtree as a leftover**, not only an undecided one:
//!   a human working file by file marks the function that left as deleted, and that is exactly
//!   the node whose twin is in another file. Accepting a candidate replaces those marks.
//! * **Candidates are looked for from the outermost leftovers**: each is compared by hash with
//!   every leftover node of the other side, nested ones included, so a function is found inside a
//!   new file. Two files nobody has touched yet are two roots, not a candidate for each of their
//!   functions - seed or solve the files' own pairs first and the pool is what moved.
//! * **A painting is the changeset's, not a pair's.** Branching one in the text view (`s`)
//!   copies the pair's ranges as the code session does, and [`after_key`] then branches the
//!   other files' ranges too ([`branch_elsewhere`]), or the new painting would say they did not
//!   change. From `Minimal` to `Full` it also widens to column 0 a line that left for, or came
//!   from, an unrelated file ([`widen_departed_lines`]): such a line is a deleted or inserted
//!   line to its own file's invariants, which the code session's branch cannot know.
//! * **A non-code pair** (a picture, a binary) gets a verdict from a prompt on the board, in
//!   `human_content`'s levels. The content session's viewer is bound to a content fixture's
//!   directory and its own save, so it is not embedded here.
//! * **Multi-map groups and embedded layers** have no changeset form, so `x`/`X` and `Enter` are
//!   off in the pair view; several partners are several entries (`+`).
//! * `+`, `-`, the pool's `a` and the board's relation keys change the changeset directly and are
//!   not on the pair view's undo stack, which holds projections.
//! * **Promotion**: saving a sample whose every relation is solved and whose truth validates moves
//!   it to `src/test/data/changesets/<dataset>/` and writes its stub. It needs a `README.md` in the
//!   sample, since the corpus requires one saying where the code is from and under what licence.
//!   In the corpus, the stub exists exactly while the changeset is solved.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use omnidiff::code::Code;
use omnidiff::diff::content::{Level, Verdict, Vocabulary};
use omnidiff::diff::{ASTMappingOperation, diff_code};
use omnidiff::test::helper::human_changeset::{
    CHANGESET_DATASETS, Changeset, ChangesetEntry, ChangesetSource, ChangesetTextEntry,
    ChangesetTextMapping, ChangesetTrees, ChangesetViolation, FileRelation, FileRelationKind,
    FileSpan, HintStatus, HumanChangeset, NodeRef, SOURCE_FILE_NAME, TRUTH_FILE_NAME,
    changeset_dir, changesets_root, validate,
};
use omnidiff::test::helper::human_content::Judgement;
use omnidiff::test::helper::human_mapping::invariants;
use omnidiff::test::helper::human_mapping::{
    HumanMapping, HumanMappingEntry, HumanOperation, HumanTextEntry, HumanTextMapping,
    HumanTextOperation, HumanTextSpan, HumanTextVerdict, NamedTextMapping, path_refs,
};
use omnidiff::test::helper::{node_for_path, path_for_node, precompute_paths};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use tree_sitter::Node;

use crate::actions::{auto_match_pair, classify_match_operation};
use crate::events::{FrameState, SessionEnd, run_case_session};
use crate::keylog::key_name;
use crate::layers::LayerPaint;
use crate::navigate::reveal_node;
use crate::render::centered_rect;
use crate::state::{
    App, Focus, Modal, OpenTarget, PanelState, TextOverlay, TextPaintState,
    expand_leading_whitespace_for_full, expand_updated_identifiers_for_full,
    expand_whitespace_before_removals_for_full, omnidiff_text_entries, solution_entries_mut,
    starting_solution,
};
use crate::stubs::{LICENSE_HEADER, module_name, set_mod_declaration};
use crate::{HELP_TEXT, samples_root, starting_cursor_id};

const BEFORE: usize = 0;
const AFTER: usize = 1;
const SIDES: [&str; 2] = ["before", "after"];

/// The pair view's own keys, put in front of the code session's footer.
pub(crate) const PAIR_FOOTER: &str = "b/Backspace board  y pin  Y match pin with cursor  + add \
    partner  - partners  L pool  S seed  V violations  |  ";

const BOARD_FOOTER: &str = "Tab side  j/k move  f counterpart  Enter open pair  p pair  S split  \
    M merge  a added  d removed  u unpair  c/C confirm  x solved  e verdict  L pool  V violations  \
    s save  ? help  q quit";

// ─── What the session holds ─────────────────────────────────────────────────────────────────

/// One partner of a node: the other end of one match entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Partner {
    pub(crate) file: String,
    pub(crate) path: Vec<String>,
    pub(crate) operation: HumanOperation,
    /// 1-based, for a human; 0 when the path does not resolve.
    pub(crate) line: usize,
}

impl Partner {
    fn place(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
}

/// The node `y` pinned: it stays selected across the board and other pairs until `Y` uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pin {
    pub(crate) side: usize,
    pub(crate) file: String,
    pub(crate) path: Vec<String>,
    /// `function_item at src/a.rs:10`, for the headers.
    pub(crate) label: String,
}

/// A pair view to open: the two files, and optionally where to put the cursors, keys to run on
/// arrival and a place to show in the text view.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PairRequest {
    pub(crate) before: String,
    pub(crate) after: String,
    pub(crate) before_path: Option<Vec<String>>,
    pub(crate) after_path: Option<Vec<String>>,
    pub(crate) keys: Vec<KeyCode>,
    pub(crate) sites: Vec<(usize, HumanTextSpan)>,
}

/// What a key in the pair view asks the changeset session for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Request {
    Board,
    Pool,
    Violations,
    Pair(PairRequest),
}

/// The open pair: its files, and which parts of `App::mapping` are stand-ins (see the module doc).
pub(crate) struct OpenPair {
    pub(crate) before: String,
    pub(crate) after: String,
    /// Per side, the paths of the nodes whose entry in the projection is a stand-in.
    pseudo: [BTreeSet<Vec<String>>; 2],
    /// The cursors as of the last key, so moving onto a node with partners elsewhere says so.
    last_cursor: [usize; 2],
    /// The painting being edited as of the last key: what a new painting was branched from.
    last_solution: String,
}

/// One subtree of the leftover pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolNode {
    pub(crate) side: usize,
    pub(crate) file: String,
    pub(crate) path: Vec<String>,
    pub(crate) line: usize,
    pub(crate) kind: String,
    pub(crate) lines: usize,
    /// Nodes in the subtree: the size the pool sorts by.
    pub(crate) nodes: usize,
    /// `?` nothing said, `-` deleted, `+` inserted.
    pub(crate) state: char,
    pub(crate) snippet: String,
}

impl PoolNode {
    fn node_ref(&self) -> NodeRef {
        NodeRef {
            file: self.file.clone(),
            path: self.path.clone(),
        }
    }
}

/// One row of the pool: a hash-identical before/after candidate pair, or a subtree with no twin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolRow {
    pub(crate) before: Option<PoolNode>,
    pub(crate) after: Option<PoolNode>,
}

impl PoolRow {
    pub(crate) fn is_candidate(&self) -> bool {
        self.before.is_some() && self.after.is_some()
    }

    fn size(&self) -> usize {
        self.before
            .iter()
            .chain(&self.after)
            .map(|node| node.nodes)
            .max()
            .unwrap_or(0)
    }
}

/// What is drawn over the board.
pub(crate) enum Overlay {
    Pool {
        rows: Vec<PoolRow>,
        selected: usize,
        by_size: bool,
    },
    Violations {
        selected: usize,
    },
    /// The verdict prompt for `truth.files[relation]`.
    Verdict {
        relation: usize,
    },
    Help {
        scroll: u16,
    },
}

/// A file as the board shows it.
struct FileStats {
    language: String,
    lines: usize,
    /// Nodes no entry says anything about; `None` for a file with no tree.
    unmarked: Option<usize>,
}

pub(crate) struct Session {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
    /// Under `samples/changesets/`: saving a finished one promotes it.
    pub(crate) is_sample: bool,
    pub(crate) source: ChangesetSource,
    pub(crate) trees: ChangesetTrees,
    /// The truth in memory. `files` holds confirmed relations only.
    pub(crate) truth: HumanChangeset,
    /// Relations nobody confirmed: shown dim, never saved.
    pub(crate) proposals: Vec<FileRelation>,
    pub(crate) dirty: bool,
    /// Which list the board's keys act on.
    pub(crate) focus: usize,
    pub(crate) cursor: [usize; 2],
    pub(crate) pin: Option<Pin>,
    pub(crate) overlay: Option<Overlay>,
    pub(crate) status: String,
    pub(crate) pair: Option<OpenPair>,
    /// Keys the next pair view runs as it opens.
    queued: VecDeque<KeyCode>,
    /// What `validate` found at the last save or `V`.
    pub(crate) violations: Vec<ChangesetViolation>,
    quit_armed: bool,
    /// Where a promoted sample goes, and where stubs live: the repository's, except in tests.
    pub(crate) corpus_root: PathBuf,
    pub(crate) fixtures_root: PathBuf,
    /// Each file as the pair view needs it: its own parse, with the hashes `m` reads. Kept so a
    /// file's node ids are the same every time it is opened.
    codes: [HashMap<String, Arc<Code>>; 2],
    stats: Option<[BTreeMap<String, FileStats>; 2]>,
}

/// The changeset directory `name` opens, and whether it is a sample.
pub(crate) fn find_dir(name: &str) -> Option<(PathBuf, bool)> {
    if let Some(dir) = changeset_dir(name) {
        return Some((dir, false));
    }
    let sample = samples_root().join("changesets").join(name);
    sample.is_dir().then_some((sample, true))
}

fn is_match(operation: HumanOperation) -> bool {
    matches!(
        operation,
        HumanOperation::Identical | HumanOperation::Update | HumanOperation::MatchButNotIdentical
    )
}

fn starts_with(path: &[String], prefix: &[String]) -> bool {
    path.len() >= prefix.len() && path[..prefix.len()] == *prefix
}

fn relation_names(relation: &FileRelation, side: usize, path: &str) -> bool {
    [&relation.before, &relation.after][side].as_deref() == Some(path)
}

fn same_or_renamed(before: &str, after: &str) -> FileRelationKind {
    if before == after {
        FileRelationKind::Same
    } else {
        FileRelationKind::Renamed
    }
}

fn kind_name(kind: FileRelationKind) -> &'static str {
    match kind {
        FileRelationKind::Same => "same",
        FileRelationKind::Renamed => "renamed",
        FileRelationKind::Split => "split",
        FileRelationKind::Merged => "merged",
        FileRelationKind::Added => "added",
        FileRelationKind::Removed => "removed",
    }
}

impl Session {
    pub(crate) fn open(dir: &Path, is_sample: bool) -> Result<Self> {
        let changeset = Changeset::load(dir)?;
        let name = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("test")
            .join("fixtures");
        let mut session = Session {
            name,
            dir: dir.to_path_buf(),
            is_sample,
            source: changeset.source,
            trees: changeset.trees,
            truth: changeset.truth.unwrap_or_default(),
            proposals: Vec::new(),
            dirty: false,
            focus: BEFORE,
            cursor: [0, 0],
            pin: None,
            overlay: None,
            status: String::new(),
            pair: None,
            queued: VecDeque::new(),
            violations: Vec::new(),
            quit_armed: false,
            corpus_root: changesets_root(),
            fixtures_root,
            codes: Default::default(),
            stats: None,
        };
        session.propose();
        session.status = format!(
            "Opened '{}': {} before file(s), {} after file(s), {} relation(s) on file, {} proposed \
             (c confirms one, C all)",
            session.name,
            session.trees.before.len(),
            session.trees.after.len(),
            session.truth.files.len(),
            session.proposals.len()
        );
        Ok(session)
    }

    fn side(
        &self,
        side: usize,
    ) -> &BTreeMap<String, omnidiff::test::helper::human_changeset::ChangesetFile> {
        if side == BEFORE {
            &self.trees.before
        } else {
            &self.trees.after
        }
    }

    pub(crate) fn files(&self, side: usize) -> Vec<String> {
        self.side(side).keys().cloned().collect()
    }

    /// The file under the board's cursor on `side`.
    pub(crate) fn selected(&self, side: usize) -> Option<String> {
        self.side(side).keys().nth(self.cursor[side]).cloned()
    }

    fn has_relation(&self, side: usize, path: &str) -> bool {
        self.truth
            .files
            .iter()
            .any(|relation| relation_names(relation, side, path))
    }

    /// Fills `proposals` for the files the truth relates to nothing: what the VCS hints say, then
    /// equal paths. A file takes at most one proposal.
    pub(crate) fn propose(&mut self) {
        self.proposals.clear();
        let mut taken: [BTreeSet<String>; 2] = Default::default();
        let mut candidates: Vec<(Option<String>, Option<String>, FileRelationKind)> = Vec::new();
        for hint in &self.source.vcs_hints {
            let path = Some(hint.path.clone());
            match hint.status {
                HintStatus::M => candidates.push((path.clone(), path, FileRelationKind::Same)),
                HintStatus::R => {
                    candidates.push((hint.old_path.clone(), path, FileRelationKind::Renamed))
                }
                HintStatus::A => candidates.push((None, path, FileRelationKind::Added)),
                HintStatus::D => candidates.push((path, None, FileRelationKind::Removed)),
                // A copy's source keeps its own relation; which that is, the human says.
                HintStatus::C => {}
            }
        }
        for path in self.trees.before.keys() {
            if self.trees.after.contains_key(path) {
                candidates.push((
                    Some(path.clone()),
                    Some(path.clone()),
                    FileRelationKind::Same,
                ));
            }
        }
        for (before, after, relation) in candidates {
            let free = |side: usize, path: &Option<String>, taken: &[BTreeSet<String>; 2]| {
                path.as_ref().is_none_or(|path| {
                    self.side(side).contains_key(path)
                        && !self.has_relation(side, path)
                        && !taken[side].contains(path)
                })
            };
            if (before.is_none() && after.is_none())
                || !free(BEFORE, &before, &taken)
                || !free(AFTER, &after, &taken)
            {
                continue;
            }
            taken[BEFORE].extend(before.clone());
            taken[AFTER].extend(after.clone());
            self.proposals.push(FileRelation {
                before,
                after,
                relation,
                solved: false,
                verdict: None,
            });
        }
    }

    /// Records that the truth changed.
    fn touch(&mut self) {
        self.dirty = true;
        self.stats = None;
        self.quit_armed = false;
    }

    /// `before file -> after file` and the pin, for the pair view's title line.
    pub(crate) fn pair_header(&self) -> Option<String> {
        let pair = self.pair.as_ref()?;
        let relation = self
            .truth
            .files
            .iter()
            .find(|relation| {
                relation_names(relation, BEFORE, &pair.before)
                    && relation_names(relation, AFTER, &pair.after)
            })
            .map_or("no relation", |relation| kind_name(relation.relation));
        let pin = self
            .pin
            .as_ref()
            .map(|pin| format!("  [pinned: {}]", pin.label))
            .unwrap_or_default();
        Some(format!(
            "before {} -> after {} ({relation}){pin}",
            pair.before, pair.after
        ))
    }

    /// `file` of `side` as the pair view opens it.
    fn code(&mut self, side: usize, file: &str) -> Result<Arc<Code>> {
        if let Some(code) = self.codes[side].get(file) {
            return Ok(Arc::clone(code));
        }
        let found = self
            .side(side)
            .get(file)
            .with_context(|| format!("no {} file {file}", SIDES[side]))?;
        let mut code = found
            .code
            .clone()
            .with_context(|| format!("{file} is not text: give the pair a verdict (e)"))?;
        // A clone has new node ids and no hashes (see `Code::clone`).
        if code.ast.is_some() {
            code.ensure_parsed()?;
        }
        let code = Arc::new(code);
        self.codes[side].insert(file.to_string(), Arc::clone(&code));
        Ok(code)
    }

    fn line_of(&self, side: usize, file: &str, path: &[String]) -> usize {
        self.side(side)
            .get(file)
            .and_then(|file| file.root())
            .and_then(|root| node_for_path(root, &path_refs(path)).ok())
            .map_or(0, |node| node.start_position().row + 1)
    }

    /// Every partner of the node `path` in `file` on `side`.
    pub(crate) fn partners_of(&self, side: usize, file: &str, path: &[String]) -> Vec<Partner> {
        self.truth
            .entries
            .iter()
            .filter(|entry| is_match(entry.operation))
            .filter_map(|entry| {
                let (Some(before), Some(after)) = (&entry.before, &entry.after) else {
                    return None;
                };
                let (own, other) = if side == BEFORE {
                    (before, after)
                } else {
                    (after, before)
                };
                (own.file == file && own.path == path).then(|| Partner {
                    file: other.file.clone(),
                    path: other.path.clone(),
                    operation: entry.operation,
                    line: self.line_of(1 - side, &other.file, &other.path),
                })
            })
            .collect()
    }

    // ─── Saving ─────────────────────────────────────────────────────────────────────────────

    fn stub_path(&self) -> PathBuf {
        self.fixtures_root
            .join("changesets")
            .join(format!("{}.rs", module_name(&self.name)))
    }

    /// The stub exists exactly when the changeset is solved, as
    /// `solved_changesets_have_a_stub_and_only_they_do` demands.
    fn set_stub(&self, solved: bool) -> Result<bool> {
        let path = self.stub_path();
        let mod_file = self.fixtures_root.join("changesets.rs");
        if !mod_file.exists() {
            fs::create_dir_all(&self.fixtures_root)?;
            fs::write(&mod_file, "")?;
        }
        let existed = path.exists();
        if solved && !existed {
            fs::create_dir_all(path.parent().context("the stub has no directory")?)?;
            fs::write(&path, stub_contents(&self.name))
                .with_context(|| format!("writing {path:?}"))?;
        } else if !solved && existed {
            fs::remove_file(&path).with_context(|| format!("removing {path:?}"))?;
        }
        set_mod_declaration(&mod_file, &module_name(&self.name), solved)?;
        Ok(existed != solved)
    }

    /// Moves a finished sample into the corpus, keeping its `source.json` and `README.md`.
    fn promote(&mut self) -> Result<String> {
        let dataset = self.source.dataset.clone();
        ensure!(
            CHANGESET_DATASETS.contains(&dataset.as_str()),
            "source.json names the dataset '{dataset}', which is not one of {CHANGESET_DATASETS:?}"
        );
        ensure!(
            self.dir.join("README.md").is_file(),
            "the sample has no README.md saying where it is from and under what licence, which \
             the corpus requires: add one, then save again"
        );
        for known in CHANGESET_DATASETS {
            ensure!(
                !self.corpus_root.join(known).join(&self.name).exists(),
                "a changeset called '{}' is already in {known}",
                self.name
            );
        }
        let target = self.corpus_root.join(&dataset).join(&self.name);
        fs::create_dir_all(target.parent().context("no dataset directory")?)?;
        fs::rename(&self.dir, &target)
            .with_context(|| format!("moving {:?} to {target:?}", self.dir))?;
        self.dir = target;
        self.is_sample = false;
        self.source.status = "PROMOTED".to_string();
        fs::write(self.dir.join(SOURCE_FILE_NAME), self.source.to_json()?)?;
        self.set_stub(true)?;
        Ok(format!(
            "promoted to changesets/{dataset}/{} with its stub",
            self.name
        ))
    }

    /// `s`: writes `human_changeset.json`, validates it, and keeps the stub (or promotes the
    /// sample) in step with whether the changeset is solved. Partial progress saves as it is.
    pub(crate) fn save(&mut self) -> Result<String> {
        self.truth.save(&self.dir, &self.trees)?;
        self.dirty = false;
        self.violations = validate(&self.truth, &self.trees);
        let solved = self.truth.is_solved(&self.trees);
        let mut status = format!("Saved {TRUTH_FILE_NAME}");
        if !self.proposals.is_empty() {
            status.push_str(&format!(
                " ({} proposed relation(s) are not confirmed and were not saved)",
                self.proposals.len()
            ));
        }
        if self.violations.is_empty() {
            status.push_str("; it validates");
        } else {
            status.push_str(&format!(
                "; {} violation(s) - V lists them",
                self.violations.len()
            ));
        }
        if self.is_sample {
            if solved && self.violations.is_empty() {
                status.push_str(&format!("; {}", self.promote()?));
            } else {
                status.push_str("; still a sample (promoted once solved and valid)");
            }
        } else {
            let changed = self.set_stub(solved)?;
            status.push_str(match (solved, changed) {
                (true, true) => "; solved - stub written",
                (true, false) => "; solved",
                (false, true) => "; not solved - stub removed",
                (false, false) => "; not solved yet",
            });
        }
        Ok(status)
    }
}

/// A solved changeset's stub, as `src/test/fixtures/changesets/` holds them.
pub(crate) fn stub_contents(name: &str) -> String {
    format!(
        "{LICENSE_HEADER}\nuse crate::test::helper::human_changeset::assert_solved_ground_truth;\n\
         use anyhow::Result;\n\n#[test]\nfn ground_truth() -> Result<()> {{\n    \
         assert_solved_ground_truth(\"{name}\")\n}}\n"
    )
}

// ─── The projection onto one pair ───────────────────────────────────────────────────────────

/// A changeset's truth as one pair of files reads it. See the module doc.
pub(crate) struct Projection {
    pub(crate) mapping: HumanMapping,
    pseudo: [BTreeSet<Vec<String>>; 2],
    /// Per side, the partners the mapping does not hold, by the path of the node that has them.
    pub(crate) partners: [BTreeMap<Vec<String>, Vec<Partner>>; 2],
    /// The painted ranges of the two files whose other end is in a third file: not the pair's to
    /// edit, so they are drawn over its text view (`App::layer_paint`) and are not in `mapping`.
    foreign_paint: LayerPaint,
}

/// The indices of the entries between `before` and `after` the projection shows as matches: the
/// first on file for each node. The rest are further partners.
fn primaries(truth: &HumanChangeset, before: &str, after: &str) -> BTreeSet<usize> {
    let mut paired: [HashSet<&[String]>; 2] = Default::default();
    let mut primary = BTreeSet::new();
    for (index, entry) in truth.entries.iter().enumerate() {
        let (Some(b), Some(a)) = (&entry.before, &entry.after) else {
            continue;
        };
        if !is_match(entry.operation) || b.file != before || a.file != after {
            continue;
        }
        if paired[BEFORE].contains(b.path.as_slice()) || paired[AFTER].contains(a.path.as_slice()) {
            continue;
        }
        paired[BEFORE].insert(&b.path);
        paired[AFTER].insert(&a.path);
        primary.insert(index);
    }
    primary
}

/// Whether a one-sided entry belongs to the pair: a delete in its before file, an insert in its
/// after file.
fn one_sided_in_pair(entry: &ChangesetEntry, before: &str, after: &str) -> bool {
    match (&entry.before, &entry.after) {
        (Some(node), None) => node.file == before,
        (None, Some(node)) => node.file == after,
        _ => false,
    }
}

/// How a painted entry lies across the pair: its ranges inside it, per side, and whether it has
/// ranges outside.
fn split_painted(
    entry: &ChangesetTextEntry,
    before: &str,
    after: &str,
) -> ([Vec<HumanTextSpan>; 2], bool) {
    let inside = |spans: &[FileSpan], file: &str| -> Vec<HumanTextSpan> {
        spans
            .iter()
            .filter(|span| span.file == file)
            .map(|span| span.range)
            .collect()
    };
    let within = [inside(&entry.before, before), inside(&entry.after, after)];
    let outside =
        within[BEFORE].len() != entry.before.len() || within[AFTER].len() != entry.after.len();
    (within, outside)
}

pub(crate) fn project(session: &Session, before: &str, after: &str) -> Projection {
    let truth = &session.truth;
    let primary = primaries(truth, before, after);
    let mut entries = Vec::new();
    let mut paired: [HashSet<&[String]>; 2] = Default::default();
    let mut partners: [BTreeMap<Vec<String>, Vec<Partner>>; 2] = Default::default();
    for (index, entry) in truth.entries.iter().enumerate() {
        if one_sided_in_pair(entry, before, after) && !is_match(entry.operation) {
            entries.push(HumanMappingEntry {
                operation: entry.operation,
                before_path: entry.before.as_ref().map(|node| node.path.clone()),
                after_path: entry.after.as_ref().map(|node| node.path.clone()),
            });
            continue;
        }
        let (Some(b), Some(a)) = (&entry.before, &entry.after) else {
            continue;
        };
        if !is_match(entry.operation) {
            continue;
        }
        if primary.contains(&index) {
            paired[BEFORE].insert(&b.path);
            paired[AFTER].insert(&a.path);
            entries.push(HumanMappingEntry {
                operation: entry.operation,
                before_path: Some(b.path.clone()),
                after_path: Some(a.path.clone()),
            });
            continue;
        }
        if b.file == before {
            partners[BEFORE]
                .entry(b.path.clone())
                .or_default()
                .push(Partner {
                    file: a.file.clone(),
                    path: a.path.clone(),
                    operation: entry.operation,
                    line: session.line_of(AFTER, &a.file, &a.path),
                });
        }
        if a.file == after {
            partners[AFTER]
                .entry(a.path.clone())
                .or_default()
                .push(Partner {
                    file: b.file.clone(),
                    path: b.path.clone(),
                    operation: entry.operation,
                    line: session.line_of(BEFORE, &b.file, &b.path),
                });
        }
    }
    let mut pseudo: [BTreeSet<Vec<String>>; 2] = Default::default();
    for side in [BEFORE, AFTER] {
        for path in partners[side].keys() {
            if paired[side].contains(path.as_slice()) {
                continue;
            }
            pseudo[side].insert(path.clone());
            entries.push(pseudo_entry(side, path));
        }
    }

    let mut foreign_paint = LayerPaint::default();
    let text_mappings = truth
        .text_mappings
        .iter()
        .map(|painting| {
            let mut own = Vec::new();
            let mut foreign: [Vec<(HumanTextSpan, HumanTextVerdict)>; 2] = Default::default();
            for entry in &painting.entries {
                let ([b, a], outside) = split_painted(entry, before, after);
                if !outside {
                    if !(b.is_empty() && a.is_empty()) {
                        own.push(HumanTextEntry {
                            operation: entry.operation,
                            before: b,
                            after: a,
                        });
                    }
                    continue;
                }
                let Ok(verdict) = entry.verdict(&session.trees) else {
                    continue;
                };
                for (side, spans) in [b, a].into_iter().enumerate() {
                    foreign[side].extend(spans.into_iter().map(|span| (span, verdict)));
                }
            }
            foreign_paint
                .paintings
                .push((painting.name.clone(), foreign));
            NamedTextMapping {
                name: painting.name.clone(),
                mapping: HumanTextMapping { entries: own },
            }
        })
        .collect();

    Projection {
        mapping: HumanMapping {
            entries,
            text_mappings,
            ..Default::default()
        },
        pseudo,
        partners,
        foreign_paint,
    }
}

/// The stand-in for a node of `side` whose partners are all outside the projection.
fn pseudo_entry(side: usize, path: &[String]) -> HumanMappingEntry {
    if side == BEFORE {
        HumanMappingEntry {
            operation: HumanOperation::Delete,
            before_path: Some(path.to_vec()),
            after_path: None,
        }
    } else {
        HumanMappingEntry {
            operation: HumanOperation::Insert,
            before_path: None,
            after_path: Some(path.to_vec()),
        }
    }
}

fn entry_path(entry: &HumanMappingEntry, side: usize) -> Option<&Vec<String>> {
    if side == BEFORE {
        entry.before_path.as_ref()
    } else {
        entry.after_path.as_ref()
    }
}

fn is_pseudo_entry(entry: &HumanMappingEntry, side: usize) -> bool {
    let wanted = if side == BEFORE {
        HumanOperation::Delete
    } else {
        HumanOperation::Insert
    };
    entry.operation == wanted && entry_path(entry, 1 - side).is_none()
}

/// Puts back every stand-in a key replaced or removed, so a node matched elsewhere never gains a
/// partner (or loses its standing) through a single-file key. Returns how many were restored. A
/// with-children mark over such a node is reduced to a plain one: the node is not removed.
pub(crate) fn guard(mapping: &mut HumanMapping, pseudo: &[BTreeSet<Vec<String>>; 2]) -> usize {
    let mut restored = 0;
    for side in [BEFORE, AFTER] {
        for path in &pseudo[side] {
            let touching: Vec<&HumanMappingEntry> = mapping
                .entries
                .iter()
                .filter(|entry| entry_path(entry, side) == Some(path))
                .collect();
            let intact = touching.len() == 1 && is_pseudo_entry(touching[0], side);
            if !intact {
                mapping
                    .entries
                    .retain(|entry| entry_path(entry, side) != Some(path));
                mapping.entries.push(pseudo_entry(side, path));
                restored += 1;
            }
            let with_children = if side == BEFORE {
                (HumanOperation::DeleteWithChildren, HumanOperation::Delete)
            } else {
                (HumanOperation::InsertWithChildren, HumanOperation::Insert)
            };
            for entry in &mut mapping.entries {
                let covers = entry.operation == with_children.0
                    && entry_path(entry, side)
                        .is_some_and(|own| own.len() < path.len() && starts_with(path, own));
                if covers {
                    entry.operation = with_children.1;
                    restored += 1;
                }
            }
        }
    }
    restored
}

/// Writes the pair's `mapping` back into the changeset: its entries replace the ones the
/// projection showed, stand-ins left out, and its painted ranges replace the ones that lie wholly
/// in the pair.
pub(crate) fn write_back(truth: &mut HumanChangeset, pair: &OpenPair, mapping: &HumanMapping) {
    let primary = primaries(truth, &pair.before, &pair.after);
    let mut kept = Vec::with_capacity(truth.entries.len());
    for (index, entry) in std::mem::take(&mut truth.entries).into_iter().enumerate() {
        let shown = primary.contains(&index)
            || (!is_match(entry.operation) && one_sided_in_pair(&entry, &pair.before, &pair.after));
        if !shown {
            kept.push(entry);
        }
    }
    // In front, so the same entries are the projection's matches next time.
    let mut entries: Vec<ChangesetEntry> = mapping
        .entries
        .iter()
        .filter(|entry| {
            ![BEFORE, AFTER].into_iter().any(|side| {
                is_pseudo_entry(entry, side)
                    && entry_path(entry, side).is_some_and(|path| pair.pseudo[side].contains(path))
            })
        })
        .map(|entry| ChangesetEntry {
            operation: entry.operation,
            before: entry.before_path.as_ref().map(|path| NodeRef {
                file: pair.before.clone(),
                path: path.clone(),
            }),
            after: entry.after_path.as_ref().map(|path| NodeRef {
                file: pair.after.clone(),
                path: path.clone(),
            }),
        })
        .collect();
    entries.extend(kept);
    truth.entries = entries;

    for named in &mapping.text_mappings {
        if !truth
            .text_mappings
            .iter()
            .any(|painting| painting.name == named.name)
        {
            truth.text_mappings.push(ChangesetTextMapping {
                name: named.name.clone(),
                entries: Vec::new(),
            });
        }
    }
    for painting in &mut truth.text_mappings {
        let now: Vec<HumanTextEntry> = mapping
            .text_mappings
            .iter()
            .find(|named| named.name == painting.name)
            .map(|named| named.mapping.entries.clone())
            .unwrap_or_default();
        // What the projection showed goes; what is painted now comes back.
        painting.entries.retain(|entry| {
            let ([b, a], outside) = split_painted(entry, &pair.before, &pair.after);
            outside || (b.is_empty() && a.is_empty())
        });
        let spans = |ranges: &[HumanTextSpan], file: &str| -> Vec<FileSpan> {
            ranges
                .iter()
                .map(|range| FileSpan {
                    file: file.to_string(),
                    range: *range,
                })
                .collect()
        };
        painting
            .entries
            .extend(now.into_iter().map(|entry| ChangesetTextEntry {
                operation: entry.operation,
                before: spans(&entry.before, &pair.before),
                after: spans(&entry.after, &pair.after),
            }));
    }
    // A painting the pair view deleted, with nothing left in it anywhere, is gone.
    truth.text_mappings.retain(|painting| {
        !painting.entries.is_empty()
            || mapping
                .text_mappings
                .iter()
                .any(|named| named.name == painting.name)
    });
}

/// Copies the ranges of painting `from` that are not the pair's own (those the pair view copied
/// itself) into the new painting `to`, file pair by file pair, with the widening a branch from
/// Minimal to Full does in the code session. Returns how many ranges it copied.
fn branch_elsewhere(
    session: &mut Session,
    from: &str,
    to: &str,
    before: &str,
    after: &str,
) -> usize {
    let Some(source) = session
        .truth
        .text_mappings
        .iter()
        .find(|painting| painting.name == from)
    else {
        return 0;
    };
    let one_file = |spans: &[FileSpan]| -> Option<Option<String>> {
        let first = spans.first().map(|span| span.file.clone());
        spans
            .iter()
            .all(|span| Some(&span.file) == first.as_ref())
            .then_some(first)
    };
    let mut groups: BTreeMap<(Option<String>, Option<String>), Vec<HumanTextEntry>> =
        BTreeMap::new();
    let mut copied: Vec<ChangesetTextEntry> = Vec::new();
    for entry in &source.entries {
        let (_, outside) = split_painted(entry, before, after);
        if !outside {
            continue;
        }
        match (one_file(&entry.before), one_file(&entry.after)) {
            (Some(before_file), Some(after_file)) => groups
                .entry((before_file, after_file))
                .or_default()
                .push(HumanTextEntry {
                    operation: entry.operation,
                    before: entry.before.iter().map(|span| span.range).collect(),
                    after: entry.after.iter().map(|span| span.range).collect(),
                }),
            // Ranges of several files on one side: copied as they are.
            _ => copied.push(entry.clone()),
        }
    }
    let widen = invariants::designates_minimal(from) && invariants::designates_full(to);
    for ((before_file, after_file), mut entries) in groups {
        let text = |side: usize, file: &Option<String>| {
            file.as_ref()
                .and_then(|file| session.side(side).get(file))
                .and_then(|file| file.text())
                .unwrap_or("")
        };
        let (before_text, after_text) = (text(BEFORE, &before_file), text(AFTER, &after_file));
        if widen {
            expand_leading_whitespace_for_full(&mut entries, before_text, after_text);
            expand_whitespace_before_removals_for_full(&mut entries, before_text, after_text);
            expand_updated_identifiers_for_full(&mut entries, before_text, after_text);
        }
        let spans = |ranges: &[HumanTextSpan], file: &Option<String>| -> Vec<FileSpan> {
            ranges
                .iter()
                .map(|range| FileSpan {
                    file: file.clone().unwrap_or_default(),
                    range: *range,
                })
                .collect()
        };
        copied.extend(entries.iter().map(|entry| ChangesetTextEntry {
            operation: entry.operation,
            before: spans(&entry.before, &before_file),
            after: spans(&entry.after, &after_file),
        }));
    }
    let count = copied.len();
    if let Some(target) = session
        .truth
        .text_mappings
        .iter_mut()
        .find(|painting| painting.name == to)
    {
        target.entries.extend(copied);
    }
    count
}

/// In painting `name`, widens to column 0 every line that is wholly gone from its file or wholly
/// new in it, where "gone" and "new" include the ends of a match between files no `same` or
/// `renamed` relation pairs - which is how such a line reads from inside its own file, and so how
/// the invariants (4) read it. The code session's branch does this for deletes and inserts only,
/// since within one file a match is a surviving line. A match whose two ends would stop reading
/// the same is left as it was.
fn widen_departed_lines(session: &mut Session, name: &str) {
    let Some(index) = session
        .truth
        .text_mappings
        .iter()
        .position(|painting| painting.name == name)
    else {
        return;
    };
    let in_place = |before: &str, after: &str| {
        session.truth.files.iter().any(|relation| {
            relation_names(relation, BEFORE, before)
                && relation_names(relation, AFTER, after)
                && matches!(
                    relation.relation,
                    FileRelationKind::Same | FileRelationKind::Renamed
                )
        })
    };
    let original = session.truth.text_mappings[index].entries.clone();
    let mut widened = original.clone();
    for side in [BEFORE, AFTER] {
        let wanted = if side == BEFORE {
            HumanTextOperation::Delete
        } else {
            HumanTextOperation::Insert
        };
        for (file, found) in session.side(side) {
            let Some(text) = found.text() else { continue };
            // This file's ranges as its own pair view reads them, each with the entry it is of.
            let mut owners = Vec::new();
            let mut local = Vec::new();
            for (owner, entry) in widened.iter().enumerate() {
                let spans = [&entry.before, &entry.after][side];
                if spans.is_empty() || spans.iter().any(|span| span.file != *file) {
                    continue;
                }
                let other = [&entry.before, &entry.after][1 - side];
                let stays = entry.operation == HumanTextOperation::Match
                    && other.iter().all(|span| {
                        if side == BEFORE {
                            in_place(file, &span.file)
                        } else {
                            in_place(&span.file, file)
                        }
                    });
                let ranges: Vec<HumanTextSpan> = spans.iter().map(|span| span.range).collect();
                let (before, after) = if side == BEFORE {
                    (ranges, Vec::new())
                } else {
                    (Vec::new(), ranges)
                };
                local.push(HumanTextEntry {
                    operation: if stays {
                        HumanTextOperation::Match
                    } else {
                        wanted
                    },
                    before,
                    after,
                });
                owners.push(owner);
            }
            let (before_text, after_text) = if side == BEFORE {
                (text, "")
            } else {
                ("", text)
            };
            expand_leading_whitespace_for_full(&mut local, before_text, after_text);
            for (entry, owner) in local.iter().zip(owners) {
                let ranges = [&entry.before, &entry.after][side];
                let spans = if side == BEFORE {
                    &mut widened[owner].before
                } else {
                    &mut widened[owner].after
                };
                for (span, range) in spans.iter_mut().zip(ranges) {
                    span.range = *range;
                }
            }
        }
    }
    for (entry, was) in widened.iter_mut().zip(&original) {
        let same_verdict = match (entry.verdict(&session.trees), was.verdict(&session.trees)) {
            (Ok(now), Ok(then)) => now == then,
            _ => false,
        };
        if !same_verdict {
            *entry = was.clone();
        }
    }
    session.truth.text_mappings[index].entries = widened;
}

/// Projects the changeset onto the open pair again: `App::mapping`, the stand-ins, and each
/// panel's `elsewhere` table.
fn reproject(app: &mut App, before: &Code, after: &Code) {
    let Some(session) = app.changeset.as_mut() else {
        return;
    };
    let Some((before_file, after_file)) = session
        .pair
        .as_ref()
        .map(|pair| (pair.before.clone(), pair.after.clone()))
    else {
        return;
    };
    let projection = project(session, &before_file, &after_file);
    for (side, (panel, code)) in [(&mut app.before, before), (&mut app.after, after)]
        .into_iter()
        .enumerate()
    {
        panel.elsewhere.clear();
        let Some(root) = code.ast.as_ref().map(|tree| tree.root_node()) else {
            continue;
        };
        for (path, partners) in &projection.partners[side] {
            if let Ok(node) = node_for_path(root, &path_refs(path)) {
                panel
                    .elsewhere
                    .insert(node.id(), partners.iter().map(Partner::place).collect());
            }
        }
    }
    let pair = session.pair.as_mut().expect("checked above");
    pair.pseudo = projection.pseudo;
    app.mapping = projection.mapping;
    // Every painting has an overlay, even an empty one: `LayerPaint::for_solution` would
    // otherwise draw another painting's.
    let mut foreign = projection.foreign_paint;
    for name in crate::state::SUGGESTED_SOLUTION_NAMES {
        if !foreign.paintings.iter().any(|(known, _)| known == name) {
            foreign
                .paintings
                .push((name.to_string(), Default::default()));
        }
    }
    app.layer_paint = vec![foreign];
}

/// After every key of the pair view. An edit goes back into the changeset and the pair is
/// projected again; a cursor that moved onto a node with partners elsewhere says where they are.
/// Returns whether `App::mapping` was replaced.
pub(crate) fn after_key(app: &mut App, edits_before: u64, before: &Code, after: &Code) -> bool {
    let cursors = [app.before.cursor_id, app.after.cursor_id];
    let Some(session) = app.changeset.as_mut() else {
        return false;
    };
    let Some(pair) = session.pair.as_mut() else {
        return false;
    };
    let branched_from = std::mem::replace(&mut pair.last_solution, app.text_solution.clone());
    if app.edits == edits_before {
        let side = usize::from(app.focus == Focus::After);
        let moved = pair.last_cursor[side] != cursors[side];
        pair.last_cursor = cursors;
        let panel = if side == BEFORE {
            &app.before
        } else {
            &app.after
        };
        if let (true, None, Some(places)) = (moved, &app.modal, panel.elsewhere.get(&cursors[side]))
        {
            app.status = Some(format!(
                "This node is matched to {} (- lists its partners, + adds one)",
                places.join(", ")
            ));
        }
        return false;
    }
    let restored = guard(&mut app.mapping, &pair.pseudo);
    // A painting is the whole changeset's: one branched here (`s` in the text view, a copy) is
    // branched in every other file too, or it would say those files did not change.
    let copied = app
        .status
        .as_deref()
        .is_some_and(|status| status.contains("as a copy"));
    let new_paintings: Vec<String> = app
        .mapping
        .text_mappings
        .iter()
        .map(|named| named.name.clone())
        .filter(|name| {
            !session
                .truth
                .text_mappings
                .iter()
                .any(|known| known.name == *name)
        })
        .collect();
    write_back(&mut session.truth, pair, &app.mapping);
    let (before_file, after_file) = (pair.before.clone(), pair.after.clone());
    let mut elsewhere = 0;
    if copied {
        for name in &new_paintings {
            elsewhere += branch_elsewhere(session, &branched_from, name, &before_file, &after_file);
            if invariants::designates_minimal(&branched_from) && invariants::designates_full(name) {
                widen_departed_lines(session, name);
            }
        }
    }
    session.touch();
    app.dirty = true;
    reproject(app, before, after);
    if elsewhere > 0 {
        let status = app.status.take().unwrap_or_default();
        app.status = Some(format!(
            "{status}; {elsewhere} range(s) in the changeset's other files branched with it"
        ));
    }
    if restored > 0 {
        let status = app.status.take().unwrap_or_default();
        app.status = Some(format!(
            "{status} [{restored} mark(s) on nodes matched elsewhere were left as they were: + \
             adds a partner, - removes one]"
        ));
    }
    true
}

/// The next key a freshly opened pair view runs before it reads the keyboard.
pub(crate) fn take_queued_key(app: &mut App) -> Option<KeyCode> {
    let session = app.changeset.as_mut()?;
    session.pair.as_ref()?;
    session.queued.pop_front()
}

// ─── Opening and leaving a pair ─────────────────────────────────────────────────────────────

/// Turns the app into a code session over the pair `request` names. Returns its two files.
pub(crate) fn enter_pair(app: &mut App, request: &PairRequest) -> Result<(Arc<Code>, Arc<Code>)> {
    let session = app.changeset.as_mut().context("no changeset is open")?;
    let before = session.code(BEFORE, &request.before)?;
    let after = session.code(AFTER, &request.after)?;
    session.pair = Some(OpenPair {
        before: request.before.clone(),
        after: request.after.clone(),
        pseudo: Default::default(),
        last_cursor: [usize::MAX; 2],
        last_solution: String::new(),
    });
    session.queued = request.keys.iter().copied().collect();
    session.overlay = None;
    let dirty = session.dirty;
    let seedable = session.truth.files.iter().any(|relation| {
        relation_names(relation, BEFORE, &request.before)
            && relation_names(relation, AFTER, &request.after)
            && matches!(
                relation.relation,
                FileRelationKind::Same | FileRelationKind::Renamed
            )
    });

    app.before = PanelState::new(starting_cursor_id(&before));
    app.after = PanelState::new(starting_cursor_id(&after));
    app.focus = Focus::Before;
    app.modal = None;
    app.undo_stack.clear();
    app.redo_stack.clear();
    app.algo_diff = None;
    app.algo_diff_pending = None;
    app.algo_text_spans = None;
    app.text_overlay = TextOverlay::default();
    app.layers.clear();
    app.clear_multi_select();
    app.should_quit = false;
    reproject(app, &before, &after);
    app.dirty = dirty;
    app.text_solution = starting_solution(&app.mapping);
    if let Some(pair) = app
        .changeset
        .as_mut()
        .and_then(|session| session.pair.as_mut())
    {
        pair.last_solution = app.text_solution.clone();
    }

    for (panel, code, path) in [
        (&mut app.before, &before, &request.before_path),
        (&mut app.after, &after, &request.after_path),
    ] {
        let (Some(path), Some(tree)) = (path, code.ast.as_ref()) else {
            continue;
        };
        if let Ok(node) = node_for_path(tree.root_node(), &path_refs(path)) {
            reveal_node(panel, tree.root_node(), node.id());
        }
    }
    if request.after_path.is_some() && request.before_path.is_none() {
        app.focus = Focus::After;
    }
    let cursors = [app.before.cursor_id, app.after.cursor_id];
    if let Some(pair) = app
        .changeset
        .as_mut()
        .and_then(|session| session.pair.as_mut())
    {
        pair.last_cursor = cursors;
    }
    if let Some((first_side, _)) = request.sites.first() {
        let mut state = TextPaintState {
            side: *first_side,
            ..Default::default()
        };
        for (side, span) in request.sites.iter().rev() {
            state.cursor[*side] = (span.start_row, span.start_column);
        }
        state.scroll_into_view(20);
        app.modal = Some(Modal::TextView { state });
    }

    let session = app.changeset.as_ref().expect("checked above");
    let pair = session.pair.as_ref().expect("just set");
    let recorded = app.mapping.entries.len() - pair.pseudo[BEFORE].len() - pair.pseudo[AFTER].len();
    app.status = Some(if recorded == 0 && seedable && before.ast.is_some() {
        format!(
            "Opened before {} -> after {}: nothing recorded for this pair - S seeds its mapping \
             and painting from omnidiff's single-file diff; b goes back to the board",
            request.before, request.after
        )
    } else {
        format!(
            "Opened before {} -> after {}: {recorded} entr(ies) of this pair; b goes back to the \
             board",
            request.before, request.after
        )
    });
    Ok((before, after))
}

/// Closes the pair view. Its edits are already in the changeset ([`after_key`]).
pub(crate) fn leave_pair(app: &mut App) {
    if let Some(session) = app.changeset.as_mut() {
        session.pair = None;
        session.queued.clear();
    }
    app.modal = None;
    app.mapping = HumanMapping::default();
    app.before.elsewhere.clear();
    app.after.elsewhere.clear();
    app.layer_paint.clear();
    app.should_quit = false;
}

/// Where the session goes when a pair view ends.
pub(crate) enum Next {
    Board,
    Pair(PairRequest),
}

/// Reads how the pair view ended (it is closed by now) into what to show next.
pub(crate) fn route(app: &mut App, end: SessionEnd) -> Next {
    let Some(session) = app.changeset.as_mut() else {
        return Next::Board;
    };
    match end {
        SessionEnd::Open(OpenTarget::Changeset(Request::Pair(request))) => {
            return Next::Pair(request);
        }
        SessionEnd::Open(OpenTarget::Changeset(Request::Pool)) => session.open_pool(),
        SessionEnd::Open(OpenTarget::Changeset(Request::Violations)) => session.open_violations(),
        SessionEnd::Open(OpenTarget::Changeset(Request::Board)) | SessionEnd::Quit => {
            session.status = "Back on the board".to_string();
        }
        SessionEnd::Open(other) => {
            session.status = format!(
                "'{}' cannot be opened from a changeset session: quit and start human_solver on it",
                other.name()
            );
        }
    }
    Next::Board
}

// ─── The pair view's own keys ───────────────────────────────────────────────────────────────

fn back(request: Request) -> Option<Option<OpenTarget>> {
    Some(Some(OpenTarget::Changeset(request)))
}

fn say(app: &mut App, status: impl Into<String>) -> Option<Option<OpenTarget>> {
    app.status = Some(status.into());
    Some(None)
}

/// The keys the pair view adds to, or takes from, the code session. `None` hands the key on to
/// the code session; `Some` is the key's whole effect, as `handle_key` would return it.
pub(crate) fn pair_key(
    app: &mut App,
    code: KeyCode,
    frame: &FrameState,
    before: &Code,
    after: &Code,
) -> Option<Option<OpenTarget>> {
    let session = app.changeset.as_ref()?;
    let pair = session.pair.as_ref()?;
    if app.modal.is_some() {
        return None;
    }
    let (before_file, after_file) = (pair.before.clone(), pair.after.clone());
    let cursor = |side: usize| -> Option<Node> {
        if side == BEFORE {
            frame.before_flat.node_for_id(app.before.cursor_id)
        } else {
            frame.after_flat.node_for_id(app.after.cursor_id)
        }
    };
    let focused = usize::from(app.focus == Focus::After);
    let stands_in = |side: usize| -> bool {
        cursor(side).is_some_and(|node| pair.pseudo[side].contains(&path_for_node(node)))
    };
    let ranging = app.before.anchor.is_some() || app.after.anchor.is_some();

    match code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') | KeyCode::Backspace => {
            back(Request::Board)
        }
        KeyCode::Char('L') => back(Request::Pool),
        KeyCode::Char('V') => back(Request::Violations),
        KeyCode::Char('s') => {
            let session = app.changeset.as_mut()?;
            let status = match session.save() {
                Ok(status) => status,
                Err(error) => format!("Error: {error:#}"),
            };
            app.dirty = session.dirty;
            say(app, status)
        }
        KeyCode::Char('x') | KeyCode::Char('X') => say(
            app,
            "A changeset has no multi-map groups: several partners are several entries (+ adds one)",
        ),
        KeyCode::Enter => say(
            app,
            "Code in a string has no changeset form yet: it is not opened here",
        ),
        KeyCode::Char('o' | 'O' | 'C' | '{' | '}' | 'R' | 'e') => say(
            app,
            "Not in a changeset session: b goes back to the board, q there quits",
        ),
        KeyCode::Char('m' | 'M') if !ranging && (stands_in(BEFORE) || stands_in(AFTER)) => say(
            app,
            "A node here already has a partner in another file: + adds this one as a further \
             partner, - lists and removes partners",
        ),
        KeyCode::Char('d' | 'D') if !ranging && focused == BEFORE && stands_in(BEFORE) => say(
            app,
            "This node is matched in another file, so it is not deleted: - lists its partners",
        ),
        KeyCode::Char('i' | 'I') if !ranging && focused == AFTER && stands_in(AFTER) => say(
            app,
            "This node is matched in another file, so it is not inserted: - lists its partners",
        ),
        KeyCode::Char('u') if !ranging && stands_in(focused) => say(
            app,
            "This node's partners are in other files: - lists them and removes one",
        ),
        KeyCode::Char('y') => {
            let Some(node) = cursor(focused) else {
                return say(app, "No node under the cursor to pin");
            };
            let file = [&before_file, &after_file][focused].clone();
            let pin = Pin {
                side: focused,
                label: format!(
                    "{} {} at {file}:{}",
                    SIDES[focused],
                    node.kind(),
                    node.start_position().row + 1
                ),
                file,
                path: path_for_node(node),
            };
            let session = app.changeset.as_mut()?;
            if session.pin.as_ref() == Some(&pin) {
                session.pin = None;
                return say(app, "Unpinned");
            }
            let status = format!(
                "Pinned {}: open any file (b, then Enter), put the {} cursor on its counterpart, \
                 and Y matches the two",
                pin.label,
                SIDES[1 - focused]
            );
            session.pin = Some(pin);
            say(app, status)
        }
        KeyCode::Char('Y') => {
            let Some(pin) = session.pin.clone() else {
                return say(app, "Nothing is pinned: y pins the node under the cursor");
            };
            let Some(other) = cursor(1 - pin.side) else {
                return say(app, "No node under the other cursor to match the pin with");
            };
            let other_path = Some(path_for_node(other));
            let request = if pin.side == BEFORE {
                PairRequest {
                    before: pin.file,
                    after: after_file,
                    before_path: Some(pin.path),
                    after_path: other_path,
                    keys: vec![KeyCode::Char('M')],
                    sites: Vec::new(),
                }
            } else {
                PairRequest {
                    before: before_file,
                    after: pin.file,
                    before_path: other_path,
                    after_path: Some(pin.path),
                    keys: vec![KeyCode::Char('M')],
                    sites: Vec::new(),
                }
            };
            back(Request::Pair(request))
        }
        KeyCode::Char('+') => {
            let status = match add_partner(app, frame, before, after) {
                Ok(status) => status,
                Err(error) => format!("Error: {error:#}"),
            };
            say(app, status)
        }
        KeyCode::Char('-') => {
            let Some(node) = cursor(focused) else {
                return say(app, "No node under the cursor");
            };
            let file = [&before_file, &after_file][focused].clone();
            let path = path_for_node(node);
            let partners = session.partners_of(focused, &file, &path);
            if partners.is_empty() {
                return say(app, "This node has no partner");
            }
            app.modal = Some(Modal::Partners {
                side: focused,
                file,
                path,
                partners,
                selected: 0,
            });
            Some(None)
        }
        KeyCode::Char('S') => {
            let status = match seed_pair(app, before, after) {
                Ok(status) => status,
                Err(error) => format!("Not seeded: {error:#}"),
            };
            say(app, status)
        }
        _ => None,
    }
}

/// `+`: matches the two cursor nodes as one more entry, whatever partners they have. An identical
/// pair takes its whole subtree, as `identical_with_children`; any other pair is filled in the
/// way `M` fills one.
fn add_partner(app: &mut App, frame: &FrameState, before: &Code, after: &Code) -> Result<String> {
    let (before_root, after_root) = frame.roots().context("no trees to match in")?;
    let b = frame
        .before_flat
        .node_for_id(app.before.cursor_id)
        .context("no before node under the cursor")?;
    let a = frame
        .after_flat
        .node_for_id(app.after.cursor_id)
        .context("no after node under the cursor")?;
    ensure!(
        b.kind() == a.kind(),
        "the kinds differ ({} and {}): + adds a partner of the same kind",
        b.kind(),
        a.kind()
    );
    let hashes = |code: &Code| {
        code.metadata
            .ast_metadata
            .as_ref()
            .map(|metadata| metadata.node_to_full_hash.clone())
            .unwrap_or_default()
    };
    let operation = classify_match_operation(
        b,
        a,
        frame.before_src,
        frame.after_src,
        &hashes(before),
        &hashes(after),
    );
    let session = app.changeset.as_mut().context("no changeset is open")?;
    let pair = session.pair.as_ref().context("no pair is open")?;
    let (before_file, after_file) = (pair.before.clone(), pair.after.clone());
    let node_ref = |file: &str, path: Vec<String>| NodeRef {
        file: file.to_string(),
        path,
    };
    let new: Vec<ChangesetEntry> = if operation == HumanOperation::Identical {
        HumanChangeset::identical_subtree(
            &session.trees,
            &node_ref(&before_file, path_for_node(b)),
            &node_ref(&after_file, path_for_node(a)),
        )?
    } else {
        let mut filled = Vec::new();
        let (mut matched, mut skipped) = (0, 0);
        auto_match_pair(
            &mut filled,
            &mut HashSet::new(),
            &mut HashSet::new(),
            &frame.caches,
            b,
            a,
            frame.before_src,
            frame.after_src,
            &precompute_paths(before_root),
            &precompute_paths(after_root),
            &mut matched,
            &mut skipped,
            &mut HashSet::new(),
            &mut HashSet::new(),
        );
        filled
            .into_iter()
            .filter_map(|entry| {
                Some(ChangesetEntry::matched(
                    entry.operation,
                    node_ref(&before_file, entry.before_path?),
                    node_ref(&after_file, entry.after_path?),
                ))
            })
            .collect()
    };
    let known: HashSet<(NodeRef, NodeRef)> = session
        .truth
        .entries
        .iter()
        .filter_map(|entry| Some((entry.before.clone()?, entry.after.clone()?)))
        .collect();
    let new: Vec<ChangesetEntry> = new
        .into_iter()
        .filter(|entry| {
            !known.contains(&(
                entry.before.clone().expect("a match"),
                entry.after.clone().expect("a match"),
            ))
        })
        .collect();
    ensure!(!new.is_empty(), "these two nodes are already matched");
    // A node that gains a partner is no longer deleted or inserted.
    let touched: [HashSet<&NodeRef>; 2] = [
        new.iter()
            .filter_map(|entry| entry.before.as_ref())
            .collect(),
        new.iter()
            .filter_map(|entry| entry.after.as_ref())
            .collect(),
    ];
    session.truth.entries.retain(|entry| {
        is_match(entry.operation)
            || !(entry
                .before
                .as_ref()
                .is_some_and(|node| touched[BEFORE].contains(node))
                || entry
                    .after
                    .as_ref()
                    .is_some_and(|node| touched[AFTER].contains(node)))
    });
    let added = new.len();
    session.truth.entries.extend(new);
    session.touch();
    let partners = session
        .partners_of(BEFORE, &before_file, &path_for_node(b))
        .len();
    reproject(app, before, after);
    app.mark_dirty();
    Ok(format!(
        "Added {added} entr(ies) as {operation:?}: the before node now has {partners} partner(s)"
    ))
}

/// `-`'s list: `-` removes the selected partner (the entry and every entry between the two
/// subtrees), Enter opens the pair view on it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn partners_key(
    app: &mut App,
    code: KeyCode,
    side: usize,
    file: String,
    path: Vec<String>,
    partners: Vec<Partner>,
    selected: usize,
    before: &Code,
    after: &Code,
) -> Option<OpenTarget> {
    let last = partners.len().saturating_sub(1);
    let reopen = |app: &mut App, partners: Vec<Partner>, selected: usize| {
        app.modal = Some(Modal::Partners {
            side,
            file: file.clone(),
            path: path.clone(),
            partners,
            selected,
        });
    };
    match code {
        KeyCode::Up | KeyCode::Char('k') => reopen(app, partners, selected.saturating_sub(1)),
        KeyCode::Down | KeyCode::Char('j') => reopen(app, partners, (selected + 1).min(last)),
        KeyCode::Esc | KeyCode::Char('q') => app.status = Some("Closed the partners".to_string()),
        KeyCode::Enter => {
            let partner = partners.get(selected)?;
            let (own, other) = (Some(path.clone()), Some(partner.path.clone()));
            let request = if side == BEFORE {
                PairRequest {
                    before: file.clone(),
                    after: partner.file.clone(),
                    before_path: own,
                    after_path: other,
                    ..Default::default()
                }
            } else {
                PairRequest {
                    before: partner.file.clone(),
                    after: file.clone(),
                    before_path: other,
                    after_path: own,
                    ..Default::default()
                }
            };
            return Some(OpenTarget::Changeset(Request::Pair(request)));
        }
        KeyCode::Char('-') => {
            let partner = partners.get(selected)?.clone();
            let session = app.changeset.as_mut()?;
            let ends = |entry: &ChangesetEntry| -> Option<bool> {
                let (b, a) = (entry.before.as_ref()?, entry.after.as_ref()?);
                let (own, other) = if side == BEFORE { (b, a) } else { (a, b) };
                Some(
                    own.file == file
                        && other.file == partner.file
                        && starts_with(&own.path, &path)
                        && starts_with(&other.path, &partner.path),
                )
            };
            let count = session.truth.entries.len();
            session
                .truth
                .entries
                .retain(|entry| !(is_match(entry.operation) && ends(entry) == Some(true)));
            let removed = count - session.truth.entries.len();
            session.touch();
            reproject(app, before, after);
            app.mark_dirty();
            app.status = Some(format!(
                "Removed the partner at {} ({removed} entr(ies) between the two subtrees)",
                partner.place()
            ));
        }
        _ => reopen(app, partners, selected),
    }
    None
}

pub(crate) fn render_partners(
    frame: &mut ratatui::Frame,
    area: Rect,
    side: usize,
    file: &str,
    path: &[String],
    partners: &[Partner],
    selected: usize,
) {
    let popup = centered_rect(70, 50, area);
    frame.render_widget(Clear, popup);
    let lines: Vec<Line> = partners
        .iter()
        .enumerate()
        .map(|(index, partner)| {
            let text = format!(
                "{} {}  {:?}  {}",
                SIDES[1 - side],
                partner.place(),
                partner.operation,
                partner.path.last().map_or("(root)", String::as_str)
            );
            if index == selected {
                Line::from(text).reversed()
            } else {
                text.into()
            }
        })
        .collect();
    let title = format!(
        "Partners of {} {file} {} - j/k move, - removes, Enter opens, Esc closes",
        SIDES[side],
        path.last().map_or("(root)", String::as_str)
    );
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
        popup,
    );
}

/// `S`: the single-file engine's matches and painted matches of the open pair, as a draft. Only
/// for a `same` or `renamed` pair with nothing recorded. Nodes with partners elsewhere are left
/// alone, and so is everything the engine calls deleted or inserted.
fn seed_pair(app: &mut App, before: &Code, after: &Code) -> Result<String> {
    let session = app.changeset.as_ref().context("no changeset is open")?;
    let pair = session.pair.as_ref().context("no pair is open")?;
    let related = session.truth.files.iter().any(|relation| {
        relation_names(relation, BEFORE, &pair.before)
            && relation_names(relation, AFTER, &pair.after)
            && matches!(
                relation.relation,
                FileRelationKind::Same | FileRelationKind::Renamed
            )
    });
    ensure!(
        related,
        "S seeds a same or renamed pair only (p on the board pairs two files): a match between \
         unrelated files is never taken from an engine"
    );
    let stand_ins = pair.pseudo[BEFORE].len() + pair.pseudo[AFTER].len();
    let painted: usize = app
        .mapping
        .text_mappings
        .iter()
        .map(|named| named.mapping.entries.len())
        .sum();
    ensure!(
        app.mapping.entries.len() == stand_ins && painted == 0,
        "this pair already has entries or painted ranges (! clears them)"
    );
    let (Some(before_tree), Some(after_tree)) = (before.ast.as_ref(), after.ast.as_ref()) else {
        bail!("no trees to seed");
    };
    let diff = diff_code(before, after);
    let ast = diff.ast.context("omnidiff produced no AST diff")?;
    let (before_root, after_root) = (before_tree.root_node(), after_tree.root_node());
    let paths = [precompute_paths(before_root), precompute_paths(after_root)];
    let by_id: [HashMap<usize, Node>; 2] = [before_root, after_root].map(|root| {
        let mut nodes = HashMap::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            nodes.insert(node.id(), node);
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
        nodes
    });
    let stands_in = |side: usize, id: usize| pair.pseudo[side].contains(&paths[side][&id]);

    let mut entries = Vec::new();
    let roots_free = !stands_in(BEFORE, before_root.id()) && !stands_in(AFTER, after_root.id());
    // One partner a node, as the engine's own node maps have it: its `mapping` also holds the
    // further pairs of an N:M group, which are not a single-file seed's to assert.
    let mut pairs: Vec<(usize, usize, HumanOperation)> = ast
        .before_node_map
        .iter()
        .filter_map(|(&b, &a)| {
            if ast.after_node_map.get(&a) != Some(&b) {
                return None;
            }
            let operation = match ast.mapping.get(&(b, a))?.operation {
                ASTMappingOperation::Identical => HumanOperation::Identical,
                ASTMappingOperation::Update => HumanOperation::Update,
                ASTMappingOperation::MatchButNotIdentical => HumanOperation::MatchButNotIdentical,
                _ => return None,
            };
            (by_id[BEFORE].contains_key(&b) && by_id[AFTER].contains_key(&a))
                .then_some((b, a, operation))
        })
        .collect();
    pairs.sort_by_key(|(b, a, _)| (paths[BEFORE][b].clone(), paths[AFTER][a].clone()));
    // The engine leaves the roots out of its maps (`ASTDiff::is_complete`).
    if roots_free
        && before_root.kind() == after_root.kind()
        && !ast.before_node_map.contains_key(&before_root.id())
    {
        let hash = |code: &Code, id: usize| {
            code.metadata
                .ast_metadata
                .as_ref()
                .and_then(|metadata| metadata.node_to_full_hash.get(&id).copied())
        };
        let identical = hash(before, before_root.id()).is_some()
            && hash(before, before_root.id()) == hash(after, after_root.id());
        pairs.push((
            before_root.id(),
            after_root.id(),
            if identical {
                HumanOperation::Identical
            } else {
                HumanOperation::MatchButNotIdentical
            },
        ));
    }
    let mut matched: [HashSet<usize>; 2] = Default::default();
    for (b, a, operation) in pairs {
        if stands_in(BEFORE, b) || stands_in(AFTER, a) {
            continue;
        }
        matched[BEFORE].insert(b);
        matched[AFTER].insert(a);
        entries.push(HumanMappingEntry {
            operation,
            before_path: Some(paths[BEFORE][&b].clone()),
            after_path: Some(paths[AFTER][&a].clone()),
        });
    }
    // What the engine matched nothing to stays undecided: in a changeset "deleted here" may be
    // "moved to another file", and that is the human's to say.
    let seeded = entries.len();

    // The painting, where it does not run into a range another file's painting holds.
    let overlaps = |a: HumanTextSpan, b: HumanTextSpan| {
        (a.start_row, a.start_column) < (b.end_row, b.end_column)
            && (b.start_row, b.start_column) < (a.end_row, a.end_column)
    };
    let held: [Vec<HumanTextSpan>; 2] = [BEFORE, AFTER].map(|side| {
        app.layer_paint
            .iter()
            .filter_map(|layer| layer.for_solution(&app.text_solution))
            .flat_map(|spans| spans[side].iter().map(|(span, _)| *span))
            .collect()
    });
    let (painting, paint_note) = match omnidiff_text_entries(before, after, Some(&ast)) {
        Ok(found) => {
            let free: Vec<HumanTextEntry> = found
                .into_iter()
                // Only what the engine matched: a range it calls deleted or inserted may have
                // moved to another file, as with the nodes above.
                .filter(|entry| entry.operation == HumanTextOperation::Match)
                .filter(|entry| {
                    ![&entry.before, &entry.after]
                        .into_iter()
                        .enumerate()
                        .any(|(side, spans)| {
                            spans
                                .iter()
                                .any(|span| held[side].iter().any(|other| overlaps(*span, *other)))
                        })
                })
                .collect();
            let note = format!(
                "{} painted range(s) into '{}'",
                free.len(),
                app.text_solution
            );
            (free, note)
        }
        Err(reason) => (Vec::new(), format!("no painting ({reason})")),
    };

    app.mapping.entries.extend(entries);
    let solution = app.text_solution.clone();
    solution_entries_mut(&mut app.mapping, &solution).extend(painting);
    app.algo_diff = Some(ast);
    app.mark_dirty();
    Ok(format!(
        "Seeded {seeded} entr(ies) and {paint_note} from omnidiff's single-file diff: a draft to \
         correct, not truth"
    ))
}

// ─── The leftover pool ──────────────────────────────────────────────────────────────────────

/// Per side and file: the paths with a match, and the removed paths with whether the mark takes
/// the children.
struct Marks {
    matched: [HashMap<String, HashSet<Vec<String>>>; 2],
    removed: [HashMap<String, HashMap<Vec<String>, bool>>; 2],
}

impl Marks {
    fn of(truth: &HumanChangeset) -> Self {
        let mut marks = Marks {
            matched: Default::default(),
            removed: Default::default(),
        };
        for entry in &truth.entries {
            for (side, node) in [&entry.before, &entry.after].into_iter().enumerate() {
                let Some(node) = node else { continue };
                if is_match(entry.operation) {
                    marks.matched[side]
                        .entry(node.file.clone())
                        .or_default()
                        .insert(node.path.clone());
                } else {
                    let with_children = matches!(
                        entry.operation,
                        HumanOperation::DeleteWithChildren | HumanOperation::InsertWithChildren
                    );
                    marks.removed[side]
                        .entry(node.file.clone())
                        .or_default()
                        .insert(node.path.clone(), with_children);
                }
            }
        }
        marks
    }

    fn is_matched(&self, side: usize, file: &str, path: &[String]) -> bool {
        self.matched[side]
            .get(file)
            .is_some_and(|paths| paths.contains(path))
    }

    /// Whether `path` is removed, by its own entry or an ancestor's with-children one.
    fn is_removed(&self, side: usize, file: &str, path: &[String]) -> bool {
        let Some(removed) = self.removed[side].get(file) else {
            return false;
        };
        removed.contains_key(path)
            || (0..path.len()).any(|length| removed.get(&path[..length]) == Some(&true))
    }

    /// Whether an ancestor's with-children mark covers `path`.
    fn is_covered(&self, side: usize, file: &str, path: &[String]) -> bool {
        self.removed[side].get(file).is_some_and(|removed| {
            (0..path.len()).any(|length| removed.get(&path[..length]) == Some(&true))
        })
    }
}

/// A subtree small enough to have twins everywhere (`)`, an identifier) is no candidate.
fn worth_pairing(node: Node) -> bool {
    node.child_count() > 0 && node.end_byte() - node.start_byte() >= 10
}

fn pool_node(
    side: usize,
    file: &str,
    node: Node,
    path: Vec<String>,
    nodes: usize,
    marks: &Marks,
    text: &str,
) -> PoolNode {
    let state = if !marks.is_removed(side, file, &path) {
        '?'
    } else if side == BEFORE {
        '-'
    } else {
        '+'
    };
    let snippet: String = text
        .get(node.start_byte()..node.end_byte())
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(60)
        .collect();
    PoolNode {
        side,
        file: file.to_string(),
        path,
        line: node.start_position().row + 1,
        kind: node.kind().to_string(),
        lines: node.end_position().row - node.start_position().row + 1,
        nodes,
        state,
        snippet,
    }
}

/// The pool: every outermost subtree with no partner anywhere in it, on both sides; a before and
/// an after subtree with the same full hash are one candidate row. Candidates first, then by size
/// (`by_size`) or by file and line.
pub(crate) fn pool_rows(session: &Session, by_size: bool) -> Vec<PoolRow> {
    let marks = Marks::of(&session.truth);
    let mut outermost: [Vec<(PoolNode, Option<u64>)>; 2] = Default::default();
    let mut by_hash: [HashMap<u64, Vec<PoolNode>>; 2] = Default::default();
    for side in [BEFORE, AFTER] {
        for (file, found) in session.side(side) {
            let (Some(root), Some(code)) = (found.root(), found.code.as_ref()) else {
                continue;
            };
            let hashes = code
                .metadata
                .ast_metadata
                .as_ref()
                .map(|metadata| &metadata.node_to_full_hash);
            let paths = precompute_paths(root);
            // Bottom up: whether the subtree holds a match, and how many nodes it has.
            let mut seen: HashMap<usize, (bool, usize)> = HashMap::new();
            fn measure(
                node: Node,
                seen: &mut HashMap<usize, (bool, usize)>,
                matched: &dyn Fn(usize) -> bool,
            ) -> (bool, usize) {
                let (mut any, mut count) = (matched(node.id()), 1);
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    let (child_any, child_count) = measure(child, seen, matched);
                    any |= child_any;
                    count += child_count;
                }
                seen.insert(node.id(), (any, count));
                (any, count)
            }
            measure(root, &mut seen, &|id| {
                marks.is_matched(side, file, &paths[&id])
            });
            let mut stack = vec![(root, false)];
            while let Some((node, inside)) = stack.pop() {
                let (any, count) = seen[&node.id()];
                let leftover = inside || !any;
                if leftover {
                    let hash = hashes.and_then(|hashes| hashes.get(&node.id()).copied());
                    let entry = pool_node(
                        side,
                        file,
                        node,
                        paths[&node.id()].clone(),
                        count,
                        &marks,
                        &code.contents,
                    );
                    if let (Some(hash), true) = (hash, worth_pairing(node)) {
                        by_hash[side].entry(hash).or_default().push(entry.clone());
                    }
                    if !inside {
                        outermost[side].push((entry, hash.filter(|_| worth_pairing(node))));
                    }
                }
                let mut cursor = node.walk();
                let children: Vec<Node> = node.children(&mut cursor).collect();
                stack.extend(children.into_iter().rev().map(|child| (child, leftover)));
            }
        }
    }

    let mut rows: Vec<PoolRow> = Vec::new();
    let mut paired: HashSet<(usize, String, Vec<String>)> = HashSet::new();
    for side in [BEFORE, AFTER] {
        for (node, hash) in &outermost[side] {
            let Some(twins) = hash.and_then(|hash| by_hash[1 - side].get(&hash)) else {
                continue;
            };
            for twin in twins {
                let (before, after) = if side == BEFORE {
                    (node, twin)
                } else {
                    (twin, node)
                };
                let row = PoolRow {
                    before: Some(before.clone()),
                    after: Some(after.clone()),
                };
                if !rows.contains(&row) {
                    for end in [before, after] {
                        paired.insert((end.side, end.file.clone(), end.path.clone()));
                    }
                    rows.push(row);
                }
            }
        }
    }
    for side in [BEFORE, AFTER] {
        for (node, _) in &outermost[side] {
            if paired.contains(&(node.side, node.file.clone(), node.path.clone())) {
                continue;
            }
            rows.push(if side == BEFORE {
                PoolRow {
                    before: Some(node.clone()),
                    after: None,
                }
            } else {
                PoolRow {
                    before: None,
                    after: Some(node.clone()),
                }
            });
        }
    }
    let place = |row: &PoolRow| {
        let node = row
            .before
            .as_ref()
            .or(row.after.as_ref())
            .expect("a row has a node");
        (node.side, node.file.clone(), node.line)
    };
    rows.sort_by(|a, b| {
        b.is_candidate().cmp(&a.is_candidate()).then_with(|| {
            if by_size {
                b.size()
                    .cmp(&a.size())
                    .then_with(|| place(a).cmp(&place(b)))
            } else {
                place(a).cmp(&place(b))
            }
        })
    });
    rows
}

impl Session {
    pub(crate) fn open_pool(&mut self) {
        let rows = pool_rows(self, true);
        let candidates = rows.iter().filter(|row| row.is_candidate()).count();
        self.status = format!(
            "{} leftover subtree(s), {candidates} hash-identical candidate pair(s): a accepts one, \
             Enter opens it",
            rows.len()
        );
        self.overlay = Some(Overlay::Pool {
            rows,
            selected: 0,
            by_size: true,
        });
    }

    pub(crate) fn open_violations(&mut self) {
        self.violations = validate(&self.truth, &self.trees);
        if self.violations.is_empty() {
            self.status = format!(
                "The truth in memory validates{}",
                if self.truth.is_solved(&self.trees) {
                    ", and every file is in a solved relation"
                } else {
                    "; not every relation is solved yet"
                }
            );
            self.overlay = None;
        } else {
            self.status = format!(
                "{} violation(s): j/k move, Enter opens the pair it names, Esc closes",
                self.violations.len()
            );
            self.overlay = Some(Overlay::Violations { selected: 0 });
        }
    }

    /// The pool's `a`: the candidate's two subtrees matched as identical, children and all, in
    /// place of whatever delete or insert marks were on them.
    fn accept_candidate(&mut self, row: &PoolRow) -> Result<String> {
        let (Some(before), Some(after)) = (&row.before, &row.after) else {
            bail!("this subtree has no hash-identical twin to accept");
        };
        let marks = Marks::of(&self.truth);
        for node in [before, after] {
            ensure!(
                !marks.is_covered(node.side, &node.file, &node.path),
                "{} {}:{} is under a with-children mark: unmark that ancestor first",
                SIDES[node.side],
                node.file,
                node.line
            );
        }
        let new =
            HumanChangeset::identical_subtree(&self.trees, &before.node_ref(), &after.node_ref())?;
        let inside = |side: usize, node: &Option<NodeRef>| {
            let end = [before, after][side];
            node.as_ref()
                .is_some_and(|node| node.file == end.file && starts_with(&node.path, &end.path))
        };
        self.truth.entries.retain(|entry| {
            is_match(entry.operation)
                || !(inside(BEFORE, &entry.before) || inside(AFTER, &entry.after))
        });
        let added = new.len();
        self.truth.entries.extend(new);
        self.touch();
        Ok(format!(
            "Matched before {}:{} with after {}:{} as identical, {added} node pair(s)",
            before.file, before.line, after.file, after.line
        ))
    }

    /// The pair view a pool row opens: a candidate's two files at its two nodes; for a subtree
    /// with no twin, its file beside its counterpart (or the board's cursor file).
    fn pool_request(&self, row: &PoolRow) -> Option<PairRequest> {
        let counterpart = |side: usize, file: &str| -> Option<String> {
            self.truth
                .files
                .iter()
                .chain(&self.proposals)
                .filter(|relation| relation_names(relation, side, file))
                .find_map(|relation| [&relation.before, &relation.after][1 - side].clone())
                .or_else(|| self.selected(1 - side))
        };
        // A subtree with no twin opens beside the pinned one, when that is on the other side.
        let pinned = |side: usize| self.pin.as_ref().filter(|pin| pin.side == side);
        Some(match (&row.before, &row.after) {
            (Some(before), None) if pinned(AFTER).is_some() => {
                let pin = pinned(AFTER)?;
                PairRequest {
                    before: before.file.clone(),
                    after: pin.file.clone(),
                    before_path: Some(before.path.clone()),
                    after_path: Some(pin.path.clone()),
                    ..Default::default()
                }
            }
            (None, Some(after)) if pinned(BEFORE).is_some() => {
                let pin = pinned(BEFORE)?;
                PairRequest {
                    before: pin.file.clone(),
                    after: after.file.clone(),
                    before_path: Some(pin.path.clone()),
                    after_path: Some(after.path.clone()),
                    ..Default::default()
                }
            }
            (Some(before), Some(after)) => PairRequest {
                before: before.file.clone(),
                after: after.file.clone(),
                before_path: Some(before.path.clone()),
                after_path: Some(after.path.clone()),
                ..Default::default()
            },
            (Some(before), None) => PairRequest {
                before: before.file.clone(),
                after: counterpart(BEFORE, &before.file)?,
                before_path: Some(before.path.clone()),
                ..Default::default()
            },
            (None, Some(after)) => PairRequest {
                before: counterpart(AFTER, &after.file)?,
                after: after.file.clone(),
                after_path: Some(after.path.clone()),
                ..Default::default()
            },
            (None, None) => return None,
        })
    }
}

// ─── The board ──────────────────────────────────────────────────────────────────────────────

/// What a key on the board asks of the loop around it.
pub(crate) enum BoardEnd {
    Stay,
    Quit,
    Open(PairRequest),
}

impl Session {
    /// The confirmed relation between the two cursor files, else the focused file's only one.
    fn relation_at_cursors(&self) -> Option<usize> {
        let (before, after) = (self.selected(BEFORE), self.selected(AFTER));
        let between = self.truth.files.iter().position(|relation| {
            relation.before.is_some()
                && relation.after.is_some()
                && relation.before == before
                && relation.after == after
        });
        between.or_else(|| {
            let file = self.selected(self.focus)?;
            let mut own = self
                .truth
                .files
                .iter()
                .enumerate()
                .filter(|(_, relation)| relation_names(relation, self.focus, &file));
            let first = own.next()?;
            own.next().is_none().then_some(first.0)
        })
    }

    fn drop_proposals(&mut self, before: Option<&str>, after: Option<&str>) {
        self.proposals.retain(|relation| {
            !(before.is_some_and(|path| relation_names(relation, BEFORE, path))
                || after.is_some_and(|path| relation_names(relation, AFTER, path)))
        });
    }

    /// A split or merge left with one counterpart is a plain pair again.
    fn settle_kinds(&mut self) {
        let count = |files: &[FileRelation], side: usize, path: &Option<String>| {
            files
                .iter()
                .filter(|relation| {
                    relation.before.is_some()
                        && relation.after.is_some()
                        && [&relation.before, &relation.after][side] == path
                })
                .count()
        };
        for index in 0..self.truth.files.len() {
            let relation = &self.truth.files[index];
            let (Some(before), Some(after)) = (&relation.before, &relation.after) else {
                continue;
            };
            let alone = count(&self.truth.files, BEFORE, &relation.before) == 1
                && count(&self.truth.files, AFTER, &relation.after) == 1;
            if alone
                && matches!(
                    relation.relation,
                    FileRelationKind::Split | FileRelationKind::Merged
                )
            {
                self.truth.files[index].relation = same_or_renamed(before, after);
            }
        }
    }

    fn pair_files(&mut self) -> Result<String> {
        let before = self.selected(BEFORE).context("no before file selected")?;
        let after = self.selected(AFTER).context("no after file selected")?;
        for (side, file) in [(BEFORE, &before), (AFTER, &after)] {
            ensure!(
                !self.has_relation(side, file),
                "{} {file} already has a relation: u unpairs it, S splits, M merges",
                SIDES[side]
            );
        }
        self.drop_proposals(Some(&before), Some(&after));
        let relation = same_or_renamed(&before, &after);
        self.truth.files.push(FileRelation {
            before: Some(before.clone()),
            after: Some(after.clone()),
            relation,
            solved: false,
            verdict: None,
        });
        self.touch();
        Ok(format!(
            "Paired {before} -> {after} as {}",
            kind_name(relation)
        ))
    }

    /// `S` (`fan` = before: one before file, several after) and `M` (several before, one after):
    /// relates the two cursor files, and makes every relation of the shared file the same kind.
    fn fan_files(&mut self, fan: usize) -> Result<String> {
        let before = self.selected(BEFORE).context("no before file selected")?;
        let after = self.selected(AFTER).context("no after file selected")?;
        let kind = if fan == BEFORE {
            FileRelationKind::Split
        } else {
            FileRelationKind::Merged
        };
        let shared = if fan == BEFORE { &before } else { &after };
        let one_sided = |relation: &FileRelation| {
            (relation.before.is_none() || relation.after.is_none())
                && (relation_names(relation, BEFORE, &before)
                    || relation_names(relation, AFTER, &after))
        };
        self.truth.files.retain(|relation| !one_sided(relation));
        self.drop_proposals(Some(&before), Some(&after));
        let exists = self.truth.files.iter().any(|relation| {
            relation_names(relation, BEFORE, &before) && relation_names(relation, AFTER, &after)
        });
        if !exists {
            self.truth.files.push(FileRelation {
                before: Some(before.clone()),
                after: Some(after.clone()),
                relation: kind,
                solved: false,
                verdict: None,
            });
        }
        let mut members = 0;
        for relation in &mut self.truth.files {
            if relation.before.is_some()
                && relation.after.is_some()
                && relation_names(relation, fan, shared)
            {
                relation.relation = kind;
                members += 1;
            }
        }
        self.touch();
        Ok(format!(
            "{} {shared} is {} with {members} {} file(s){}",
            SIDES[fan],
            kind_name(kind),
            SIDES[1 - fan],
            if members < 2 {
                ": put the other cursor on the next one and press the key again"
            } else {
                ""
            }
        ))
    }

    fn one_sided(&mut self, side: usize) -> Result<String> {
        let file = self.selected(side).context("no file selected")?;
        ensure!(
            !self.has_relation(side, &file),
            "{} {file} already has a relation: u unpairs it first",
            SIDES[side]
        );
        let (before, after, kind) = if side == BEFORE {
            (Some(file.clone()), None, FileRelationKind::Removed)
        } else {
            (None, Some(file.clone()), FileRelationKind::Added)
        };
        self.drop_proposals(before.as_deref(), after.as_deref());
        self.truth.files.push(FileRelation {
            before,
            after,
            relation: kind,
            solved: false,
            verdict: None,
        });
        self.touch();
        Ok(format!("Marked {file} {}", kind_name(kind)))
    }

    fn unpair(&mut self) -> Result<String> {
        let (before, after) = (self.selected(BEFORE), self.selected(AFTER));
        let between = |relation: &FileRelation| {
            relation.before.is_some()
                && relation.after.is_some()
                && relation.before == before
                && relation.after == after
        };
        let known = self.truth.files.len() + self.proposals.len();
        if self.truth.files.iter().chain(&self.proposals).any(between) {
            self.truth.files.retain(|relation| !between(relation));
            self.proposals.retain(|relation| !between(relation));
        } else {
            let file = self.selected(self.focus).context("no file selected")?;
            let side = self.focus;
            self.truth
                .files
                .retain(|relation| !relation_names(relation, side, &file));
            self.proposals
                .retain(|relation| !relation_names(relation, side, &file));
        }
        let removed = known - self.truth.files.len() - self.proposals.len();
        ensure!(removed > 0, "nothing to unpair here");
        self.settle_kinds();
        self.touch();
        Ok(format!(
            "Removed {removed} relation(s); the nodes matched between the files stay as they are"
        ))
    }

    /// `c`: the focused file's proposals become truth. `all` is `C`.
    fn confirm(&mut self, all: bool) -> Result<String> {
        let file = self.selected(self.focus);
        let side = self.focus;
        let (taken, left): (Vec<FileRelation>, Vec<FileRelation>) =
            std::mem::take(&mut self.proposals)
                .into_iter()
                .partition(|relation| {
                    all || file
                        .as_ref()
                        .is_some_and(|file| relation_names(relation, side, file))
                });
        self.proposals = left;
        ensure!(!taken.is_empty(), "no proposed relation to confirm here");
        let count = taken.len();
        self.truth.files.extend(taken);
        self.touch();
        Ok(format!("Confirmed {count} proposed relation(s)"))
    }

    fn toggle_solved(&mut self) -> Result<String> {
        let index = self.relation_at_cursors().context(
            "no confirmed relation here: put both cursors on a pair, or confirm a proposal (c)",
        )?;
        let relation = &mut self.truth.files[index];
        relation.solved = !relation.solved;
        let status = format!(
            "{} {} is {}",
            kind_name(relation.relation),
            relation
                .before
                .iter()
                .chain(&relation.after)
                .cloned()
                .collect::<Vec<_>>()
                .join(" -> "),
            if relation.solved {
                "solved"
            } else {
                "not solved"
            }
        );
        self.touch();
        Ok(status)
    }

    /// Whether the two cursor files can open as a pair view; otherwise the verdict prompt.
    fn open_selected(&mut self) -> Result<BoardEnd> {
        let before = self.selected(BEFORE).context("no before file selected")?;
        let after = self.selected(AFTER).context("no after file selected")?;
        let is_text = |side: usize, file: &str| {
            self.side(side)
                .get(file)
                .is_some_and(|file| file.code.is_some())
        };
        if is_text(BEFORE, &before) && is_text(AFTER, &after) {
            return Ok(BoardEnd::Open(PairRequest {
                before,
                after,
                ..Default::default()
            }));
        }
        self.verdict_prompt()?;
        Ok(BoardEnd::Stay)
    }

    fn verdict_prompt(&mut self) -> Result<()> {
        let (before, after) = (self.selected(BEFORE), self.selected(AFTER));
        let relation = self
            .truth
            .files
            .iter()
            .position(|relation| {
                relation.before.is_some()
                    && relation.after.is_some()
                    && relation.before == before
                    && relation.after == after
            })
            .context("a verdict is for a confirmed pair of files: pair them first (p)")?;
        let has_trees = |side: usize, file: &Option<String>| {
            file.as_ref()
                .and_then(|file| self.side(side).get(file))
                .is_some_and(|file| file.root().is_some())
        };
        ensure!(
            !(has_trees(BEFORE, &before) && has_trees(AFTER, &after)),
            "both files have trees: their nodes are the truth, not a verdict (Enter opens them)"
        );
        self.overlay = Some(Overlay::Verdict { relation });
        self.status = "What changed between the two files? A level, c for can't judge".to_string();
        Ok(())
    }

    fn file_stats(&mut self) -> &[BTreeMap<String, FileStats>; 2] {
        if self.stats.is_none() {
            let marks = Marks::of(&self.truth);
            let stats = [BEFORE, AFTER].map(|side| {
                self.side(side)
                    .iter()
                    .map(|(path, file)| {
                        let language = match &file.code {
                            None => "binary".to_string(),
                            Some(code) => match code.metadata.language {
                                Some(language) if file.root().is_some() => {
                                    omnidiff::code::language::human_name(language)
                                }
                                _ => "text".to_string(),
                            },
                        };
                        let lines = file.text().map_or(0, |text| text.lines().count());
                        let unmarked = file.root().map(|root| {
                            let paths = precompute_paths(root);
                            paths
                                .values()
                                .filter(|node| {
                                    !marks.is_matched(side, path, node)
                                        && !marks.is_removed(side, path, node)
                                })
                                .count()
                        });
                        (
                            path.clone(),
                            FileStats {
                                language,
                                lines,
                                unmarked,
                            },
                        )
                    })
                    .collect()
            });
            self.stats = Some(stats);
        }
        self.stats.as_ref().expect("just filled")
    }
}

fn overlay_key(session: &mut Session, code: KeyCode) -> BoardEnd {
    let Some(overlay) = session.overlay.take() else {
        return BoardEnd::Stay;
    };
    match overlay {
        Overlay::Help { scroll } => match code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {}
            KeyCode::Down | KeyCode::Char('j') => {
                session.overlay = Some(Overlay::Help {
                    scroll: scroll.saturating_add(1),
                })
            }
            KeyCode::Up | KeyCode::Char('k') => {
                session.overlay = Some(Overlay::Help {
                    scroll: scroll.saturating_sub(1),
                })
            }
            _ => session.overlay = Some(Overlay::Help { scroll }),
        },
        Overlay::Verdict { relation } => {
            let judgement = match code {
                KeyCode::Char(digit @ '0'..='5') => Level::ALL
                    .get(digit as usize - '0' as usize)
                    .map(|level| Judgement::Verdict(Verdict::new(*level))),
                KeyCode::Char('c') => Some(Judgement::CantJudge(String::new())),
                KeyCode::Esc | KeyCode::Char('q') => {
                    session.status = "No verdict given".to_string();
                    return BoardEnd::Stay;
                }
                _ => None,
            };
            match (judgement, session.truth.files.get_mut(relation)) {
                (Some(judgement), Some(found)) => {
                    session.status = format!(
                        "Verdict: {} (x marks the relation solved)",
                        judgement.label_in(Vocabulary::Pictures)
                    );
                    found.verdict = Some(judgement);
                    session.touch();
                }
                _ => session.overlay = Some(Overlay::Verdict { relation }),
            }
        }
        Overlay::Violations { selected } => {
            let last = session.violations.len().saturating_sub(1);
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('V') => {}
                KeyCode::Down | KeyCode::Char('j') => {
                    session.overlay = Some(Overlay::Violations {
                        selected: (selected + 1).min(last),
                    })
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    session.overlay = Some(Overlay::Violations {
                        selected: selected.saturating_sub(1),
                    })
                }
                KeyCode::Enter => {
                    let found = session.violations.get(selected);
                    match found.and_then(|found| Some((found.pair.clone()?, found.sites.clone()))) {
                        Some(((before, after), sites)) => {
                            return BoardEnd::Open(PairRequest {
                                before,
                                after,
                                sites,
                                ..Default::default()
                            });
                        }
                        None => {
                            session.status =
                                "This violation names no pair of files to open".to_string();
                            session.overlay = Some(Overlay::Violations { selected });
                        }
                    }
                }
                _ => session.overlay = Some(Overlay::Violations { selected }),
            }
        }
        Overlay::Pool {
            rows,
            selected,
            by_size,
        } => {
            let last = rows.len().saturating_sub(1);
            let keep = |session: &mut Session, rows, selected, by_size| {
                session.overlay = Some(Overlay::Pool {
                    rows,
                    selected,
                    by_size,
                })
            };
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('L') => {}
                KeyCode::Down | KeyCode::Char('j') => {
                    keep(session, rows, (selected + 1).min(last), by_size)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    keep(session, rows, selected.saturating_sub(1), by_size)
                }
                KeyCode::Char('g') => keep(session, rows, 0, by_size),
                KeyCode::Char('G') => keep(session, rows, last, by_size),
                KeyCode::Char('s') => {
                    session.status = if by_size {
                        "Pool sorted by file and line".to_string()
                    } else {
                        "Pool sorted by size".to_string()
                    };
                    let rows = pool_rows(session, !by_size);
                    keep(session, rows, 0, !by_size);
                }
                KeyCode::Char('a') => {
                    let outcome = match rows.get(selected) {
                        Some(row) => session.accept_candidate(row),
                        None => Err(anyhow::anyhow!("the pool is empty")),
                    };
                    match outcome {
                        Ok(status) => {
                            session.status = status;
                            let rows = pool_rows(session, by_size);
                            let selected = selected.min(rows.len().saturating_sub(1));
                            keep(session, rows, selected, by_size);
                        }
                        Err(error) => {
                            session.status = format!("Error: {error:#}");
                            keep(session, rows, selected, by_size);
                        }
                    }
                }
                KeyCode::Char('y') => {
                    let node = rows
                        .get(selected)
                        .and_then(|row| row.before.as_ref().xor(row.after.as_ref()));
                    session.status = match node {
                        Some(node) => {
                            let pin = Pin {
                                side: node.side,
                                file: node.file.clone(),
                                path: node.path.clone(),
                                label: format!(
                                    "{} {} at {}:{}",
                                    SIDES[node.side], node.kind, node.file, node.line
                                ),
                            };
                            let status = format!(
                                "Pinned {}: Enter on {} subtree opens the two side by side",
                                pin.label,
                                ["an after", "a before"][node.side]
                            );
                            session.pin = Some(pin);
                            status
                        }
                        None => "y pins a subtree that has no twin; a candidate opens with Enter"
                            .to_string(),
                    };
                    keep(session, rows, selected, by_size);
                }
                KeyCode::Enter => {
                    let request = rows.get(selected).and_then(|row| session.pool_request(row));
                    match request {
                        Some(request) => return BoardEnd::Open(request),
                        None => {
                            session.status = "No file to open this beside".to_string();
                            keep(session, rows, selected, by_size);
                        }
                    }
                }
                _ => keep(session, rows, selected, by_size),
            }
        }
    }
    BoardEnd::Stay
}

/// One key on the board, or on what is drawn over it.
pub(crate) fn board_key(app: &mut App, code: KeyCode) -> BoardEnd {
    let Some(session) = app.changeset.as_mut() else {
        return BoardEnd::Quit;
    };
    if session.overlay.is_some() {
        return overlay_key(session, code);
    }
    let count = session.side(session.focus).len();
    let focus = session.focus;
    let armed = std::mem::take(&mut session.quit_armed);
    let outcome: Result<String> = match code {
        KeyCode::Tab | KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Left | KeyCode::Right => {
            session.focus = 1 - session.focus;
            return BoardEnd::Stay;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            session.cursor[focus] = (session.cursor[focus] + 1).min(count.saturating_sub(1));
            return BoardEnd::Stay;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            session.cursor[focus] = session.cursor[focus].saturating_sub(1);
            return BoardEnd::Stay;
        }
        KeyCode::Char('g') => {
            session.cursor[focus] = 0;
            return BoardEnd::Stay;
        }
        KeyCode::Char('G') => {
            session.cursor[focus] = count.saturating_sub(1);
            return BoardEnd::Stay;
        }
        KeyCode::Char('f') => (|| {
            let file = session.selected(focus).context("no file selected")?;
            let other = session
                .truth
                .files
                .iter()
                .chain(&session.proposals)
                .filter(|relation| relation_names(relation, focus, &file))
                .find_map(|relation| [&relation.before, &relation.after][1 - focus].clone())
                .context("this file has no counterpart")?;
            let index = session
                .side(1 - focus)
                .keys()
                .position(|path| *path == other)
                .context("its counterpart is not in the changeset")?;
            session.cursor[1 - focus] = index;
            Ok(format!("The {} cursor is on {other}", SIDES[1 - focus]))
        })(),
        KeyCode::Enter => match session.open_selected() {
            Ok(end) => return end,
            Err(error) => Err(error),
        },
        KeyCode::Char('e') => session.verdict_prompt().map(|()| session.status.clone()),
        KeyCode::Char('p') => session.pair_files(),
        KeyCode::Char('S') => session.fan_files(BEFORE),
        KeyCode::Char('M') => session.fan_files(AFTER),
        KeyCode::Char('a') => session.one_sided(AFTER),
        KeyCode::Char('d') => session.one_sided(BEFORE),
        KeyCode::Char('u') => session.unpair(),
        KeyCode::Char('c') => session.confirm(false),
        KeyCode::Char('C') => session.confirm(true),
        KeyCode::Char('x') => session.toggle_solved(),
        KeyCode::Char('L') => {
            session.open_pool();
            return BoardEnd::Stay;
        }
        KeyCode::Char('V') => {
            session.open_violations();
            return BoardEnd::Stay;
        }
        KeyCode::Char('s') => session.save(),
        KeyCode::Char('?') => {
            // The one sheet for every session, opened where the changeset's keys are.
            let scroll = HELP_TEXT
                .lines()
                .position(|line| line.starts_with("CHANGESETS"))
                .unwrap_or(0);
            session.overlay = Some(Overlay::Help {
                scroll: scroll as u16,
            });
            return BoardEnd::Stay;
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            if session.dirty && !armed {
                session.quit_armed = true;
                Ok("Unsaved changes: s saves, q again quits without saving".to_string())
            } else {
                return BoardEnd::Quit;
            }
        }
        _ => return BoardEnd::Stay,
    };
    session.status = match outcome {
        Ok(status) => status,
        Err(error) => format!("Error: {error:#}"),
    };
    BoardEnd::Stay
}

/// The relations of one file as the board writes them after it, proposals dim.
fn relation_spans(session: &Session, side: usize, file: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let describe = |relation: &FileRelation| {
        let other = [&relation.before, &relation.after][1 - side].as_deref();
        let arrow = if side == BEFORE { "->" } else { "<-" };
        match other {
            Some(other) if other == file => kind_name(relation.relation).to_string(),
            Some(other) => format!("{} {arrow} {other}", kind_name(relation.relation)),
            None => kind_name(relation.relation).to_string(),
        }
    };
    for relation in &session.truth.files {
        if !relation_names(relation, side, file) {
            continue;
        }
        let verdict = relation
            .verdict
            .as_ref()
            .map(|verdict| format!(" ({})", verdict.label_in(Vocabulary::Pictures)))
            .unwrap_or_default();
        let text = format!(
            "  {} {}{verdict}",
            if relation.solved { "[x]" } else { "[ ]" },
            describe(relation)
        );
        spans.push(if relation.solved {
            text.green()
        } else {
            text.cyan()
        });
    }
    for relation in &session.proposals {
        if relation_names(relation, side, file) {
            spans.push(format!("  ? {} (proposed)", describe(relation)).dim());
        }
    }
    if spans.is_empty() {
        spans.push("  no relation".yellow());
    }
    spans
}

fn board_lines(session: &mut Session, side: usize) -> Vec<Line<'static>> {
    let files = session.files(side);
    let width = files.iter().map(|file| file.len()).max().unwrap_or(0);
    session.file_stats();
    let session: &Session = session;
    let stats = &session.stats.as_ref().expect("just filled")[side];
    files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let stat = &stats[file];
            let nodes = match stat.unmarked {
                None => "no tree".to_string(),
                Some(0) => "all marked".to_string(),
                Some(count) => format!("{count} unmarked"),
            };
            let selected = index == session.cursor[side];
            let mut spans: Vec<Span> = vec![
                if selected { "> " } else { "  " }.into(),
                format!("{file:<width$}").bold(),
                format!(
                    "  {:<10} {:>5} lines  {nodes:<13}",
                    stat.language, stat.lines
                )
                .dim(),
            ];
            spans.extend(relation_spans(session, side, file));
            let line = Line::from(spans);
            match (selected, session.focus == side) {
                (true, true) => line.reversed(),
                (true, false) => line.underlined(),
                _ => line,
            }
        })
        .collect()
}

fn pool_line(row: &PoolRow) -> String {
    let describe = |node: &PoolNode| {
        format!(
            "{} {}:{} {} {}L/{}n [{}]",
            SIDES[node.side], node.file, node.line, node.kind, node.lines, node.nodes, node.state
        )
    };
    match (&row.before, &row.after) {
        (Some(before), Some(after)) => format!(
            "= {}  <=>  {}  | {}",
            describe(before),
            describe(after),
            before.snippet
        ),
        (Some(node), None) | (None, Some(node)) => {
            format!("  {}  | {}", describe(node), node.snippet)
        }
        (None, None) => String::new(),
    }
}

fn list_popup(frame: &mut ratatui::Frame, area: Rect, title: String, rows: Vec<Line>, at: usize) {
    let popup = centered_rect(94, 80, area);
    frame.render_widget(Clear, popup);
    let height = popup.height.saturating_sub(2).max(1) as usize;
    let offset = at
        .saturating_sub(height / 2)
        .min(rows.len().saturating_sub(height));
    let visible: Vec<Line> = rows.into_iter().skip(offset).take(height).collect();
    frame.render_widget(
        Paragraph::new(visible).block(Block::default().borders(Borders::ALL).title(title)),
        popup,
    );
}

pub(crate) fn draw_board(frame: &mut ratatui::Frame, app: &mut App) {
    let Some(session) = app.changeset.as_mut() else {
        return;
    };
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(4),
        ])
        .split(area);
    let solved = session
        .truth
        .files
        .iter()
        .filter(|relation| relation.solved)
        .count();
    let title = format!(
        " human_solver — {} [changeset{}] — {} relation(s), {solved} solved, {} proposed{}{}",
        session.name,
        if session.is_sample { " sample" } else { "" },
        session.truth.files.len(),
        session.proposals.len(),
        session
            .pin
            .as_ref()
            .map(|pin| format!(" — pinned: {}", pin.label))
            .unwrap_or_default(),
        if session.dirty { "  [UNSAVED]" } else { "" },
    );
    frame.render_widget(Paragraph::new(title.bold()), chunks[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    for side in [BEFORE, AFTER] {
        let lines = board_lines(session, side);
        let title = format!("{} — {} file(s)", ["Before", "After"][side], lines.len());
        let block = Block::default().borders(Borders::ALL).title(title);
        let block = if session.focus == side {
            block.yellow()
        } else {
            block
        };
        frame.render_widget(Paragraph::new(lines).block(block), columns[side]);
    }
    frame.render_widget(
        Paragraph::new(format!("{}\n{BOARD_FOOTER}", session.status)).wrap(Wrap { trim: true }),
        chunks[2],
    );

    let mark = |index: usize, selected: usize, text: String| -> Line<'static> {
        if index == selected {
            Line::from(text).reversed()
        } else {
            text.into()
        }
    };
    match &session.overlay {
        None => {}
        Some(Overlay::Help { scroll }) => {
            let popup = centered_rect(90, 90, area);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(HELP_TEXT)
                    .block(Block::default().borders(Borders::ALL).title("Help"))
                    .scroll((*scroll, 0)),
                popup,
            );
        }
        Some(Overlay::Verdict { relation }) => {
            let popup = centered_rect(60, 40, area);
            frame.render_widget(Clear, popup);
            let mut lines: Vec<Line> = Vec::new();
            if let Some(found) = session.truth.files.get(*relation) {
                lines.push(
                    format!(
                        "{} -> {}",
                        found.before.as_deref().unwrap_or("-"),
                        found.after.as_deref().unwrap_or("-")
                    )
                    .bold()
                    .into(),
                );
            }
            for (index, level) in Level::ALL.iter().enumerate() {
                lines.push(format!("{index}  {}", level.name_in(Vocabulary::Pictures)).into());
            }
            lines.push("c  can't judge".into());
            lines.push("Esc  no verdict".dim().into());
            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Verdict for a pair without trees"),
                ),
                popup,
            );
        }
        Some(Overlay::Violations { selected }) => {
            let rows = session
                .violations
                .iter()
                .enumerate()
                .map(|(index, found)| mark(index, *selected, found.to_string()))
                .collect();
            let title = format!(
                "Violations — {} — j/k move, Enter opens the pair, Esc closes",
                session.violations.len()
            );
            list_popup(frame, area, title, rows, *selected);
        }
        Some(Overlay::Pool {
            rows,
            selected,
            by_size,
        }) => {
            let lines = rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let line = mark(index, *selected, pool_line(row));
                    if row.is_candidate() && index != *selected {
                        line.cyan()
                    } else {
                        line
                    }
                })
                .collect();
            let title = format!(
                "Leftover pool — {} subtree(s) without a partner, = marks hash-identical \
                 candidates — sorted by {} (s) — a accept, Enter open, Esc close",
                rows.len(),
                if *by_size { "size" } else { "file" }
            );
            list_popup(frame, area, title, lines, *selected);
        }
    }
}

/// Which view a board key went to, as the key log names it.
fn board_mode(session: &Session) -> &'static str {
    match session.overlay {
        None => "board",
        Some(Overlay::Pool { .. }) => "pool",
        Some(Overlay::Violations { .. }) => "violations",
        Some(Overlay::Verdict { .. }) => "verdict",
        Some(Overlay::Help { .. }) => "help",
    }
}

/// The changeset session's loop: the board, and under it a code session for each pair opened.
pub(crate) fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let mut needs_redraw = true;
    loop {
        if needs_redraw {
            terminal.draw(|frame| draw_board(frame, app))?;
            needs_redraw = false;
        }
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        needs_redraw = true;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let Some(session) = app.changeset.as_ref() else {
            return Ok(());
        };
        let mode = board_mode(session);
        let (revision, was_dirty) = (session.truth.clone(), session.dirty);
        let end = board_key(app, key.code);
        let edited = app
            .changeset
            .as_ref()
            .is_some_and(|session| session.dirty && (!was_dirty || session.truth != revision));
        if let Some(log) = app.key_log.as_mut() {
            log.record(&app.name, mode, &key_name(key, mode), edited);
        }
        let mut request = match end {
            BoardEnd::Stay => continue,
            BoardEnd::Quit => return Ok(()),
            BoardEnd::Open(request) => request,
        };
        loop {
            let (before, after) = match enter_pair(app, &request) {
                Ok(codes) => codes,
                Err(error) => {
                    leave_pair(app);
                    if let Some(session) = app.changeset.as_mut() {
                        session.status = format!("Cannot open the pair: {error:#}");
                    }
                    break;
                }
            };
            let end = run_case_session(terminal, app, &before, &after)?;
            leave_pair(app);
            match route(app, end) {
                Next::Board => break,
                Next::Pair(next) => request = next,
            }
        }
    }
}

#[cfg(test)]
#[path = "changeset_tests.rs"]
mod tests;
