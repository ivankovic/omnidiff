/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! The content session: a sample or fixture whose pair is content rather than code - two pictures
//! (`sample_test_diffs --content pictures`), two archives, fonts, ... (`--content <family>`) - is
//! judged here rather than in the tree session, which needs trees.
//!
//! What the human records is verdicts (`diff::content::Verdict`, saved by
//! `test::helper::human_content`): a level, keys `0`-`5`, and any of its tags, toggled by `r`
//! resized, `c` canvas, `R` rotated, `d` timing, `f` frames and `m` members - those that fit what is
//! judged: a picture's shape and timing, a container's members, nothing for text. `u` records that
//! it cannot be judged, with a typed note on what OmniDiff draws wrong. A picture pair gets one
//! verdict. A container pair gets one per changed member and, if the human wants, one for the
//! whole pair: the member list's first row is the pair, `j`/`k` move, `n`/`N` jump to the next or
//! previous member without a verdict, and `A` gives the last verdict to every member still without
//! one - for a font with hundreds of changed glyphs, which a pair verdict can also stand in for.
//!
//! The pair is shown through `ContentViewer` in its annotation mode - the files' own metadata, and
//! nothing the engine decided - so the verdicts stay the human's. `e` shows the engine's view on
//! request (its verdicts, outlined regions, the difference view) and hides it again; each sample
//! starts with it hidden. `s` promotes the sample to `src/test/data/<family>/<name>/` with its
//! verdicts and a stub, or, once promoted, saves changed verdicts; `x` rejects it with a reason.
//!
//! A content fixture is opened from `o`, which lists them beside the code cases (one dataset per
//! family): its verdicts are changed and saved the same way, and there is nothing to promote or
//! reject. `o` and `O` work here as in the tree session, which takes back over for a code case or
//! sample.
//!
//! The terminal is asked once which graphics protocol it speaks, on the first content session:
//! this binary reads keys synchronously, so the answer cannot be lost to an event reader.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use omnidiff::diff::content::container::{MemberDetail, MemberStatus};
use omnidiff::diff::content::{ContentDiff, Family, Level, Tag, Verdict};
use omnidiff::test::helper::human_content::{self, HumanContent, Judgement, Mismatches};
use omnidiff::tui::components::content_viewer::ContentViewer;
use omnidiff::tui::components::picture_viewer::{self, PictureColors};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui_image::picker::Picker;

use crate::events::{
    SessionEnd, handle_open_diff_picker, handle_open_sample_picker, open_diff_picker,
    open_sample_picker, reject_sample, update_sample_csv,
};
use crate::render::{render_open_diff_picker, render_open_sample_picker};
use crate::state::{App, Modal, OpenTarget};
use crate::stubs::{LICENSE_HEADER, fixtures_dir, insert_mod_declaration, module_name};
use crate::{
    DiffPickerData, SampleSource, promoted_case_name, samples_root, source_json_for_sample,
};

/// The content family of sample `name`, if its pair is content: the dataset its `source.json`
/// was sampled into, or failing that what its bytes are.
pub(crate) fn sample_family(name: &str) -> Option<Family> {
    let dir = samples_root().join(name);
    source_json_for_sample(name)
        .and_then(|source| Family::from_name(&source.dataset))
        .or_else(|| family_of_pair(&dir))
}

/// The family of the content fixture `name`: the family whose directory holds it.
pub(crate) fn fixture_family(name: &str) -> Option<Family> {
    Family::ALL
        .into_iter()
        .find(|family| pair_paths(&human_content::root(*family).join(name)).is_some())
}

/// What the pair in `dir` is, by its bytes.
fn family_of_pair(dir: &Path) -> Option<Family> {
    let (before, after) = pair_paths(dir)?;
    let read = |path: &Path| fs::read(path).ok();
    let (before, after) = (read(&before)?, read(&after)?);
    omnidiff::diff::content::pair_kind(&before, &after)?;
    let side = if before.is_empty() { &after } else { &before };
    Some(omnidiff::diff::content::sniff(side)?.family())
}

/// What a content session judges.
pub(crate) enum ContentCase {
    /// A content sample, from `O`.
    Sample(String, Family),
    /// A content fixture, from `o`.
    Fixture(String, Family),
}

impl ContentCase {
    /// The content case `end` opens, if it opens one rather than a code case or sample.
    pub(crate) fn opened_by(end: &SessionEnd) -> Option<ContentCase> {
        match end {
            SessionEnd::Open(OpenTarget::Sample(name)) => {
                sample_family(name).map(|family| ContentCase::Sample(name.clone(), family))
            }
            SessionEnd::Open(OpenTarget::Diffs(name)) => {
                fixture_family(name).map(|family| ContentCase::Fixture(name.clone(), family))
            }
            _ => None,
        }
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            ContentCase::Sample(name, _) | ContentCase::Fixture(name, _) => name,
        }
    }

    fn family(&self) -> Family {
        match self {
            ContentCase::Sample(_, family) | ContentCase::Fixture(_, family) => *family,
        }
    }
}

/// `dir`'s `before.<ext>.test` and `after.<ext>.test`.
pub(crate) fn pair_paths(dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut before = None;
    let mut after = None;
    for entry in fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        let file = path.file_name()?.to_string_lossy().into_owned();
        if !file.ends_with(".test") {
            continue;
        }
        if file.starts_with("before.") {
            before = Some(path);
        } else if file.starts_with("after.") {
            after = Some(path);
        }
    }
    before.zip(after)
}

/// The terminal's graphics protocol, asked once; half blocks if it does not answer.
fn picker() -> Picker {
    static PICKER: OnceLock<Picker> = OnceLock::new();
    PICKER
        .get_or_init(|| picture_viewer::query_graphics().unwrap_or_else(Picker::halfblocks))
        .clone()
}

/// Where the pair being judged lives, and so what `s` and `x` do.
enum Origin {
    /// A sample, and the content fixture it was promoted to once it is.
    Sample {
        source: SampleSource,
        promoted: Option<String>,
    },
    /// A content fixture opened from `o`.
    Fixture(String),
}

impl Origin {
    /// The content fixture verdicts are saved into, once there is one.
    fn fixture(&self) -> Option<&str> {
        match self {
            Origin::Sample { promoted, .. } => promoted.as_deref(),
            Origin::Fixture(fixture) => Some(fixture),
        }
    }
}

/// The verdicts given so far: on the pair, and per changed member of a container.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Judgements {
    pair: Option<Judgement>,
    members: BTreeMap<String, Judgement>,
}

