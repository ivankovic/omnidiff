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

//! Content made of named members, diffed member by member: an archive's files, a font's glyphs,
//! a PDF's pages, a catalog's messages, a cursor theme's cursors. The model diffoscope uses for
//! everything.
//!
//! Each side decodes to a [`Container`]: its members in their own order, each with a key and a
//! hash of what it shows. Members **match by key** (a path, a code point, a page number), and a
//! matched pair whose hashes are equal is the same; nothing else of it is decoded. Only a changed
//! member is opened ([`Container::open`]) and diffed by what it holds: a picture by the picture
//! engine, text by lines, bytes by whatever they turn out to be ([`super::diff`], so a picture in
//! a zip is a picture). A font with two edited glyphs among six hundred renders two.
//!
//! A member that only one side has is added or removed. Renames are not followed: a moved file
//! is one removed and one added.
//!
//! **Except in a sequence** ([`Container::sequence`]: a PDF's pages), whose keys are positions.
//! When the two sides have as many members, they still match by key. When they have not, a member
//! was inserted or removed, and every later key would name another member: the members then
//! align by hash, the way an animation's frames do (`picture::align`, a longest common subsequence
//! bounded by its table size), and the members left between matches pair in order, the rest added
//! or removed. A page inserted in the middle is one added page; the pages after it are unchanged.
//!
//! Changed members are opened and compared on every core, each worker taking the next member as
//! it finishes one (a few large members do not leave the other cores idle) and opening members
//! through one [`Opener`] per side, which may keep what it shares between them (a PDF parsed
//! once per worker). A member whose decoder panics, or that would take the diff past its expansion budget
//! ([`super::MAX_DIFF_EXPANDED_BYTES`]), does not open: it is changed, with no detail.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use serde::Serialize;

use super::{Budget, ContentDiff, Family, Level, Tag, Verdict, contained};
use crate::diff::picture::{self, Frames, PictureDiff, PictureInfo, REPLACED_SHARE};

/// How deep containers nest before the rest is compared as bytes: a zip in a zip in a zip is
/// real, and an archive that contains itself is an attack.
pub const MAX_DEPTH: usize = 3;

/// The most bytes a side may expand to, in total: past it the side does not decode, and the pair
/// is reported as binary. A decompression bomb is a few kilobytes on disk. Every byte expanded is
/// also charged to the whole diff's budget ([`super::MAX_DIFF_EXPANDED_BYTES`]), which nested
/// containers and parallel workers share.
pub const MAX_EXPANDED_BYTES: u64 = 512 * 1024 * 1024;

/// One member of a container, as listed: what it is called, and a hash of what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub key: String,
    pub hash: u64,
}

/// A member's content, once opened.
pub enum MemberContent {
    /// Bytes to be diffed by what they are: an archive's file.
    Bytes(Vec<u8>),
    /// A picture already decoded: a glyph, a page, a cursor.
    Picture(Box<(PictureInfo, Frames)>),
    /// Text: a message, a font's names.
    Text(String),
}

/// Opens members of one container one after another, on one thread (see [`Container::opener`]).
pub type Opener<'a> = Box<dyn FnMut(usize) -> Result<MemberContent> + 'a>;

/// One side of a container pair, decoded.
pub trait Container: Send + Sync {
    fn info(&self) -> &ContainerInfo;
    /// Every member, in the container's own order.
    fn members(&self) -> &[Member];
    /// Member `index`'s content.
    fn open(&self, index: usize) -> Result<MemberContent>;
    /// A way to open many members on one thread, sharing what opening them can share; by default
    /// [`Container::open`] each.
    fn opener(&self) -> Opener<'_> {
        Box::new(move |index| self.open(index))
    }
    /// True if the members are a sequence keyed by position (a PDF's pages), which aligns by hash
    /// when a member was inserted or removed (see the module doc).
    fn sequence(&self) -> bool {
        false
    }
}

