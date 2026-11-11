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

//! Diff line metadata over OSC 1717 (v1), the protocol by which a diff renderer tells the program
//! showing its output which patch line each row stands for. lazygit (0.66 and later) sets
//! `OSC1717=V1` on its renderer and uses the records to stage, navigate and open the line under
//! the cursor in a rendering that no longer looks like a unified diff.
//!
//! **Negotiation.** Only when the environment variable `OSC1717`, a comma-separated set, holds
//! `V1` does headless output change: the handshake `ESC ] 1717 ; 1 ESC \` comes first on stdout,
//! once per run and on every path that prints (a binary or content notice, the "could not diff"
//! line, an error that exits 2 after it), and each row then carries a record. Otherwise the output
//! is byte for byte what it always was. `--mode json` never emits any: it is not a rendering.
//!
//! **Records.** `ESC ] 1717 ; 1 ; type ; new-line ; old-line ; file ESC \`, written as a row's
//! first bytes, before its gutter and its first SGR, so a host that reads "the nearest record at
//! or left of a cell" lands on it. The stacked layout never wraps a row itself (the host does), so
//! one record per row is all §6.3 asks. The side-by-side layout (`headless::side_by_side`) has two
//! halves a row and wraps long lines itself: each half starts with its own record, and each
//! continuation row repeats it (§6.2, §6.3); everything below holds per half.
//!
//! * `f` on every file-header row: `=== before: … ===`, `=== after: … ===`, every line of a
//!   binary or content notice, and the "could not diff" line. The summary label (`No changes`,
//!   ...) above them is decoration and carries none.
//! * `c`/`a`/`d` on content rows, classified in *patch space*: by git's own line diff of the same
//!   pair, never by omnidiff's painting. A line omnidiff matched to a moved line is drawn
//!   unpainted, yet git's patch deletes it and adds it elsewhere, and lazygit stages by its own
//!   `git diff`: a deleted row is looked up by its old line number, an added one by its new line
//!   number, and an identity its patch lacks is "lines not found". So in the before section an old
//!   line git deletes is `d` (with the new position it sits at, §5.3), any other is `c` with the
//!   new line it maps to; in the after section a new line git adds is `a`, any other `c`.
//! * `h` on the rows that head a region of shown lines - the `... N unchanged lines ...` marker
//!   before it and the `@` breadcrumb of its enclosing declaration - carrying the region's first
//!   new line. Only in the *after* section: both sections show the same regions of one diff, and
//!   §5.5 asks for one `h` per hunk. Both rows of such a header carry the same record, as §6.4
//!   recommends for a multi-row header. A region at the top of the file has no header row, and so
//!   no `h`; the marker after the last region heads nothing and carries none.
//! * Nothing on the moved-chunk box's header and footer, the blank line after the summary, or a
//!   row past the file's last line (the empty piece after a final newline).
//!
//! **git's patch.** `git diff --no-index --no-ext-diff --no-color --no-textconv -U0
//! --inter-hunk-context=0 -- OLD NEW`, run in the process's own directory so that the repository's
//! and the user's `diff.algorithm` and `diff.indentHeuristic` apply, as they do to lazygit's
//! `git diff`. Without `GIT_EXTERNAL_DIFF`, `GIT_DIFF_OPTS` and `OSC1717`, so it neither recurses
//! into omnidiff nor changes shape. Only the changed lines are read, so context size does not
//! matter: git's choice of which lines change does not depend on it. It is started before the diff
//! and read after it, so the two overlap. If git is missing, fails, calls the pair binary (it
//! does for UTF-16, which omnidiff decodes), or its patch does not fit the two files' line counts,
//! there are no content records, only the handshake and the `f`s: no record beats a wrong one.
//!
//! **Every changed line is shown.** When records are negotiated, each section's context windows
//! are widened to keep git's changed lines too, beside omnidiff's own: a change omnidiff sees as
//! none (a line moved intact, a whitespace edit under `--minimal`) would otherwise fall inside a
//! collapsed run, where no row can carry it and lazygit could not stage it.
//!
//! **A deletion's new position** (§5.3) is git's arithmetic on lazygit's patch, which has context:
//! the hunk's new start plus the lines above it that are not deleted. In a `-U0` hunk that only
//! deletes, git writes the line *before* the deletion as the new start (`-5,2 +4,0`), which a
//! patch with context would not, so the deletion sits one further on (5) - unless the new file is
//! empty (git's `+0,0`, §5.4) or lazygit asked for no context (`--context 0`), whose patch is
//! `-U0` too.
//!
//! **`file`** is the path git named: argument 1 of the 7-argument form, the new path (argument 8)
//! of the 9-argument rename form, and the after path as given for a direct two-path run - never a
//! title or a decorated name, and not escaped for display. A path with a control character (C0,
//! DEL or C1: an `ESC` or `BEL` would end the OSC early) or that is not UTF-8 gets no records at
//! all, only the handshake.
//!
//! **Whitespace, and the `M` panel.** The records describe git's patch without `-w`, and so does
//! the patch lazygit stages from: its ignore-whitespace toggle only changes what it shows, since a
//! patch missing changes would apply to nothing (`AddCommonDiffArgs` in lazygit 0.66). What omnidiff
//! paints follows the `M` panel's saved options like any headless run (the Full preset when none
//! are saved), and never changes a record: a whitespace-only line is unpainted under Minimal and
//! still stageable as the change git has it as.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// The handshake (§4.4), the very first bytes a negotiated run writes.
pub const HANDSHAKE: &str = "\u{1b}]1717;1\u{1b}\\";