impl Judgements {
    fn is_empty(&self) -> bool {
        self.pair.is_none() && self.members.is_empty()
    }
}

struct ContentSession {
    name: String,
    family: Family,
    origin: Origin,
    /// The file's path in its repository, for the title line.
    path: String,
    viewer: ContentViewer,
    judgement: Judgements,
    /// The verdicts on disk, to warn before quitting with unsaved ones.
    saved: Judgements,
    /// The verdict last given, which `A` gives to every member without one.
    last: Option<Judgement>,
    /// A rejection reason being typed, after `x`.
    reject_input: Option<String>,
    /// A can't-judge note being typed, after `u`.
    note_input: Option<String>,
    /// `q` was pressed with unsaved verdicts, and was warned about it: another `q` quits.
    quit_armed: bool,
    status: String,
}

/// The verdicts saved for `family`'s fixture `fixture`, if any.
fn load_judgement(family: Family, fixture: &str) -> Judgements {
    human_content::load(family, fixture)
        .map(|content| Judgements {
            pair: content.verdict,
            members: content.members,
        })
        .unwrap_or_default()
}

/// Runs the content session for `case` until the human quits or opens something else.
pub(crate) fn run_content_session(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    case: &ContentCase,
) -> Result<SessionEnd> {
    let family = case.family();
    let (dir, origin) = match case {
        ContentCase::Sample(name, _) => {
            let dir = samples_root().join(name);
            let source = source_json_for_sample(name)
                .with_context(|| format!("{dir:?} has no readable source.json"))?;
            let promoted = promoted_case_name(&source);
            (dir, Origin::Sample { source, promoted })
        }
        ContentCase::Fixture(name, _) => (
            human_content::root(family).join(name),
            Origin::Fixture(name.clone()),
        ),
    };
    let path = match &origin {
        Origin::Sample { source, .. } => source.path.clone(),
        Origin::Fixture(_) => readme_file(&dir).unwrap_or_default(),
    };
    let (before, after) =
        pair_paths(&dir).with_context(|| format!("{dir:?} has no before/after pair"))?;
    let viewer = ContentViewer::open_for_annotation(&before, &after, picker())
        .with_context(|| format!("{dir:?} is not a pair of {} that decode", family.name()))?;
    let saved = origin
        .fixture()
        .map(|fixture| load_judgement(family, fixture))
        .unwrap_or_default();
    let mut session = ContentSession {
        name: case.name().to_string(),
        family,
        origin,
        path,
        viewer,
        judgement: saved.clone(),
        saved,
        last: None,
        reject_input: None,
        note_input: None,
        quit_armed: false,
        status: "0-5 records a verdict; s saves it".to_string(),
    };
    update_marks(&mut session);
    loop {
        omnidiff::tui::ui::draw_synchronized(terminal, |frame| draw(frame, &mut session, app))?;
        // A playing animation needs drawing at its frame rate, not only on a key.
        let wait = if session.viewer.is_playing() { 20 } else { 250 };
        if !event::poll(Duration::from_millis(wait))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if let Some(log) = app.key_log.as_mut() {
            let typing = session.reject_input.is_some() || session.note_input.is_some();
            let mode = if typing { "content-typing" } else { "content" };
            let text = if typing {
                "typed".to_string()
            } else {
                format!("{:?}", key.code)
            };
            log.record(&session.name, mode, &text, false);
        }

        if let Some(end) = handle_key(&mut session, app, key.code) {
            return Ok(end);
        }
    }
}

/// The keys that toggle each tag.
const TAG_KEYS: [(char, Tag); 6] = [
    ('r', Tag::Resized),
    ('c', Tag::Canvas),
    ('R', Tag::Rotated),
    ('d', Tag::Timing),
    ('f', Tag::Frames),
    ('m', Tag::Members),
];

/// What `0`-`5` give a verdict to: the selected member of a container, or the pair.
enum Target {
    Pair,
    Member(String),
}

fn target(session: &ContentSession) -> Result<Target, String> {
    let Some(members) = session.viewer.members() else {
        return Ok(Target::Pair);
    };
    if members.has_nested() {
        return Err("Verdicts are given at the top level: Backspace goes back".to_string());
    }
    match (members.selected_key(), members.selected_status()) {
        (None, _) => Ok(Target::Pair),
        (Some(key), Some(MemberStatus::Changed)) => Ok(Target::Member(key.to_string())),
        (Some(key), _) => Err(format!(
            "Only a changed member takes a verdict; {key} is unchanged, added or removed"
        )),
    }
}

/// What is judged, for the tags it can take: is it a picture, is it a container.
fn shape(session: &ContentSession, target: &Target) -> (bool, bool) {
    let Some(members) = session.viewer.members() else {
        return (true, false);
    };
    let Target::Member(key) = target else {
        return (false, true);
    };
    match members
        .diff()
        .member(key)
        .and_then(|member| member.detail.as_ref())
    {
        Some(MemberDetail::Picture(_)) => (true, false),
        Some(MemberDetail::Content { content }) => match content.as_ref() {
            ContentDiff::Picture(_) => (true, false),
            ContentDiff::Container(_) => (false, true),
        },
        _ => (false, false),
    }
}

/// True if the member `key` is text, whose levels have names of their own (`Level::text_name`):
/// every message of a catalog, and any member diffed as text.
fn is_text(session: &ContentSession, key: &str) -> bool {
    is_catalog(session)
        || session
            .viewer
            .members()
            .and_then(|members| members.diff().member(key))
            .and_then(|member| member.detail.as_ref())
            .is_some_and(|detail| matches!(detail, MemberDetail::Text { .. }))
}

/// A message catalog is text through and through: its messages, and so the whole file.
fn is_catalog(session: &ContentSession) -> bool {
    session.family == Family::Catalogs
}

/// [`is_text`] for `target`; the pair is text only for a catalog.
fn target_is_text(session: &ContentSession, target: &Target) -> bool {
    match target {
        Target::Pair => is_catalog(session),
        Target::Member(key) => is_text(session, key),
    }
}

/// `judgement` with only the tags `target` can take.
fn fitted(session: &ContentSession, target: &Target, judgement: &Judgement) -> Judgement {
    let (picture, container) = shape(session, target);
    match judgement {
        Judgement::Verdict(verdict) => {
            let mut fitted = Verdict::new(verdict.level);
            for tag in verdict.tags.iter() {
                if tag.fits(picture, container) {
                    fitted = fitted.with(tag);
                }
            }
            Judgement::Verdict(fitted)
        }
        cant => cant.clone(),
    }
}