/// One side of a container pair, as the file says it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContainerInfo {
    /// The format the bytes are in (`"ZIP"`, `"TAR.GZ"`).
    pub format: String,
    /// The file's size in bytes.
    pub bytes: usize,
    /// How many members it has.
    pub members: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Same,
    Changed,
    Added,
    Removed,
}

/// What changed in a member both sides have.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemberDetail {
    /// A picture member, or bytes that are a picture.
    Picture(PictureDiff),
    /// Bytes that are content of another kind: an archive in an archive.
    Content { content: Box<ContentDiff> },
    /// Text, compared by lines: how many each side has, and how many only it has.
    Text {
        before_lines: usize,
        after_lines: usize,
        removed: usize,
        added: usize,
        /// When lines changed but only in their whitespace, which.
        #[serde(skip_serializing_if = "Option::is_none")]
        whitespace: Option<Whitespace>,
        /// True if the two sides share at least half their words, counted with repeats: text
        /// rewritten rather than replaced.
        words_kept: bool,
    },
    /// Bytes nothing more is known about.
    Binary {
        before_bytes: usize,
        after_bytes: usize,
    },
}

/// Which whitespace alone differs between two texts whose lines do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Whitespace {
    /// At the ends of lines, or blank lines at the end.
    Trailing,
    /// Anywhere: reflowed or re-indented.
    Anywhere,
}

impl MemberDetail {
    /// The engine's verdict on the member: the picture's or nested content's own; and for text,
    /// invisible when no line changed (only line endings or the encoding did), imperceptible for
    /// trailing whitespace, artifacts for other whitespace, edited up to [`REPLACED_SHARE`] of its
    /// lines, and past it redrawn if the words stayed and replaced if not. Bytes are edited.
    pub fn verdict(&self) -> Verdict {
        let level = match self {
            MemberDetail::Picture(diff) => return diff.verdict(),
            MemberDetail::Content { content } => return content.verdict(),
            MemberDetail::Text {
                before_lines,
                after_lines,
                removed,
                added,
                whitespace,
                words_kept,
            } => {
                let changed = removed + added;
                if changed == 0 {
                    Level::Invisible
                } else if let Some(whitespace) = whitespace {
                    match whitespace {
                        Whitespace::Trailing => Level::Imperceptible,
                        Whitespace::Anywhere => Level::Artifacts,
                    }
                } else if changed as f64 <= REPLACED_SHARE * (before_lines + after_lines) as f64 {
                    Level::Edited
                } else if *words_kept {
                    Level::Redrawn
                } else {
                    Level::Replaced
                }
            }
            MemberDetail::Binary { .. } => Level::Edited,
        };
        Verdict::new(level)
    }
}

/// One member of a container diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemberDiff {
    pub key: String,
    pub status: MemberStatus,
    /// For a changed member: what changed, if it opened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<MemberDetail>,
}

impl MemberDiff {
    /// The engine's verdict on a changed member; `None` for one that is the same, added or
    /// removed, which is a fact rather than a judgement.
    pub fn verdict(&self) -> Option<Verdict> {
        match (self.status, &self.detail) {
            (MemberStatus::Changed, Some(detail)) => Some(detail.verdict()),
            (MemberStatus::Changed, None) => Some(Verdict::new(Level::Edited)),
            _ => None,
        }
    }
}

/// What changed between two containers of one family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContainerDiff {
    pub family: Family,
    pub before: Option<ContainerInfo>,
    pub after: Option<ContainerInfo>,
    /// Members both sides have unchanged; left out of `members`, which can run to thousands.
    pub unchanged: usize,
    /// Every member that changed, was added or was removed, in [`align`]'s order.
    pub members: Vec<MemberDiff>,
}

impl ContainerDiff {
    /// The engine's verdict on the pair: identical (invisible) when no member changed, was added
    /// or was removed (a recompressed archive, a re-hinted font), and changed (edited) otherwise,
    /// tagged with which of those happened. What happened inside a member is that member's
    /// verdict. An added or deleted file is edited.
    pub fn verdict(&self) -> Verdict {
        if self.before.is_none() || self.after.is_none() {
            return Verdict::new(Level::Edited);
        }
        let mut verdict = Verdict::new(if self.members.is_empty() {
            Level::Invisible
        } else {
            Level::Edited
        });
        for (status, tag) in [
            (MemberStatus::Added, Tag::AddedFiles),
            (MemberStatus::Removed, Tag::RemovedFiles),
            (MemberStatus::Changed, Tag::ChangedFiles),
        ] {
            if self.members.iter().any(|member| member.status == status) {
                verdict = verdict.with(tag);
            }
        }
        verdict
    }