/// Whether `value`, the `OSC1717` environment variable, offers v1: a comma-separated set.
pub fn offered(value: Option<&OsStr>) -> bool {
    value
        .and_then(OsStr::to_str)
        .is_some_and(|set| set.split(',').any(|version| version.trim() == "V1"))
}

/// Whether this process's environment negotiates v1.
pub fn negotiated() -> bool {
    offered(std::env::var_os("OSC1717").as_deref())
}

/// `path` as a record's `file` field, or `None` when it cannot be one: not UTF-8, empty, or
/// holding a control character that would end or corrupt the sequence.
pub fn file_field(path: &Path) -> Option<String> {
    let text = path.to_str()?;
    (!text.is_empty() && !text.contains(char::is_control)).then(|| text.to_string())
}

/// One record. `new` and `old` are 1-based line numbers, `None` for an empty field.
pub fn record(kind: char, new: Option<usize>, old: Option<usize>, file: &str) -> String {
    let field = |line: Option<usize>| line.map_or(String::new(), |line| line.to_string());
    format!(
        "\u{1b}]1717;1;{kind};{};{};{file}\u{1b}\\",
        field(new),
        field(old)
    )
}

/// The `f` record of `file`.
pub fn file_record(file: &str) -> String {
    record('f', None, None, file)
}

/// `text` with `record` in front of each of its lines.
pub fn tag_lines(text: &str, record: &str) -> String {
    let mut out = String::with_capacity(text.len() + record.len() * 4);
    for line in text.split_inclusive('\n') {
        out.push_str(record);
        out.push_str(line);
    }
    out
}

/// How many lines `text` has, as git counts them: a final line without a newline counts, the
/// empty piece after a final newline does not.
pub fn line_count(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.split('\n').count() - usize::from(text.ends_with('\n'))
    }
}

/// One `@@ -a,b +c,d @@` hunk header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
}

impl Hunk {
    /// Parses `@@ -a[,b] +c[,d] @@...`; an omitted count is 1.
    fn parse(line: &str) -> Option<Self> {
        let rest = line.strip_prefix("@@ -")?;
        let (ranges, _) = rest.split_once(" @@")?;
        let (old, new) = ranges.split_once(" +")?;
        let range = |text: &str| -> Option<(usize, usize)> {
            match text.split_once(',') {
                Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
                None => Some((text.parse().ok()?, 1)),
            }
        };
        let ((old_start, old_count), (new_start, new_count)) = (range(old)?, range(new)?);
        Some(Self {
            old_start,
            old_count,
            new_start,
            new_count,
        })
    }

    /// The first old line after this hunk: every old line from it on is shifted by the hunk. A
    /// hunk that deletes nothing names the line it inserts *after*.
    fn old_end(&self) -> usize {
        self.old_start + self.old_count.max(1)
    }
}

/// A deleted old line's place in the new file: the new-side line counter where it stands, and
/// whether its hunk only deletes (see the module doc's "A deletion's new position").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Deletion {
    new_at: usize,
    pure: bool,
}

/// git's line diff of a pair, as the sets lazygit stages by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Patch {
    hunks: Vec<Hunk>,
    deleted: BTreeMap<usize, Deletion>,
    added: BTreeSet<usize>,
    /// Context lines inside a hunk (none with `--inter-hunk-context=0`, read in case).
    context: BTreeMap<usize, usize>,
}