/// The judgement `target` has so far.
fn judgement_of(session: &ContentSession, target: &Target) -> Option<Judgement> {
    match target {
        Target::Pair => session.judgement.pair.clone(),
        Target::Member(key) => session.judgement.members.get(key).cloned(),
    }
}

/// Records `judgement` for `target`.
fn record(session: &mut ContentSession, target: Target, judgement: Judgement) {
    let label = judgement.label_for(target_is_text(session, &target));
    session.status = match &target {
        Target::Pair => format!("The whole file: {label} (s to save)"),
        Target::Member(key) => format!("{key}: {label} (s to save)"),
    };
    session.last = Some(judgement.clone());
    match target {
        Target::Pair => session.judgement.pair = Some(judgement),
        Target::Member(key) => {
            session.judgement.members.insert(key, judgement);
        }
    }
    update_marks(session);
}

/// Gives `level` to whatever is selected, keeping the tags it has.
fn judge(session: &mut ContentSession, level: Level) {
    match target(session) {
        Ok(target) => {
            let tags = match judgement_of(session, &target) {
                Some(Judgement::Verdict(verdict)) => verdict.tags,
                _ => Default::default(),
            };
            record(session, target, Judgement::Verdict(Verdict { level, tags }));
        }
        Err(message) => session.status = message,
    }
}

/// Toggles `tag` on whatever is selected, which needs a level first.
fn toggle_tag(session: &mut ContentSession, tag: Tag) {
    let target = match target(session) {
        Ok(target) => target,
        Err(message) => {
            session.status = message;
            return;
        }
    };
    let (picture, container) = shape(session, &target);
    if !tag.fits(picture, container) {
        let what = match (&target, picture, container) {
            (_, false, false) => "text or bytes",
            (Target::Pair, false, true) | (Target::Member(_), false, true) => "a container",
            _ => "a picture",
        };
        session.status = format!("{what} is never {}", tag.name());
        return;
    }
    let Some(Judgement::Verdict(mut verdict)) = judgement_of(session, &target) else {
        session.status = format!("Give a level first (0-5), then {} tags it", tag.name());
        return;
    };
    verdict.tags.toggle(tag);
    record(session, target, Judgement::Verdict(verdict));
}

/// Moves to the next (or previous) changed member without a verdict, from the selection.
fn next_unjudged(session: &mut ContentSession, forward: bool) {
    let Some(members) = session.viewer.members() else {
        return;
    };
    let keys = members.changed_keys();
    let current = members
        .selected_key()
        .and_then(|key| keys.iter().position(|candidate| candidate == key));
    let order: Vec<usize> = if forward {
        let start = current.map_or(0, |index| index + 1);
        (start..keys.len()).chain(0..start).collect()
    } else {
        let start = current.unwrap_or(keys.len());
        (0..start).rev().chain((start..keys.len()).rev()).collect()
    };
    let next = order
        .into_iter()
        .map(|index| &keys[index])
        .find(|key| !session.judgement.members.contains_key(*key))
        .cloned();
    match next {
        Some(key) => {
            if let Some(members) = session.viewer.members_mut() {
                members.select(Some(&key));
            }
            session.status = format!("{key} has no verdict yet");
        }
        None => session.status = "Every changed member has a verdict".to_string(),
    }
}

/// `A`: gives the last verdict to every changed member still without one, with the tags each
/// can take.
fn judge_the_rest(session: &mut ContentSession) {
    let Some(judgement) = session.last.clone() else {
        session.status = "Give one verdict first: A repeats it for the rest".to_string();
        return;
    };
    let Some(members) = session.viewer.members() else {
        session.status = "A picture has no members: 0-5 judges it".to_string();
        return;
    };
    let rest: Vec<String> = members
        .changed_keys()
        .into_iter()
        .filter(|key| !session.judgement.members.contains_key(key))
        .collect();
    let given = rest.len();
    for key in rest {
        let target = Target::Member(key.clone());
        let fitted = fitted(session, &target, &judgement);
        session.judgement.members.insert(key, fitted);
    }
    let noun = if given == 1 { "member" } else { "members" };
    let label = judgement.label_for(is_catalog(session));
    session.status = format!("{given} more {noun}: {label} (s to save)");
    update_marks(session);
}

/// Shows each judged member's verdict in the list, and the engine's where it is shown and differs.
fn update_marks(session: &mut ContentSession) {
    let judgement = session.judgement.clone();
    let pair_is_text = is_catalog(session);
    let texts: std::collections::HashSet<String> = session
        .viewer
        .members()
        .map(|members| members.changed_keys())
        .unwrap_or_default()
        .into_iter()
        .filter(|key| is_text(session, key))
        .collect();
    let Some(members) = session.viewer.members_mut() else {
        return;
    };
    let engine = !members.annotating();
    let mut marks: HashMap<String, String> = HashMap::new();
    let mut keys: Vec<String> = members.changed_keys();
    keys.push(String::new());
    for key in keys {
        let human = if key.is_empty() {
            judgement.pair.as_ref()
        } else {
            judgement.members.get(&key)
        };
        let found = if !engine {
            None
        } else if key.is_empty() {
            Some(members.diff().verdict())
        } else {
            members.member_verdict(&key)
        };
        let text = if key.is_empty() {
            pair_is_text
        } else {
            texts.contains(&key)
        };
        let mark = match (human, found) {
            (Some(human), Some(found)) if human.verdict().is_some_and(|human| human != found) => {
                format!(
                    "{} (omnidiff: {})",
                    human.label_for(text),
                    found.label_for(text)
                )
            }
            (Some(human), _) => human.label_for(text),
            (None, Some(found)) => format!("(omnidiff: {})", found.label_for(text)),
            (None, None) => continue,
        };
        marks.insert(key, mark);
    }
    members.set_marks(marks);
}