    /// The member called `key`, if it changed, was added or was removed.
    pub fn member(&self, key: &str) -> Option<&MemberDiff> {
        self.members.iter().find(|member| member.key == key)
    }
}

/// One row of an aligned pair of containers: a member index on each side that has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aligned {
    pub before: Option<usize>,
    pub after: Option<usize>,
    pub status: MemberStatus,
}

/// Both sides' members, matched by key, in one order: after's, with each member only before has
/// placed after the last member before it that both have (or first, if none is).
pub fn align(before: Option<&dyn Container>, after: Option<&dyn Container>) -> Vec<Aligned> {
    if let (Some(before), Some(after)) = (before, after)
        && before.sequence()
        && after.sequence()
        && before.members().len() != after.members().len()
    {
        return align_sequence(before.members(), after.members());
    }
    let empty: &[Member] = &[];
    let before_members = before.map_or(empty, |container| container.members());
    let after_members = after.map_or(empty, |container| container.members());
    let after_index: HashMap<&str, usize> = after_members
        .iter()
        .enumerate()
        .map(|(index, member)| (member.key.as_str(), index))
        .collect();
    let before_index: HashMap<&str, usize> = before_members
        .iter()
        .enumerate()
        .map(|(index, member)| (member.key.as_str(), index))
        .collect();

    // Before's members only it has, grouped under the after member they follow.
    let mut removed_after: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
    let mut anchor = None;
    for (index, member) in before_members.iter().enumerate() {
        match after_index.get(member.key.as_str()) {
            Some(&after) => anchor = Some(after),
            None => removed_after.entry(anchor).or_default().push(index),
        }
    }
    let removed = |rows: &mut Vec<Aligned>, anchor: Option<usize>| {
        for &index in removed_after.get(&anchor).into_iter().flatten() {
            rows.push(Aligned {
                before: Some(index),
                after: None,
                status: MemberStatus::Removed,
            });
        }
    };

    let mut rows = Vec::with_capacity(before_members.len().max(after_members.len()));
    removed(&mut rows, None);
    for (index, member) in after_members.iter().enumerate() {
        let row = match before_index.get(member.key.as_str()) {
            Some(&before) => Aligned {
                before: Some(before),
                after: Some(index),
                status: if before_members[before].hash == member.hash {
                    MemberStatus::Same
                } else {
                    MemberStatus::Changed
                },
            },
            None => Aligned {
                before: None,
                after: Some(index),
                status: MemberStatus::Added,
            },
        };
        rows.push(row);
        if row.before.is_some() {
            removed(&mut rows, Some(index));
        }
    }
    rows
}

/// Two sequences of members aligned by hash: matched members, then in each gap between matches
/// the members paired in order, and the rest of the longer side added or removed.
fn align_sequence(before: &[Member], after: &[Member]) -> Vec<Aligned> {
    let hashes = |members: &[Member]| members.iter().map(|member| member.hash).collect::<Vec<_>>();
    let matched = picture::align(&hashes(before), &hashes(after));
    let mut rows = Vec::with_capacity(before.len().max(after.len()));
    let (mut i, mut j) = (0, 0);
    // The end of both sides closes the last gap.
    for (b, a) in matched.into_iter().chain([(before.len(), after.len())]) {
        let paired = (b - i).min(a - j);
        rows.extend((0..paired).map(|k| Aligned {
            before: Some(i + k),
            after: Some(j + k),
            status: if before[i + k].hash == after[j + k].hash {
                MemberStatus::Same
            } else {
                MemberStatus::Changed
            },
        }));
        rows.extend((i + paired..b).map(|index| Aligned {
            before: Some(index),
            after: None,
            status: MemberStatus::Removed,
        }));
        rows.extend((j + paired..a).map(|index| Aligned {
            before: None,
            after: Some(index),
            status: MemberStatus::Added,
        }));
        if b < before.len() {
            rows.push(Aligned {
                before: Some(b),
                after: Some(a),
                status: MemberStatus::Same,
            });
        }
        (i, j) = (b + 1, a + 1);
    }
    rows
}