impl Patch {
    /// Parses git's patch text; `None` for a binary pair or a malformed hunk. Each hunk's body is
    /// read for exactly the lines its header counts (§6.5), `\ No newline at end of file` aside.
    pub fn parse(text: &str) -> Option<Self> {
        let mut patch = Self::default();
        let mut lines = text.lines();
        while let Some(line) = lines.next() {
            if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
                return None;
            }
            if !line.starts_with("@@ ") {
                continue;
            }
            let hunk = Hunk::parse(line)?;
            let (mut old, mut new) = (hunk.old_start, hunk.new_start);
            let (mut old_left, mut new_left) = (hunk.old_count, hunk.new_count);
            let pure = hunk.new_count == 0;
            // A count-0 side names the line before the hunk; its lines start one further on.
            if hunk.old_count == 0 {
                old += 1;
            }
            if hunk.new_count == 0 {
                new += 1;
            }
            while old_left > 0 || new_left > 0 {
                let body = lines.next()?;
                match body.as_bytes().first() {
                    Some(b'-') if old_left > 0 => {
                        let new_at = if pure { hunk.new_start } else { new };
                        patch.deleted.insert(old, Deletion { new_at, pure });
                        old += 1;
                        old_left -= 1;
                    }
                    Some(b'+') if new_left > 0 => {
                        patch.added.insert(new);
                        new += 1;
                        new_left -= 1;
                    }
                    Some(b' ') if old_left > 0 && new_left > 0 => {
                        patch.context.insert(old, new);
                        old += 1;
                        new += 1;
                        old_left -= 1;
                        new_left -= 1;
                    }
                    Some(b'\\') => {}
                    _ => return None,
                }
            }
            patch.hunks.push(hunk);
        }
        Some(patch)
    }

    /// Starts git on the pair; [`PendingPatch::finish`] reads its answer.
    pub fn start(old: &Path, new: &Path) -> PendingPatch {
        let child = Command::new("git")
            .args([
                "diff",
                "--no-index",
                "--no-ext-diff",
                "--no-color",
                "--no-textconv",
                "-U0",
                "--inter-hunk-context=0",
                "--",
            ])
            .arg(old)
            .arg(new)
            .env_remove("GIT_EXTERNAL_DIFF")
            .env_remove("GIT_DIFF_OPTS")
            .env_remove("OSC1717")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok();
        PendingPatch(child)
    }

    /// Whether every line the patch names exists and the unchanged lines on the two sides agree
    /// in number: a patch of other contents than the ones rendered must not label them.
    pub fn fits(&self, old_lines: usize, new_lines: usize) -> bool {
        self.deleted
            .keys()
            .all(|&line| (1..=old_lines).contains(&line))
            && self
                .added
                .iter()
                .all(|&line| (1..=new_lines).contains(&line))
            && old_lines - self.deleted.len() == new_lines - self.added.len()
    }

    /// Whether git deletes old line `line`.
    pub fn is_deleted(&self, line: usize) -> bool {
        self.deleted.contains_key(&line)
    }

    /// Whether git adds new line `line`.
    pub fn is_added(&self, line: usize) -> bool {
        self.added.contains(&line)
    }

    /// The new position deleted old line `line` sits at (§5.3), for a host whose patch has
    /// `context` lines of context and a new file of `new_lines` lines.
    pub fn deletion_position(
        &self,
        line: usize,
        context: usize,
        new_lines: usize,
    ) -> Option<usize> {
        let deletion = self.deleted.get(&line)?;
        Some(if deletion.pure && context > 0 && new_lines > 0 {
            deletion.new_at + 1
        } else {
            deletion.new_at
        })
    }

    /// The new line that unchanged old line `line` is.
    pub fn new_line_of(&self, line: usize) -> usize {
        if let Some(&new) = self.context.get(&line) {
            return new;
        }
        let shift: isize = self
            .hunks
            .iter()
            .take_while(|hunk| hunk.old_end() <= line)
            .map(|hunk| hunk.new_count as isize - hunk.old_count as isize)
            .sum();
        line.saturating_add_signed(shift)
    }
}

/// A `git diff` running beside the diff (see [`Patch::start`]).
pub struct PendingPatch(Option<Child>);

impl PendingPatch {
    /// git's patch, or `None` when git is missing or failed (exit status above 1).
    pub fn finish(self) -> Option<Patch> {
        let output = self.0?.wait_with_output().ok()?;
        if !matches!(output.status.code(), Some(0 | 1)) {
            return None;
        }
        Patch::parse(&String::from_utf8_lossy(&output.stdout))
    }
}

/// Everything headless needs to tag a text diff's rows.
pub struct Annotation {
    /// The `file` field.
    pub file: String,
    /// git's patch of the pair, `None` when there is none to trust: `f` records only.
    pub patch: Option<Patch>,
    /// lazygit's `--context`, for [`Patch::deletion_position`].
    pub context: usize,
}

/// Which section of the output a row is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Before,
    After,
}

/// [`Annotation`] for one section, with the two files' line counts.
pub struct SectionRecords<'a> {
    file: &'a str,
    patch: Option<&'a Patch>,
    context: usize,
    section: Section,
    old_lines: usize,
    new_lines: usize,
}