/// One key of the content session; `Some` when it ends the session.
fn handle_key(session: &mut ContentSession, app: &mut App, code: KeyCode) -> Option<SessionEnd> {
    // The `o` and `O` pickers, while open, take every key. Any other modal is the tree
    // session's and has no place here.
    let picked = match app.modal.take() {
        Some(Modal::OpenDiffPicker {
            options,
            selected,
            view,
            name_input,
        }) => Some(without_unsaved(app, |app| {
            handle_open_diff_picker(app, code, options, selected, view, name_input)
        })),
        Some(Modal::OpenSamplePicker {
            rows,
            selected,
            view,
            name_input,
        }) => Some(without_unsaved(app, |app| {
            handle_open_sample_picker(app, code, rows, selected, view, name_input)
        })),
        _ => None,
    };
    if let Some(target) = picked {
        return target.map(SessionEnd::Open);
    }

    if let Some(mut note) = session.note_input.take() {
        match code {
            KeyCode::Enter => match target(session) {
                Ok(target) => {
                    record(
                        session,
                        target,
                        Judgement::CantJudge(note.trim().to_string()),
                    );
                }
                Err(message) => session.status = message,
            },
            KeyCode::Esc => session.status = "Can't judge cancelled".to_string(),
            KeyCode::Backspace => {
                note.pop();
                session.note_input = Some(note);
            }
            KeyCode::Char(c) => {
                note.push(c);
                session.note_input = Some(note);
            }
            _ => session.note_input = Some(note),
        }
        return None;
    }

    if let Some(mut reason) = session.reject_input.take() {
        match code {
            KeyCode::Enter => {
                session.status = match reject(session, &reason) {
                    Ok(message) => message,
                    Err(err) => format!("{err:#}"),
                };
            }
            KeyCode::Esc => session.status = "Rejection cancelled".to_string(),
            KeyCode::Backspace => {
                reason.pop();
                session.reject_input = Some(reason);
            }
            KeyCode::Char(c) => {
                reason.push(c);
                session.reject_input = Some(reason);
            }
            _ => session.reject_input = Some(reason),
        }
        return None;
    }

    let quitting = matches!(code, KeyCode::Char('q'));
    match code {
        KeyCode::Char(digit @ '0'..='5') => {
            judge(session, Level::ALL[digit as usize - '0' as usize]);
        }
        KeyCode::Char(key) if let Some((_, tag)) = TAG_KEYS.iter().find(|(k, _)| *k == key) => {
            toggle_tag(session, *tag);
        }
        KeyCode::Char('u') => match target(session) {
            Ok(_) => session.note_input = Some(String::new()),
            Err(message) => session.status = message,
        },
        KeyCode::Char('n') => next_unjudged(session, true),
        KeyCode::Char('N') => next_unjudged(session, false),
        KeyCode::Char('A') => judge_the_rest(session),
        KeyCode::Char('s') => {
            session.status = match save(session) {
                Ok(message) => message,
                Err(err) => format!("{err:#}"),
            };
        }
        KeyCode::Char('x') => match &session.origin {
            Origin::Sample { promoted: None, .. } => {
                session.reject_input = Some(String::new());
            }
            Origin::Sample { .. } => {
                session.status = "A promoted sample cannot be rejected".to_string();
            }
            Origin::Fixture(_) => {
                session.status = "A fixture cannot be rejected".to_string();
            }
        },
        KeyCode::Char('e') => {
            let show = session.viewer.annotating();
            session.viewer.set_annotating(!show);
            session.status = if show {
                format!(
                    "omnidiff says: {} (e hides it)",
                    session.viewer.verdict().label_for(is_catalog(session))
                )
            } else {
                "omnidiff's view hidden".to_string()
            };
            update_marks(session);
        }
        KeyCode::Char('o') => open_diff_picker(app, &session.name),
        KeyCode::Char('O') => open_sample_picker(app, &session.name),
        KeyCode::Char('q') => {
            if session.judgement != session.saved && !session.quit_armed {
                session.status =
                    "The verdicts are not saved: s saves them, q again quits anyway".to_string();
                session.quit_armed = true;
            } else {
                return Some(SessionEnd::Quit);
            }
        }
        code => {
            session.viewer.handle_key(code);
        }
    }
    if !quitting {
        session.quit_armed = false;
    }
    None
}

fn draw(frame: &mut ratatui::Frame, session: &mut ContentSession, app: &App) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let state = match &session.origin {
        Origin::Sample {
            promoted: Some(fixture),
            ..
        } => format!("promoted to {fixture}"),
        Origin::Sample { promoted: None, .. } => "sample".to_string(),
        Origin::Fixture(_) => "fixture".to_string(),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            session.name.as_str().bold(),
            format!("  ({state}, {})", session.path).dim(),
        ])),
        rows[0],
    );
    let status = if session.viewer.annotating() {
        Line::from(session.viewer.status())
    } else {
        Line::from(vec![
            format!(
                "omnidiff: {}",
                session.viewer.verdict().label_for(is_catalog(session))
            )
            .cyan(),
            format!(" · {}", session.viewer.status()).into(),
        ])
    };
    frame.render_widget(Paragraph::new(status), rows[1]);

    let target = target(session);
    let (current, what) = match &target {
        Ok(target @ Target::Member(key)) => (judgement_of(session, target), key.clone()),
        Ok(target @ Target::Pair) => (judgement_of(session, target), "the whole file".to_string()),
        Err(_) => (None, "this member (it takes none)".to_string()),
    };
    let mut choices: Vec<Span> = match session.viewer.members() {
        Some(_) => vec![format!("Verdict for {what}: ").into()],
        None => vec!["Verdict: ".into()],
    };
    let reversed = Style::new().add_modifier(Modifier::REVERSED);
    let verdict = current.as_ref().and_then(Judgement::verdict);
    let text = target
        .as_ref()
        .is_ok_and(|target| target_is_text(session, target));
    for (index, level) in Level::ALL.iter().enumerate() {
        let text = format!("{index} {}", level.name_for(text));
        choices.push(if verdict.is_some_and(|verdict| verdict.level == *level) {
            Span::styled(text, reversed)
        } else {
            text.into()
        });
        choices.push(" ".into());
    }
    let (picture, container) = match &target {
        Ok(target) => shape(session, target),
        Err(_) => (false, false),
    };
    for (key, tag) in TAG_KEYS {
        if !tag.fits(picture, container) {
            continue;
        }
        let text = format!("{key} {}", tag.name());
        choices.push(
            if verdict.is_some_and(|verdict| verdict.tags.contains(tag)) {
                Span::styled(text, reversed)
            } else {
                text.dim()
            },
        );
        choices.push(" ".into());
    }
    let cant = "u can't judge";
    choices.push(match &current {
        Some(Judgement::CantJudge(_)) => Span::styled(cant, reversed),
        _ => cant.dim(),
    });
    choices.push(" ".into());
    if let Some(members) = session.viewer.members() {
        let changed = members.changed_keys();
        let judged = changed
            .iter()
            .filter(|key| session.judgement.members.contains_key(*key))
            .count();
        choices.push(format!("{judged}/{} members judged ", changed.len()).dim());
    }
    if !session.judgement.is_empty() && session.judgement != session.saved {
        choices.push("(unsaved)".yellow());
    }

    frame.render_widget(Paragraph::new(Line::from(choices)), rows[2]);

    // The content is drawn as it looks: no theme to follow, only the two outline colors the
    // annotation view never uses.
    let colors = PictureColors::from_theme(Color::Red, Color::Green, Color::Yellow);
    session.viewer.draw(frame, rows[3], colors);

    let prompt = match (&session.reject_input, &session.note_input) {
        (Some(reason), _) => format!("Reject because: {reason}_ (Enter rejects, Esc cancels)"),
        (None, Some(note)) => {
            format!("Can't judge, because OmniDiff: {note}_ (Enter records it, Esc cancels)")
        }
        (None, None) => session.status.clone(),
    };
    frame.render_widget(Paragraph::new(prompt), rows[4]);
    let keys = match session.origin {
        Origin::Fixture(_) => "0-5 level  tags  u can't judge  s save",
        Origin::Sample { .. } => "0-5 level  tags  u can't judge  s promote/save  x reject",
    };
    let more = match session.viewer.members() {
        Some(_) => {
            "  j/k member  n/N unjudged  A rest  a all  g grid  J/K change  Enter open  b blocks"
        }
        None => "  t view  h/l swipe  +/- zoom  HJKL pan  b blocks",
    };
    frame.render_widget(
        Paragraph::new(
            format!("{keys}{more}  e omnidiff's view  o cases  O samples  q quit").dim(),
        ),
        rows[5],
    );

    match &app.modal {
        Some(Modal::OpenDiffPicker {
            options,
            selected,
            view,
            name_input,
        }) => render_open_diff_picker(
            frame,
            area,
            options,
            *selected,
            view,
            name_input.as_deref(),
            DiffPickerData::from_app(app),
            app.diff_comments.as_ref(),
        ),
        Some(Modal::OpenSamplePicker {
            rows,
            selected,
            view,
            name_input,
        }) => render_open_sample_picker(frame, area, rows, *selected, view, name_input.as_deref()),
        _ => {}
    }
}