/// Compares two containers of `family` (either may be absent: an added or deleted file).
pub fn compare(
    family: Family,
    before: Option<&dyn Container>,
    after: Option<&dyn Container>,
    depth: usize,
) -> ContainerDiff {
    Budget::within(|| compare_within(family, before, after, depth))
}

fn compare_within(
    family: Family,
    before: Option<&dyn Container>,
    after: Option<&dyn Container>,
    depth: usize,
) -> ContainerDiff {
    let rows = align(before, after);
    let key = |row: &Aligned| {
        let (side, index) = match (row.after, row.before) {
            (Some(index), _) => (after, index),
            (None, Some(index)) => (before, index),
            (None, None) => unreachable!("an aligned row has a side"),
        };
        side.expect("a row's side is present").members()[index]
            .key
            .clone()
    };
    let changed: Vec<&Aligned> = rows
        .iter()
        .filter(|row| row.status == MemberStatus::Changed)
        .collect();
    // Opening and comparing changed members is the slow part (pictures, nested archives): on
    // every core.
    let openers = || Some((before?.opener(), after?.opener()));
    let details = parallel_map(&changed, openers, |openers, row| {
        let (open_before, open_after) = openers.as_mut()?;
        let index = |index: Option<usize>| index.expect("a changed member has both sides");
        contained("opening a member", || {
            Ok(compare_members(
                open_before(index(row.before))?,
                open_after(index(row.after))?,
                depth,
            ))
        })
        .ok()
    });
    let mut details = details.into_iter();
    let mut members = Vec::new();
    let mut unchanged = 0;
    for row in &rows {
        match row.status {
            MemberStatus::Same => unchanged += 1,
            status => members.push(MemberDiff {
                key: key(row),
                status,
                detail: if status == MemberStatus::Changed {
                    details.next().expect("one detail per changed member")
                } else {
                    None
                },
            }),
        }
    }
    ContainerDiff {
        family,
        before: before.map(|container| container.info().clone()),
        after: after.map(|container| container.info().clone()),
        unchanged,
        members,
    }
}

/// `f` over `items`, in order, spread over the machine's cores: each worker makes its own `state`
/// and takes the next item whenever it is done with one. The workers work within the caller's
/// [`Budget`].
fn parallel_map<T: Sync, S, U: Send>(
    items: &[T],
    state: impl Fn() -> S + Sync,
    f: impl Fn(&mut S, &T) -> U + Sync,
) -> Vec<U> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let threads = threads.min(items.len());
    let next = AtomicUsize::new(0);
    let budget = Budget::current();
    let (next, state, f) = (&next, &state, &f);
    let mut results: Vec<Option<U>> = std::iter::repeat_with(|| None).take(items.len()).collect();
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                let budget = budget.clone();
                scope.spawn(move || {
                    Budget::enter(budget, || {
                        let mut state = state();
                        let mut done = Vec::new();
                        loop {
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            let Some(item) = items.get(index) else {
                                break done;
                            };
                            done.push((index, f(&mut state, item)));
                        }
                    })
                })
            })
            .collect();
        for worker in workers {
            // `f` contains its own panics; one outside it (making the state) is the caller's.
            let done = worker
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            for (index, result) in done {
                results[index] = Some(result);
            }
        }
    });
    results
        .into_iter()
        .map(|result| result.expect("every item is done"))
        .collect()
}

/// What changed between two opened members.
pub fn compare_members(before: MemberContent, after: MemberContent, depth: usize) -> MemberDetail {
    Budget::within(|| compare_contents(before, after, depth))
}