impl Annotation {
    /// git's patch, if it fits a pair of `old_lines` and `new_lines` lines.
    pub fn fitting_patch(&self, old_lines: usize, new_lines: usize) -> Option<&Patch> {
        self.patch
            .as_ref()
            .filter(|patch| patch.fits(old_lines, new_lines))
    }

    /// The records of `section`, for a pair of `old_lines` and `new_lines` lines. A patch that
    /// does not fit the counts is dropped.
    pub fn section(
        &self,
        section: Section,
        old_lines: usize,
        new_lines: usize,
    ) -> SectionRecords<'_> {
        SectionRecords {
            file: &self.file,
            patch: self.fitting_patch(old_lines, new_lines),
            context: self.context,
            section,
            old_lines,
            new_lines,
        }
    }
}

impl SectionRecords<'_> {
    fn lines(&self) -> usize {
        match self.section {
            Section::Before => self.old_lines,
            Section::After => self.new_lines,
        }
    }

    /// Per row of a section showing `rows` rows: whether git changes the row's line.
    pub fn changed_rows(&self, rows: usize) -> Vec<bool> {
        let Some(patch) = self.patch else {
            return Vec::new();
        };
        (1..=rows)
            .map(|line| match self.section {
                Section::Before => patch.is_deleted(line),
                Section::After => patch.is_added(line),
            })
            .collect()
    }

    /// The record of content row `row` (0-based).
    pub fn content(&self, row: usize) -> Option<String> {
        let patch = self.patch?;
        let line = row + 1;
        if line > self.lines() {
            return None;
        }
        Some(match self.section {
            Section::Before => match patch.deletion_position(line, self.context, self.new_lines) {
                Some(new) => record('d', Some(new), Some(line), self.file),
                None => record('c', Some(patch.new_line_of(line)), None, self.file),
            },
            Section::After if patch.is_added(line) => record('a', Some(line), None, self.file),
            Section::After => record('c', Some(line), None, self.file),
        })
    }

    /// The record of a row heading the region whose first row is `row` (0-based): an `h` in the
    /// after section, nothing in the before section.
    pub fn region_header(&self, row: usize) -> Option<String> {
        self.patch?;
        (self.section == Section::After && row < self.new_lines)
            .then(|| record('h', Some(row + 1), None, self.file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(text: &str) -> Patch {
        Patch::parse(text).expect("parses")
    }

    #[test]
    fn the_handshake_is_negotiated_only_when_v1_is_in_the_set() {
        for (value, expected) in [
            (None, false),
            (Some(""), false),
            (Some("V2"), false),
            (Some("v1"), false),
            (Some("V1"), true),
            (Some("V2,V1"), true),
            (Some("V1, V2"), true),
        ] {
            assert_eq!(offered(value.map(OsStr::new)), expected, "{value:?}");
        }
    }

    #[test]
    fn a_record_has_five_fields_and_ends_with_st() {
        assert_eq!(
            record('d', Some(11), Some(9), "src/a;b.rs"),
            "\u{1b}]1717;1;d;11;9;src/a;b.rs\u{1b}\\"
        );
        assert_eq!(file_record("x"), "\u{1b}]1717;1;f;;;x\u{1b}\\");
    }

    #[test]
    fn a_path_with_a_control_character_is_no_file_field() {
        assert_eq!(
            file_field(Path::new("src/a b.rs")).as_deref(),
            Some("src/a b.rs")
        );
        for hostile in ["a\u{1b}b", "a\u{7}b", "a\nb", "a\u{9c}b", "a\u{7f}b", ""] {
            assert_eq!(file_field(Path::new(hostile)), None, "{hostile:?}");
        }
    }

    #[test]
    fn line_counts_follow_git() {
        assert_eq!(line_count(""), 0);
        assert_eq!(line_count("a"), 1);
        assert_eq!(line_count("a\n"), 1);
        assert_eq!(line_count("a\nb"), 2);
        assert_eq!(line_count("\n\n"), 2);
    }

    #[test]
    fn a_pure_addition_adds_and_shifts_what_follows() {
        let patch = patch("@@ -1,0 +2,2 @@\n+x\n+y\n");
        assert!(patch.is_added(2) && patch.is_added(3) && !patch.is_added(1));
        assert_eq!(patch.new_line_of(1), 1);
        assert_eq!(patch.new_line_of(2), 4);
        assert!(patch.fits(3, 5));
    }

    #[test]
    fn a_pure_deletion_sits_after_the_line_git_names() {
        // Old lines 4 and 5 of 6 deleted.
        let patch = patch("@@ -4,2 +3,0 @@\n-d\n-e\n");
        assert!(patch.is_deleted(4) && patch.is_deleted(5));
        assert_eq!(patch.deletion_position(4, 3, 4), Some(4));
        assert_eq!(
            patch.deletion_position(5, 3, 4),
            Some(4),
            "consecutive share"
        );
        assert_eq!(
            patch.deletion_position(4, 0, 4),
            Some(3),
            "a -U0 host's arithmetic"
        );
        assert_eq!(patch.new_line_of(3), 3);
        assert_eq!(patch.new_line_of(6), 4);
        assert!(patch.fits(6, 4));
        assert!(!patch.fits(6, 5));
    }

    #[test]
    fn a_change_puts_its_deletions_at_the_new_start() {
        let patch = patch("@@ -2,2 +2,3 @@\n-b\n-c\n+B\n+C\n+D\n");
        assert_eq!(patch.deletion_position(2, 3, 9), Some(2));
        assert_eq!(patch.deletion_position(3, 3, 9), Some(2));
        assert!(patch.is_added(4));
        assert_eq!(patch.new_line_of(4), 5);
    }

    #[test]
    fn a_whole_file_addition_and_deletion() {
        let added = patch("@@ -0,0 +1,3 @@\n+a\n+b\n+c\n");
        assert!((1..=3).all(|line| added.is_added(line)));
        assert!(added.fits(0, 3));

        let deleted = patch("@@ -1,3 +0,0 @@\n-a\n-b\n-c\n");
        for line in 1..=3 {
            assert_eq!(deleted.deletion_position(line, 3, 0), Some(0), "§5.4");
        }
        assert!(deleted.fits(3, 0));
    }

    #[test]
    fn omitted_counts_are_one() {
        let patch = patch("@@ -3 +3 @@ fn f() {\n-a\n+b\n");
        assert!(patch.is_deleted(3) && patch.is_added(3));
        assert_eq!(patch.deletion_position(3, 3, 5), Some(3));
        assert_eq!(patch.new_line_of(4), 4);
    }

    #[test]
    fn no_newline_at_end_of_file_is_not_a_line() {
        let text = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -2 +2 @@\n-b\n\\ No newline at end of file\n+b\n";
        let patch = patch(text);
        assert!(patch.is_deleted(2) && patch.is_added(2));
        assert!(patch.fits(2, 2));
    }

    #[test]
    fn a_hunk_is_read_for_exactly_its_counted_lines() {
        // The `-x` after the hunk is not part of it.
        assert_eq!(
            Patch::parse("@@ -1 +1 @@\n-a\n+b\n-x\n")
                .unwrap()
                .deleted
                .len(),
            1
        );
        assert_eq!(
            Patch::parse("@@ -1,2 +1 @@\n-a\n+b\n"),
            None,
            "body too short"
        );
    }

    #[test]
    fn a_binary_pair_has_no_patch() {
        assert_eq!(
            Patch::parse("diff --git a/x b/x\nBinary files a/x and b/x differ\n"),
            None
        );
    }

    #[test]
    fn context_lines_inside_a_hunk_map_across() {
        let patch = patch("@@ -1,3 +1,3 @@\n-a\n+A\n b\n-c\n+C\n");
        assert_eq!(patch.new_line_of(2), 2);
        assert!(patch.is_deleted(3) && patch.is_added(3));
    }

    #[test]
    fn git_reports_the_changed_lines_of_two_files() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = (dir.path().join("old"), dir.path().join("new"));
        std::fs::write(&old, "a\nb\nc\n").unwrap();
        std::fs::write(&new, "a\nc\nd\n").unwrap();
        let Some(patch) = Patch::start(&old, &new).finish() else {
            eprintln!("no git on PATH; skipped");
            return;
        };
        assert!(patch.is_deleted(2) && patch.is_added(3));
        assert_eq!(patch.new_line_of(3), 2);
        assert!(patch.fits(3, 3));
    }

    /// One record as the host splits it: at most five fields, the file last.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Parsed {
        kind: char,
        new: Option<usize>,
        old: Option<usize>,
        file: String,
    }

    /// Every row of `output` with its records, checking that each record is a row's first bytes
    /// (or follows another) and parses as five fields of v1.
    fn rows_with_records(output: &str) -> Vec<(Vec<Parsed>, String)> {
        const OPEN: &str = "\u{1b}]1717;";
        output
            .lines()
            .map(|mut row| {
                let mut records = Vec::new();
                while let Some(rest) = row.strip_prefix(OPEN) {
                    let end = rest.find("\u{1b}\\").expect("terminated by ST");
                    let fields: Vec<&str> = rest[..end].splitn(5, ';').collect();
                    assert_eq!(fields.len(), 5, "five fields: {row:?}");
                    assert_eq!(fields[0], "1");
                    let number = |field: &str| (!field.is_empty()).then(|| field.parse().unwrap());
                    records.push(Parsed {
                        kind: fields[1].chars().next().unwrap(),
                        new: number(fields[2]),
                        old: number(fields[3]),
                        file: fields[4].to_string(),
                    });
                    row = &rest[end + 2..];
                }
                assert!(!row.contains(OPEN), "a record inside a row: {row:?}");
                (records, row.to_string())
            })
            .collect()
    }

    /// lazygit's own arithmetic on a patch with `context` lines of context: each deleted old
    /// line's new position, and the added new lines.
    fn contexted_patch(
        old: &Path,
        new: &Path,
        context: usize,
    ) -> (BTreeMap<usize, usize>, BTreeSet<usize>) {
        let output = Command::new("git")
            .args([
                "diff",
                "--no-index",
                "--no-ext-diff",
                "--no-color",
                "--no-textconv",
            ])
            .arg(format!("-U{context}"))
            .arg("--")
            .args([old, new])
            .env_remove("GIT_EXTERNAL_DIFF")
            .output()
            .expect("git runs");
        let (mut deleted, mut added) = (BTreeMap::new(), BTreeSet::new());
        let (mut old_line, mut new_line) = (0, 0);
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(hunk) = line.starts_with("@@ ").then(|| Hunk::parse(line)).flatten() {
                // A whole-file deletion's `+0,0` puts its lines at new position 0 (§5.4).
                (old_line, new_line) = (hunk.old_start.max(1), hunk.new_start);
                continue;
            }
            match line.as_bytes().first() {
                Some(b'-') if !line.starts_with("--- ") || old_line > 0 => {
                    deleted.insert(old_line, new_line);
                    old_line += 1;
                }
                Some(b'+') if !line.starts_with("+++ ") || new_line > 0 => {
                    added.insert(new_line);
                    new_line += 1;
                }
                Some(b' ') => {
                    old_line += 1;
                    new_line += 1;
                }
                _ => {}
            }
        }
        (deleted, added)
    }

    /// The pairs the record test runs over: a sample of the fixtures, one in three of
    /// `UNIT_TEST_FIXTURES`, plus the edge cases git's patch has a special form for.
    fn sample_pairs() -> Vec<(String, String, String, String)> {
        let mut pairs = Vec::new();
        for name in crate::test::helper::UNIT_TEST_FIXTURES.iter().step_by(3) {
            let dir = crate::test::helper::diffs_case_dir(name).expect("fixture");
            let side = |prefix: &str| {
                let entry = std::fs::read_dir(&dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                    .find(|file| file.starts_with(prefix) && file.ends_with(".test"))
                    .expect("side");
                let extension = entry
                    .trim_start_matches(prefix)
                    .trim_end_matches(".test")
                    .to_string();
                (
                    std::fs::read_to_string(dir.join(&entry)).unwrap(),
                    extension,
                )
            };
            let ((before, extension), (after, _)) = (side("before"), side("after"));
            pairs.push((name.to_string(), extension, before, after));
        }
        let python = crate::test::helper::diffs_case_dir("python-refactoring").unwrap();
        pairs.push((
            "python-refactoring".into(),
            ".py".into(),
            std::fs::read_to_string(python.join("before.py.test")).unwrap(),
            std::fs::read_to_string(python.join("after.py.test")).unwrap(),
        ));
        let lines = |range: std::ops::Range<usize>| -> String {
            range.map(|i| format!("line {i}\n")).collect()
        };
        for (name, before, after) in [
            ("added", String::new(), lines(0..5)),
            ("deleted", lines(0..5), String::new()),
            ("no-newline", "a\nb".to_string(), "a\nb\n".to_string()),
            (
                "crlf",
                "a\r\nb\r\nc\r\n".to_string(),
                "a\r\nB\r\nc\r\n".to_string(),
            ),
            ("head-deleted", lines(0..30), lines(2..30)),
            ("tail-deleted", lines(0..30), lines(0..27)),
            (
                "far-apart",
                lines(0..40),
                format!("{}x\n{}", lines(0..20), lines(21..39)),
            ),
            ("identical", lines(0..5), lines(0..5)),
        ] {
            pairs.push((name.into(), ".txt".into(), before, after));
        }
        pairs
    }

    /// Over a sample of the fixtures, the records name exactly git's patch: every line it deletes
    /// in one `d` (by old line, at lazygit's new position) and every line it adds in one `a`, no
    /// other line as either, one `f` per section, `h` only after, and every field in range.
    #[test]
    fn records_name_every_line_of_gits_patch_once_and_nothing_else() {
        use crate::tui::headless::{Pair, render_annotated_text_diff, render_text_diff};
        const CONTEXT: usize = 3;
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("no git on PATH; skipped");
            return;
        }
        let file = "dir/the file;1.txt";
        for (name, extension, before, after) in sample_pairs() {
            let dir = tempfile::tempdir().unwrap();
            let (old, new) = (
                dir.path().join(format!("before{extension}")),
                dir.path().join(format!("after{extension}")),
            );
            std::fs::write(&old, &before).unwrap();
            std::fs::write(&new, &after).unwrap();
            let patch = Patch::start(&old, &new).finish();
            assert!(patch.is_some(), "{name}: git's patch");
            let pair = Pair::read(&old, &new).unwrap();
            let (data, _) = pair.diff(crate::diff::text::RenderOptions::FULL).unwrap();
            let annotation = Annotation {
                file: file.to_string(),
                patch,
                context: CONTEXT,
            };
            let output = render_annotated_text_diff(&data, false, CONTEXT, Some(&annotation));
            let (expected_deleted, expected_added) = contexted_patch(&old, &new, CONTEXT);

            let rows = rows_with_records(&output);
            let file_rows: Vec<usize> = rows
                .iter()
                .enumerate()
                .filter(|(_, (records, _))| records.iter().any(|r| r.kind == 'f'))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(file_rows.len(), 2, "{name}: one f per section\n{output}");
            assert!(rows[file_rows[0]].1.starts_with("=== before: "), "{name}");
            assert!(rows[file_rows[1]].1.starts_with("=== after: "), "{name}");

            let (mut deleted, mut added) = (BTreeMap::new(), BTreeSet::new());
            let (old_lines, new_lines) = (line_count(&before), line_count(&after));
            for (index, (records, text)) in rows.iter().enumerate() {
                let after_section = index > file_rows[1];
                for record in records {
                    assert_eq!(record.file, file, "{name}");
                    match record.kind {
                        'f' => assert_eq!((record.new, record.old), (None, None), "{name}"),
                        'd' => {
                            assert!(!after_section, "{name}: d after: {text}");
                            let old = record.old.expect("d has an old line");
                            assert!(
                                deleted.insert(old, record.new.unwrap()).is_none(),
                                "{name}: old line {old} in two records"
                            );
                        }
                        'a' => {
                            assert!(after_section, "{name}: a before: {text}");
                            assert_eq!(record.old, None);
                            let new = record.new.unwrap();
                            assert!(added.insert(new), "{name}: new line {new} in two records");
                        }
                        'c' => {
                            assert_eq!(record.old, None);
                            let new = record.new.unwrap();
                            assert!((1..=new_lines).contains(&new), "{name}: c {new}");
                            assert!(!expected_added.contains(&new), "{name}: c on an added line");
                        }
                        'h' => {
                            assert!(after_section, "{name}: h before: {text}");
                            assert!((1..=new_lines).contains(&record.new.unwrap()), "{name}");
                        }
                        other => panic!("{name}: type {other}"),
                    }
                }
                if index > file_rows[0] && index < file_rows[1] {
                    assert!(
                        records.iter().all(|r| matches!(r.kind, 'c' | 'd')),
                        "{name}"
                    );
                }
            }
            assert_eq!(deleted, expected_deleted, "{name}: deletions\n{output}");
            assert_eq!(added, expected_added, "{name}: additions\n{output}");
            assert!(deleted.keys().all(|&old| old <= old_lines));

            // Without a patch, only the `f`s are added; without an annotation, nothing.
            let bare = Annotation {
                file: file.to_string(),
                patch: None,
                context: CONTEXT,
            };
            let plain = render_text_diff(&data, false, CONTEXT);
            assert!(!plain.contains("\u{1b}]1717"), "{name}");
            let stripped = render_annotated_text_diff(&data, false, CONTEXT, Some(&bare))
                .replace(&file_record(file), "");
            assert_eq!(stripped, plain, "{name}");
        }
    }

    /// Every record of each row with whether it is the row's first bytes (a left half, or a whole
    /// row) or follows the divider (a right half).
    fn records_of_side_by_side_rows(output: &str) -> Vec<Vec<(bool, Parsed)>> {
        const OPEN: &str = "\u{1b}]1717;";
        output
            .lines()
            .map(|row| {
                let mut records = Vec::new();
                let mut from = 0;
                while let Some(at) = row[from..].find(OPEN).map(|at| from + at) {
                    let rest = &row[at + OPEN.len()..];
                    let end = rest.find("\u{1b}\\").expect("terminated by ST");
                    let fields: Vec<&str> = rest[..end].splitn(5, ';').collect();
                    assert_eq!(fields.len(), 5, "five fields: {row:?}");
                    assert_eq!(fields[0], "1");
                    let number = |field: &str| (!field.is_empty()).then(|| field.parse().unwrap());
                    records.push((
                        at == 0,
                        Parsed {
                            kind: fields[1].chars().next().unwrap(),
                            new: number(fields[2]),
                            old: number(fields[3]),
                            file: fields[4].to_string(),
                        },
                    ));
                    from = at + OPEN.len() + end + 2;
                }
                records
            })
            .collect()
    }

    /// The side-by-side layout over the same sample (§6.2, §6.3): every line git deletes on the
    /// left and every line it adds on the right, each on one run of rows (its primary row and its
    /// wrapped continuations, each repeating the record) and nowhere else; `c` on either half;
    /// one `f`, on the header row; `h` only at the start of a row. Wide and narrow, so lines wrap.
    #[test]
    fn side_by_side_records_name_every_line_of_gits_patch_on_one_run_of_rows() {
        use crate::tui::headless::{Layout, LayoutRequest, Pair, render_laid_out_text_diff};
        const CONTEXT: usize = 3;
        if Command::new("git").arg("--version").output().is_err() {
            eprintln!("no git on PATH; skipped");
            return;
        }
        let file = "dir/the file;1.txt";
        for (name, extension, before, after) in sample_pairs() {
            let dir = tempfile::tempdir().unwrap();
            let (old, new) = (
                dir.path().join(format!("before{extension}")),
                dir.path().join(format!("after{extension}")),
            );
            std::fs::write(&old, &before).unwrap();
            std::fs::write(&new, &after).unwrap();
            let patch = Patch::start(&old, &new).finish();
            let pair = Pair::read(&old, &new).unwrap();
            let (data, _) = pair.diff(crate::diff::text::RenderOptions::FULL).unwrap();
            let annotation = Annotation {
                file: file.to_string(),
                patch,
                context: CONTEXT,
            };
            let (expected_deleted, expected_added) = contexted_patch(&old, &new, CONTEXT);
            let new_lines = line_count(&after);
            for width in [160, 72] {
                let output = render_laid_out_text_diff(
                    &data,
                    false,
                    CONTEXT,
                    Some(&annotation),
                    LayoutRequest {
                        layout: Layout::SideBySide,
                        width: Some(width),
                    },
                    None,
                );
                let rows: Vec<&str> = output.lines().collect();
                let records = records_of_side_by_side_rows(&output);
                let header = rows
                    .iter()
                    .position(|row| row.contains('┬'))
                    .unwrap_or_else(|| panic!("{name} {width}: not side by side\n{output}"));
                // Each line's rows, by its record.
                let (mut deleted, mut added) = (
                    BTreeMap::<(usize, usize), Vec<usize>>::new(),
                    BTreeMap::<usize, Vec<usize>>::new(),
                );
                let mut files = 0;
                for (index, row_records) in records.iter().enumerate() {
                    let (starts, follows): (Vec<_>, Vec<_>) =
                        row_records.iter().partition(|(at_start, _)| *at_start);
                    assert!(starts.len() <= 1 && follows.len() <= 1, "{name}: {rows:?}");
                    for (at_start, record) in row_records {
                        assert_eq!(record.file, file, "{name}");
                        match (record.kind, at_start) {
                            ('f', true) => {
                                assert_eq!(index, header, "{name}");
                                files += 1;
                            }
                            ('h', true) => {
                                assert!((1..=new_lines).contains(&record.new.unwrap()), "{name}")
                            }
                            ('d', true) => deleted
                                .entry((record.old.unwrap(), record.new.unwrap()))
                                .or_default()
                                .push(index),
                            ('a', false) => {
                                assert_eq!(record.old, None, "{name}");
                                added.entry(record.new.unwrap()).or_default().push(index);
                            }
                            ('c', _) => {
                                let new = record.new.unwrap();
                                assert!((1..=new_lines).contains(&new), "{name}: c {new}");
                                assert!(!expected_added.contains(&new), "{name}: c on an add");
                            }
                            (kind, at_start) => panic!(
                                "{name}: {kind} {}: {:?}",
                                if *at_start {
                                    "first"
                                } else {
                                    "after the divider"
                                },
                                rows[index]
                            ),
                        }
                    }
                }
                assert_eq!(files, 1, "{name}: one f\n{output}");
                let one_run = |indices: &Vec<usize>| indices.windows(2).all(|w| w[1] == w[0] + 1);
                assert!(
                    deleted.values().chain(added.values()).all(one_run),
                    "{name} {width}: a line's rows are not one run\n{output}"
                );
                let deleted: BTreeMap<usize, usize> =
                    deleted.keys().map(|&(old, new)| (old, new)).collect();
                assert_eq!(deleted, expected_deleted, "{name} {width}\n{output}");
                let added: BTreeSet<usize> = added.keys().copied().collect();
                assert_eq!(added, expected_added, "{name} {width}\n{output}");
            }
        }
    }

    #[test]
    fn tag_lines_prefixes_every_line() {
        assert_eq!(tag_lines("a\nb\n", "R"), "Ra\nRb\n");
        assert_eq!(tag_lines("a", "R"), "Ra");
    }
}