/// Runs a picker's key handler as if the tree session's case had nothing unsaved. The pickers
/// would otherwise ask whether to discard it, in a modal this session cannot show; and the question
/// was already answered, by the picker that opened this session.
fn without_unsaved<T>(app: &mut App, handle: impl FnOnce(&mut App) -> T) -> T {
    let dirty = std::mem::replace(&mut app.dirty, false);
    let result = handle(app);
    app.dirty = dirty;
    result
}

/// The `File` line of a content fixture's README: the file's path in its repository.
fn readme_file(dir: &Path) -> Option<String> {
    let readme = fs::read_to_string(dir.join("README.md")).ok()?;
    readme.lines().find_map(|line| {
        let file = line.strip_prefix("- **File:** `")?;
        Some(file.strip_suffix('`')?.to_string())
    })
}

/// Promotes the sample to a content fixture with the current verdicts, or saves changed verdicts
/// into the fixture it was promoted to or opened as. Either way the fixture's stub is rewritten
/// to match what the engine says now.
fn save(session: &mut ContentSession) -> Result<String> {
    if session.judgement.is_empty() {
        bail!("No verdict yet: 0-5 records one");
    }
    if session.viewer.members().is_none() && session.judgement.pair.is_none() {
        bail!("No verdict yet: 0-5 records one");
    }
    let family = session.family;
    let human = human_content_of(session);
    let fixture = match &mut session.origin {
        Origin::Fixture(fixture)
        | Origin::Sample {
            promoted: Some(fixture),
            ..
        } => {
            human_content::save(family, fixture, &human)?;
            fixture.clone()
        }
        Origin::Sample {
            source,
            promoted: promoted @ None,
        } => {
            let fixture = session.name.clone();
            promote(family, &session.name, &fixture)?;
            human_content::save(family, &fixture, &human)?;
            if !update_sample_csv(source, &fixture)? {
                bail!("promoted to '{fixture}', but its sample.csv row was not found");
            }
            *promoted = Some(fixture.clone());
            fixture
        }
    };
    session.saved = session.judgement.clone();
    let engine = human_content::engine_diff(family, &fixture)?;
    let mismatches = human_content::mismatches(&human, &engine);
    write_stub(
        family,
        &fixture,
        &stub_contents(family, &fixture, &mismatches),
    )?;
    let disagreements = usize::from(mismatches.pair.is_some()) + mismatches.members.len();
    Ok(
        match (disagreements, mismatches.pair, session.viewer.members()) {
            (0, _, _) => format!("Saved '{fixture}' (omnidiff agrees)"),
            (_, Some(found), None) => {
                format!("Saved '{fixture}' (omnidiff says {found}; recorded in the stub)")
            }
            (n, _, _) => {
                format!("Saved '{fixture}' (omnidiff disagrees on {n}; recorded in the stub)")
            }
        },
    )
}

/// The ground truth the session's verdicts make.
fn human_content_of(session: &ContentSession) -> HumanContent {
    let changed = session
        .viewer
        .members()
        .map(|members| members.changed_keys())
        .unwrap_or_default();
    HumanContent {
        verdict: session.judgement.pair.clone(),
        members: session.judgement.members.clone(),
        unjudged: changed
            .iter()
            .filter(|key| !session.judgement.members.contains_key(*key))
            .count(),
    }
}

/// Copies sample `sample`'s pair and README into `src/test/data/<family>/<fixture>/`.
fn promote(family: Family, sample: &str, fixture: &str) -> Result<()> {
    let dir = human_content::root(family).join(fixture);
    if dir.exists() {
        bail!("{dir:?} already exists");
    }
    let from = samples_root().join(sample);
    let (before, after) =
        pair_paths(&from).with_context(|| format!("{from:?} has no before/after pair"))?;
    fs::create_dir_all(&dir).with_context(|| format!("creating {dir:?}"))?;
    for file in [before, after, from.join("README.md")] {
        let name = file.file_name().context("a file name")?;
        fs::copy(&file, dir.join(name)).with_context(|| format!("copying {file:?}"))?;
    }
    Ok(())
}