fn compare_contents(before: MemberContent, after: MemberContent, depth: usize) -> MemberDetail {
    match (before, after) {
        (MemberContent::Picture(before), MemberContent::Picture(after)) => {
            MemberDetail::Picture(picture::compare_decoded(Some(&before), Some(&after)))
        }
        (MemberContent::Text(before), MemberContent::Text(after)) => text_detail(&before, &after),
        (MemberContent::Bytes(before), MemberContent::Bytes(after)) => {
            bytes_detail(&before, &after, depth)
        }
        // A member that changed kind (a glyph that lost its outline): only its bytes differ.
        (before, after) => MemberDetail::Binary {
            before_bytes: content_len(&before),
            after_bytes: content_len(&after),
        },
    }
}

fn content_len(content: &MemberContent) -> usize {
    match content {
        MemberContent::Bytes(bytes) => bytes.len(),
        MemberContent::Picture(picture) => picture.0.bytes,
        MemberContent::Text(text) => text.len(),
    }
}

/// Bytes diffed by what they are: content of a kind OmniDiff knows, else text, else bytes.
fn bytes_detail(before: &[u8], after: &[u8], depth: usize) -> MemberDetail {
    if depth < MAX_DEPTH
        && let Ok(Some(content)) = super::diff_at_depth(before, after, depth + 1)
    {
        return match content {
            ContentDiff::Picture(picture) => MemberDetail::Picture(picture),
            content => MemberDetail::Content {
                content: Box::new(content),
            },
        };
    }
    match (as_text(before), as_text(after)) {
        (Some(before), Some(after)) => text_detail(&before, &after),
        _ => MemberDetail::Binary {
            before_bytes: before.len(),
            after_bytes: after.len(),
        },
    }
}

/// A member's bytes as text: text itself, or a Java class file as its listing
/// ([`super::class::listing`]); `None` for bytes that are neither.
pub fn as_text(bytes: &[u8]) -> Option<String> {
    super::class::listing(bytes).or_else(|| crate::code::decode_text(bytes))
}

/// The edit distance past which two texts count as entirely different, as a file without a
/// grammar does (`diff::text::plain_text_line_diff`).
const TEXT_MAX_EDIT: usize = 10_000;

/// Two texts compared by lines.
pub fn text_detail(before: &str, after: &str) -> MemberDetail {
    let (before_lines, after_lines) = (before.lines().count(), after.lines().count());
    // Only the count is needed, which Myers finds in O(TEXT_MAX_EDIT) memory without the script.
    let matched = crate::diff::text::line_match_count(before, after, TEXT_MAX_EDIT).unwrap_or(0);
    let (removed, added) = (before_lines - matched, after_lines - matched);
    let changed = removed + added > 0;
    MemberDetail::Text {
        before_lines,
        after_lines,
        removed,
        added,
        whitespace: changed.then(|| whitespace_only(before, after)).flatten(),
        words_kept: changed && words_kept(before, after),
    }
}

/// Which whitespace alone two texts differ in, if only whitespace.
fn whitespace_only(before: &str, after: &str) -> Option<Whitespace> {
    let trimmed = |text: &str| {
        let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
        let end = lines
            .iter()
            .rposition(|line| !line.is_empty())
            .map_or(0, |at| at + 1);
        lines[..end].join("\n")
    };
    let visible = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    if trimmed(before) == trimmed(after) {
        Some(Whitespace::Trailing)
    } else if visible(before) == visible(after) {
        Some(Whitespace::Anywhere)
    } else {
        None
    }
}

/// True if two texts share at least half their words, counted with repeats.
fn words_kept(before: &str, after: &str) -> bool {
    let mut counts: HashMap<&str, (usize, usize)> = HashMap::new();
    for word in before.split_whitespace() {
        counts.entry(word).or_default().0 += 1;
    }
    for word in after.split_whitespace() {
        counts.entry(word).or_default().1 += 1;
    }
    let (shared, all) = counts.values().fold((0, 0), |(shared, all), (b, a)| {
        (shared + b.min(a), all + b.max(a))
    });
    2 * shared >= all
}