/// `fixtures/<family>/<fixture>.rs`, `contents`, rewritten on every save: a verdict stub holds
/// nothing written by hand.
fn write_stub(family: Family, fixture: &str, contents: &str) -> Result<()> {
    let module = module_name(fixture);
    let dir = fixtures_dir(family.name());
    fs::create_dir_all(&dir).with_context(|| format!("creating {dir:?}"))?;
    let path = dir.join(format!("{module}.rs"));
    fs::write(&path, contents).with_context(|| format!("writing {path:?}"))?;
    // Best effort, as for the tree stubs: the file is valid either way.
    let _ = std::process::Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(&path)
        .status();
    insert_mod_declaration(family.name(), &module)
}

/// A content fixture's stub: `assert_matches_human_verdicts`, or, when the engine disagrees,
/// `assert_known_verdict_mismatches` pinned to what it says now.
pub(crate) fn stub_contents(family: Family, fixture: &str, mismatches: &Mismatches) -> String {
    let verdict = |verdict: Option<Verdict>| match verdict {
        Some(verdict) => format!("Some(\"{verdict}\")"),
        None => "None".to_string(),
    };
    let body = if mismatches.is_empty() {
        format!(
            "    human_content::assert_matches_human_verdicts(Family::{family:?}, \"{fixture}\")"
        )
    } else {
        let members: Vec<String> = mismatches
            .members
            .iter()
            .map(|(key, found)| format!("({key:?}, {})", verdict(*found)))
            .collect();
        format!(
            "    // Recorded as found, not examined.\n    human_content::assert_known_verdict_mismatches(\n        Family::{family:?},\n        \"{fixture}\",\n        {},\n        &[{}],\n    )",
            verdict(mismatches.pair),
            members.join(", ")
        )
    };
    format!(
        "{LICENSE_HEADER}use anyhow::Result;\n\nuse crate::test::helper::human_content::{{self, Family}};\n\n#[test]\nfn verdicts() -> Result<()> {{\n{body}\n}}\n"
    )
}

fn reject(session: &ContentSession, reason: &str) -> Result<String> {
    let Origin::Sample { source, .. } = &session.origin else {
        bail!("A fixture cannot be rejected");
    };
    let reason = reason.trim();
    if reason.is_empty() {
        bail!("Rejection reason cannot be empty");
    }
    match reject_sample(source, reason)? {
        true => Ok(format!("Rejected '{}': {reason}", session.name)),
        false => bail!("source row not found in sample.csv; not updated"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CaseOrigin;
    use omnidiff::test::helper::human_mapping::HumanMapping;

    /// A session over a generated pair in `dir`: white 8x4 pictures, one pixel apart.
    fn session_over(dir: &Path, name: &str, origin: Origin, path: &str) -> ContentSession {
        let mut after = image::RgbaImage::from_pixel(8, 4, image::Rgba([255, 255, 255, 255]));
        let before = after.clone();
        after.put_pixel(1, 1, image::Rgba([0, 0, 0, 255]));
        let (a, b) = (dir.join("before.png.test"), dir.join("after.png.test"));
        before
            .save_with_format(&a, image::ImageFormat::Png)
            .unwrap();
        after.save_with_format(&b, image::ImageFormat::Png).unwrap();
        ContentSession {
            name: name.to_string(),
            family: Family::Pictures,
            origin,
            path: path.to_string(),
            viewer: ContentViewer::open_for_annotation(&a, &b, Picker::halfblocks()).unwrap(),
            judgement: Judgements::default(),
            saved: Judgements::default(),
            last: None,
            reject_input: None,
            note_input: None,
            quit_armed: false,
            status: String::new(),
        }
    }

    fn sample_origin() -> Origin {
        Origin::Sample {
            source: SampleSource {
                language: "PNG".to_string(),
                repository: "repo".to_string(),
                commit: "1234abcd".to_string(),
                path: "assets/logo.png".to_string(),
                dataset: Family::Pictures.name().to_string(),
            },
            promoted: None,
        }
    }

    fn test_app() -> App {
        App::new(
            "x".to_string(),
            CaseOrigin::Diffs,
            0,
            0,
            HumanMapping::default(),
        )
    }

    fn screen(session: &mut ContentSession, app: &App, width: u16) -> String {
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(width, 16)).unwrap();
        terminal.draw(|frame| draw(frame, session, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    /// The bug this guards: `o` fell through to the viewer, which ignores it, so the only way back
    /// to a code case was `O` or a restart.
    #[test]
    fn o_opens_the_case_picker_and_esc_hands_the_keys_back() {
        // A real fixture, so the picker has its row to put the cursor on.
        let fixture = crate::list_dir_names(&human_content::root(Family::Pictures))
            .unwrap()
            .pop()
            .expect("at least one picture fixture");
        let dir = tempfile::tempdir().unwrap();
        let mut session = session_over(
            dir.path(),
            &fixture,
            Origin::Fixture(fixture.clone()),
            "logo.png",
        );
        let mut app = test_app();

        assert!(handle_key(&mut session, &mut app, KeyCode::Char('o')).is_none());
        let Some(Modal::OpenDiffPicker {
            options, selected, ..
        }) = &app.modal
        else {
            panic!("o must open the case picker, got {:?}", app.modal);
        };
        let visible =
            crate::visible_diff_options(options, &app.diff_view, DiffPickerData::from_app(&app));
        assert_eq!(
            visible[*selected], fixture,
            "the cursor starts on the picture being judged, not the tree session's case"
        );
        assert!(
            options
                .iter()
                .any(|(_, dataset)| *dataset == Family::Pictures.name()),
            "the picture fixtures are listed"
        );
        assert!(
            options
                .iter()
                .any(|(_, dataset)| *dataset != Family::Pictures.name()),
            "and so are the code cases"
        );
        assert!(
            screen(&mut session, &app, 160).contains("Open diff ["),
            "the picker is drawn over the pictures"
        );

        assert!(handle_key(&mut session, &mut app, KeyCode::Esc).is_none());
        assert!(app.modal.is_none());
        handle_key(&mut session, &mut app, KeyCode::Char('3'));
        assert_eq!(
            session.judgement.pair,
            Some(Judgement::Verdict(Verdict::new(Level::Edited))),
            "with the picker closed, keys reach the session again"
        );
    }

    /// The tree case left behind was already answered for by the picker that opened this session;
    /// asking again would raise a modal this session cannot show.
    #[test]
    fn enter_in_the_case_picker_opens_the_case_over_an_unsaved_tree_case() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = session_over(
            dir.path(),
            "png-x-repo-1234abcd-logo",
            sample_origin(),
            "assets/logo.png",
        );
        let mut app = test_app();
        app.dirty = true;
        app.modal = Some(Modal::OpenDiffPicker {
            options: vec![("png-a".to_string(), Family::Pictures.name())],
            selected: 0,
            view: Default::default(),
            name_input: None,
        });

        let end = handle_key(&mut session, &mut app, KeyCode::Enter);
        assert!(
            matches!(&end, Some(SessionEnd::Open(OpenTarget::Diffs(name))) if name == "png-a"),
            "Enter opens the selected case"
        );
        assert!(app.dirty, "the tree session's own flag is left as it was");
    }

    #[test]
    fn opened_by_sends_picture_fixtures_to_the_content_session_and_code_cases_back() {
        let fixture = crate::list_dir_names(&human_content::root(Family::Pictures))
            .unwrap()
            .into_iter()
            .next()
            .expect("at least one picture fixture");
        let code = crate::list_available_cases().unwrap()[0].0.clone();

        let open = |name: &str| SessionEnd::Open(OpenTarget::Diffs(name.to_string()));
        assert!(matches!(
            ContentCase::opened_by(&open(&fixture)),
            Some(ContentCase::Fixture(name, Family::Pictures)) if name == fixture
        ));
        assert!(ContentCase::opened_by(&open(&code)).is_none());
        assert!(ContentCase::opened_by(&SessionEnd::Quit).is_none());
    }

    #[test]
    fn a_fixture_shows_its_repository_path_and_has_nothing_to_promote_or_reject() {
        let fixture = "bmp-x-talamus-solarize-12x29-psf-8a856fdb-solarize-12x29";
        let path = readme_file(&human_content::root(Family::Pictures).join(fixture));
        assert_eq!(path.as_deref(), Some("Solarize.12x29.bmp"));

        let dir = tempfile::tempdir().unwrap();
        let mut session = session_over(
            dir.path(),
            fixture,
            Origin::Fixture(fixture.to_string()),
            &path.unwrap(),
        );
        let mut app = test_app();
        let text = screen(&mut session, &app, 160);
        assert!(text.contains("(fixture, Solarize.12x29.bmp)"), "{text}");
        assert!(
            text.contains("0-5 level  tags  u can't judge  s save  t view"),
            "{text}"
        );
        assert!(!text.contains("reject"), "{text}");

        handle_key(&mut session, &mut app, KeyCode::Char('x'));
        assert!(session.reject_input.is_none());
        assert_eq!(session.status, "A fixture cannot be rejected");
    }

    #[test]
    fn the_session_shows_the_verdicts_and_nothing_the_engine_decided() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = session_over(
            dir.path(),
            "png-x-repo-1234abcd-logo",
            sample_origin(),
            "assets/logo.png",
        );
        session.judgement.pair = Some(Judgement::Verdict(Verdict::new(Level::Edited)));
        let app = test_app();
        let screen_text = screen(&mut session, &app, 160);
        assert!(
            screen_text.contains("png-x-repo-1234abcd-logo"),
            "{screen_text}"
        );
        assert!(screen_text.contains("PNG 8x4 RGBA8"), "{screen_text}");
        assert!(screen_text.contains("0 invisible"), "{screen_text}");
        assert!(screen_text.contains("2 artifacts"), "{screen_text}");
        assert!(screen_text.contains("5 replaced"), "{screen_text}");
        assert!(screen_text.contains("r resized"), "{screen_text}");
        assert!(screen_text.contains("d timing"), "{screen_text}");
        assert!(
            !screen_text.contains("m members"),
            "a picture has no members: {screen_text}"
        );
        assert!(screen_text.contains("u can't judge"), "{screen_text}");
        assert!(screen_text.contains("(unsaved)"), "{screen_text}");
        assert!(
            !screen_text.contains("changed"),
            "no engine verdict on screen: {screen_text}"
        );

        session.viewer.set_annotating(false);
        let screen_text = screen(&mut session, &app, 160);
        assert!(screen_text.contains("omnidiff: edited"), "{screen_text}");
        assert!(screen_text.contains("of pixels changed"), "{screen_text}");
    }

    /// A session over two zips in `dir`: `a.txt` and `b.txt` edited, `c.txt` added, `same.txt`
    /// unchanged.
    fn archive_session(dir: &Path) -> ContentSession {
        let zip = |files: &[(&str, &str)]| {
            let mut bytes = Vec::new();
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            for (name, text) in files {
                writer
                    .start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                std::io::Write::write_all(&mut writer, text.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
            bytes
        };
        let (a, b) = (dir.join("before.zip.test"), dir.join("after.zip.test"));
        fs::write(
            &a,
            zip(&[
                ("a.txt", "1\n2\n3\n"),
                ("b.txt", "x\n"),
                ("same.txt", "s\n"),
            ]),
        )
        .unwrap();
        fs::write(
            &b,
            zip(&[
                ("a.txt", "1\n2\nthree\n"),
                ("b.txt", "y\n"),
                ("c.txt", "new\n"),
                ("same.txt", "s\n"),
            ]),
        )
        .unwrap();
        ContentSession {
            name: "zip-x-repo-1234abcd-bundle".to_string(),
            family: Family::Archives,
            origin: sample_origin(),
            path: "bundle.zip".to_string(),
            viewer: ContentViewer::open_for_annotation(&a, &b, Picker::halfblocks()).unwrap(),
            judgement: Judgements::default(),
            saved: Judgements::default(),
            last: None,
            reject_input: None,
            note_input: None,
            quit_armed: false,
            status: String::new(),
        }
    }

    /// A gettext `.mo` of `messages`, (original, translation) pairs - the layout
    /// `diff::content::catalog` reads.
    fn mo(messages: &[(&str, &str)]) -> Vec<u8> {
        let count = messages.len();
        let data_start = 28 + count * 16;
        let (mut strings, mut tables) = (Vec::new(), [Vec::new(), Vec::new()]);
        for (original, translation) in messages {
            for (table, text) in [(0, original), (1, translation)] {
                let offset = data_start + strings.len();
                tables[table].extend_from_slice(&(text.len() as u32).to_le_bytes());
                tables[table].extend_from_slice(&(offset as u32).to_le_bytes());
                strings.extend_from_slice(text.as_bytes());
                strings.push(0);
            }
        }
        let header = [
            0x9504_12de_u32,
            0,
            count as u32,
            28,
            28 + count as u32 * 8,
            0,
            0,
        ];
        let mut bytes: Vec<u8> = header.iter().flat_map(|word| word.to_le_bytes()).collect();
        bytes.extend(tables.concat());
        bytes.extend(strings);
        bytes
    }

    #[test]
    fn a_catalog_names_its_levels_for_text_on_the_pair_and_every_message() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (
            dir.path().join("before.mo.test"),
            dir.path().join("after.mo.test"),
        );
        fs::write(&a, mo(&[("Open", "Ouvrir"), ("Save", "Enregistrer")])).unwrap();
        fs::write(&b, mo(&[("Open", "Ouvrir..."), ("Save", "Sauver")])).unwrap();
        let mut session = ContentSession {
            name: "mo-x-repo-1234abcd-fr".to_string(),
            family: Family::Catalogs,
            path: "fr.mo".to_string(),
            viewer: ContentViewer::open_for_annotation(&a, &b, Picker::halfblocks()).unwrap(),
            ..archive_session(dir.path())
        };
        let mut app = test_app();
        let names_text = |text: &str| text.contains("2 formatting") && text.contains("4 rewritten");

        let text = screen(&mut session, &app, 200);
        assert!(names_text(&text), "the whole catalog is text: {text}");
        handle_key(&mut session, &mut app, KeyCode::Char('4'));
        assert!(session.status.contains("rewritten"), "{}", session.status);

        handle_key(&mut session, &mut app, KeyCode::Char('j'));
        assert!(
            names_text(&screen(&mut session, &app, 200)),
            "and so is a message"
        );
        session.viewer.set_annotating(false);
        let text = screen(&mut session, &app, 200);
        assert!(
            !text.contains("redrawn"),
            "nor does omnidiff's verdict say redrawn: {text}"
        );
    }

    #[test]
    fn a_container_takes_a_verdict_per_changed_member_and_one_for_the_pair() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = archive_session(dir.path());
        let mut app = test_app();

        let verdict = |text: &str| Some(Judgement::Verdict(text.parse().unwrap()));
        handle_key(&mut session, &mut app, KeyCode::Char('1'));
        assert_eq!(
            session.judgement.pair,
            verdict("imperceptible"),
            "the pair row"
        );
        handle_key(&mut session, &mut app, KeyCode::Char('m'));
        assert_eq!(
            session.judgement.pair,
            verdict("imperceptible+members"),
            "a container takes the members tag"
        );
        handle_key(&mut session, &mut app, KeyCode::Char('r'));
        assert!(
            session.status.contains("a container is never resized"),
            "{}",
            session.status
        );
        handle_key(&mut session, &mut app, KeyCode::Char('2'));
        assert_eq!(
            session.judgement.pair,
            verdict("artifacts+members"),
            "a new level keeps the tags"
        );

        handle_key(&mut session, &mut app, KeyCode::Char('n'));
        handle_key(&mut session, &mut app, KeyCode::Char('3'));
        assert_eq!(
            session.judgement.members.get("a.txt").cloned(),
            verdict("edited")
        );
        handle_key(&mut session, &mut app, KeyCode::Char('n'));
        handle_key(&mut session, &mut app, KeyCode::Char('r'));
        assert!(
            !session.judgement.members.contains_key("b.txt"),
            "text cannot be resized"
        );
        assert!(
            session.status.contains("text or bytes is never resized"),
            "{}",
            session.status
        );

        // The added member takes none.
        handle_key(&mut session, &mut app, KeyCode::Char('j'));
        handle_key(&mut session, &mut app, KeyCode::Char('4'));
        assert!(
            session.status.contains("Only a changed member"),
            "{}",
            session.status
        );

        handle_key(&mut session, &mut app, KeyCode::Char('A'));
        assert_eq!(
            session.judgement.members.get("b.txt").cloned(),
            verdict("edited"),
            "A repeats the last verdict given"
        );
        let human = human_content_of(&session);
        assert_eq!(human.unjudged, 0);
        assert!(human.is_complete());

        let text = screen(&mut session, &app, 160);
        assert!(text.contains("2/2 members judged"), "{text}");
        assert!(text.contains("a.txt  edited"), "{text}");
        assert!(text.contains("this member (it takes none)"), "{text}");
        assert!(
            !text.contains("(omnidiff:"),
            "nothing the engine decided: {text}"
        );
        session.viewer.set_annotating(false);
        update_marks(&mut session);
        let text = screen(&mut session, &app, 160);
        assert!(
            text.contains("b.txt  edited (omnidiff: replaced)"),
            "{text}"
        );

        // b.txt again, and can't judge it, with a note.
        handle_key(&mut session, &mut app, KeyCode::Char('k'));
        let text = screen(&mut session, &app, 160);
        assert!(
            text.contains("1 whitespace")
                && text.contains("2 formatting")
                && text.contains("4 rewritten"),
            "a text member's levels go by their text names: {text}"
        );
        handle_key(&mut session, &mut app, KeyCode::Char('u'));
        for key in "blank".chars() {
            handle_key(&mut session, &mut app, KeyCode::Char(key));
        }
        assert!(
            screen(&mut session, &app, 160).contains("Can't judge, because OmniDiff: blank_"),
            "the note is typed on the prompt line"
        );
        handle_key(&mut session, &mut app, KeyCode::Enter);
        assert_eq!(
            session.judgement.members.get("b.txt"),
            Some(&Judgement::CantJudge("blank".to_string()))
        );
        assert!(
            screen(&mut session, &app, 160).contains("b.txt  can't judge: blank"),
            "and is no disagreement with the engine"
        );
    }

    #[test]
    fn a_container_stub_pins_each_disagreement() {
        let agreeing = stub_contents(Family::Archives, "zip-x-a-b-c", &Mismatches::default());
        assert!(agreeing.contains(
            "human_content::assert_matches_human_verdicts(Family::Archives, \"zip-x-a-b-c\")"
        ));
        assert!(agreeing.contains("human_content::{self, Family};"));

        let pinned = stub_contents(
            Family::Archives,
            "zip-x-a-b-c",
            &Mismatches {
                pair: Some(Verdict::new(Level::Replaced).with(Tag::Members)),
                members: vec![
                    (
                        "dir/\"q\".txt".to_string(),
                        Some(Verdict::new(Level::Edited)),
                    ),
                    ("gone".to_string(), None),
                ],
            },
        );
        assert!(pinned.contains("Some(\"replaced+members\")"), "{pinned}");
        assert!(
            pinned.contains(r#"&[("dir/\"q\".txt", Some("edited")), ("gone", None)]"#),
            "{pinned}"
        );
        assert!(pinned.contains("Recorded as found"), "{pinned}");
    }
}