/// A hash of `bytes`, for a member's [`Member::hash`].
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    use std::hash::Hasher;
    let mut hasher = metrohash::MetroHash64::default();
    hasher.write(bytes);
    hasher.finish()
}

/// A container whose members are already in memory: a tar, a single compressed file, a test.
pub struct Loaded {
    pub info: ContainerInfo,
    pub members: Vec<Member>,
    pub contents: Vec<Vec<u8>>,
}

impl Loaded {
    /// A container of `files` (key, bytes), each member hashed by its bytes.
    pub fn new(format: &str, bytes: usize, files: Vec<(String, Vec<u8>)>) -> Self {
        let members = files
            .iter()
            .map(|(key, bytes)| Member {
                key: key.clone(),
                hash: hash_bytes(bytes),
            })
            .collect::<Vec<_>>();
        Self {
            info: ContainerInfo {
                format: format.to_string(),
                bytes,
                members: members.len(),
            },
            members,
            contents: files.into_iter().map(|(_, bytes)| bytes).collect(),
        }
    }
}

impl Container for Loaded {
    fn info(&self) -> &ContainerInfo {
        &self.info
    }

    fn members(&self) -> &[Member] {
        &self.members
    }

    fn open(&self, index: usize) -> Result<MemberContent> {
        Ok(MemberContent::Bytes(self.contents[index].clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(files: &[(&str, &str)]) -> Loaded {
        Loaded::new(
            "TEST",
            0,
            files
                .iter()
                .map(|(key, text)| (key.to_string(), text.as_bytes().to_vec()))
                .collect(),
        )
    }

    fn statuses(rows: &[Aligned]) -> Vec<MemberStatus> {
        rows.iter().map(|row| row.status).collect()
    }

    #[test]
    fn members_align_by_key_and_removed_ones_stay_where_they_were() {
        use MemberStatus::*;
        let before = loaded(&[("a", "1"), ("gone", "x"), ("b", "2"), ("c", "3")]);
        let after = loaded(&[("new", "n"), ("a", "1"), ("b", "two"), ("c", "3")]);
        let rows = align(Some(&before), Some(&after));
        assert_eq!(
            statuses(&rows),
            vec![Added, Same, Removed, Changed, Same],
            "{rows:?}"
        );
        assert_eq!(rows[2].before, Some(1));
    }

    #[test]
    fn a_removed_member_before_any_shared_one_comes_first() {
        let before = loaded(&[("gone", "x"), ("a", "1")]);
        let after = loaded(&[("a", "1")]);
        let rows = align(Some(&before), Some(&after));
        assert_eq!(rows[0].status, MemberStatus::Removed);
    }

    #[test]
    fn a_text_member_counts_its_lines() {
        let before = loaded(&[("readme", "one\ntwo\nthree\n")]);
        let after = loaded(&[("readme", "one\n2\nthree\nfour\n")]);
        let diff = compare(Family::Archives, Some(&before), Some(&after), 0);
        assert_eq!(diff.unchanged, 0);
        assert_eq!(
            diff.members[0].detail,
            Some(MemberDetail::Text {
                before_lines: 3,
                after_lines: 4,
                removed: 1,
                added: 2,
                whitespace: None,
                words_kept: false,
            })
        );
        assert_eq!(diff.members[0].verdict(), Some(Verdict::new(Level::Edited)));
    }

    #[test]
    fn a_containers_verdict_says_which_members_changed() {
        let before = loaded(&[("a", "1"), ("b", "2"), ("c", "3")]);
        let same = loaded(&[("a", "1"), ("b", "2"), ("c", "3")]);
        let one = loaded(&[("a", "1"), ("b", "2"), ("c", "x")]);
        let most = loaded(&[("a", "1"), ("y", "2"), ("c", "x")]);
        let crlf = loaded(&[("a", "1\r\n"), ("b", "2"), ("c", "3")]);
        let verdict = |family: Family, after: &Loaded| {
            compare(family, Some(&before), Some(after), 0)
                .verdict()
                .to_string()
        };
        // Every container says only whether and which members changed, whatever its family: the
        // members' verdicts say how.
        for family in [Family::Archives, Family::Fonts, Family::Catalogs] {
            let verdict = |after: &Loaded| verdict(family, after);
            assert_eq!(verdict(&same), "invisible");
            assert_eq!(
                verdict(&crlf),
                "edited+changed_files",
                "a member's bytes changed, though it reads the same"
            );
            assert_eq!(verdict(&one), "edited+changed_files");
            assert_eq!(
                verdict(&most),
                "edited+added_files+removed_files+changed_files"
            );
        }
        assert_eq!(
            compare(Family::Archives, None, Some(&same), 0)
                .verdict()
                .to_string(),
            "edited"
        );
    }

    #[test]
    fn a_text_member_is_judged_by_what_changed_in_it() {
        let level = |before: &str, after: &str| text_detail(before, after).verdict().to_string();
        assert_eq!(level("a b\n", "a b\r\n"), "invisible");
        assert_eq!(level("a b\n", "a b  \n\n"), "imperceptible");
        assert_eq!(level("a b\nc\n", "a\n  b c\n"), "artifacts");
        assert_eq!(level("a\nb\nc\nd\n", "a\nb\nc\nD\n"), "edited");
        assert_eq!(
            level("one two\nthree four\n", "three four one\ntwo\n"),
            "redrawn",
            "every line changed, every word kept"
        );
        assert_eq!(level("one two\nthree four\n", "five\nsix\n"), "replaced");
    }

    #[test]
    fn parallel_work_comes_back_in_order() {
        let items: Vec<u64> = (0..200).collect();
        // A few slow items early on, as one large member among small ones.
        let squares = parallel_map(
            &items,
            || (),
            |_, &item| {
                if item < 3 {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                item * item
            },
        );
        assert_eq!(
            squares,
            items.iter().map(|item| item * item).collect::<Vec<_>>()
        );
    }

    /// A container whose members panic when opened, as a decoder would on a crafted file.
    struct Panics(Loaded);

    impl Container for Panics {
        fn info(&self) -> &ContainerInfo {
            &self.0.info
        }
        fn members(&self) -> &[Member] {
            &self.0.members
        }
        fn open(&self, _: usize) -> Result<MemberContent> {
            panic!("a crafted member")
        }
    }

    #[test]
    fn a_member_that_panics_does_not_open() {
        let before = Panics(loaded(&[("a", "1"), ("b", "2")]));
        let after = loaded(&[("a", "one"), ("b", "two")]);
        let diff = compare(Family::Archives, Some(&before), Some(&after), 0);
        assert_eq!(diff.members.len(), 2);
        assert!(diff.members.iter().all(|member| member.detail.is_none()));
        assert_eq!(diff.members[0].verdict(), Some(Verdict::new(Level::Edited)));
    }

    #[test]
    fn workers_share_one_expansion_budget() -> Result<()> {
        use crate::diff::content::archive::{Zip, tests::zip};
        let (a, b) = (vec![b'a'; 1024 * 1024], vec![b'b'; 1024 * 1024]);
        let names: Vec<String> = (0..8).map(|n| format!("{n}.bin")).collect();
        let side = |contents: &[u8]| {
            zip(&names
                .iter()
                .map(|name| (name.as_str(), contents))
                .collect::<Vec<_>>())
        };
        let (before, after) = (Zip::new(&side(&a))?, Zip::new(&side(&b))?);
        // Eight changed members of a MiB a side; five MiB open two of them at most.
        let diff = Budget::enter(Some(Budget::of(5 * 1024 * 1024)), || {
            compare(Family::Archives, Some(&before), Some(&after), 0)
        });
        let opened = diff.members.iter().filter(|member| member.detail.is_some());
        assert!(opened.count() <= 2, "{diff:?}");
        let diff = compare(Family::Archives, Some(&before), Some(&after), 0);
        assert!(diff.members.iter().all(|member| member.detail.is_some()));
        Ok(())
    }
}
