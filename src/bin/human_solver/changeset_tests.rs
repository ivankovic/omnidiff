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
//! The changeset session driven by keys, as a human drives it: [`Driver`] is the loop of
//! `changeset::run` and `run_case_session` without a terminal, drawing on a `TestBackend`.
//!
//! The `solve_*` functions at the end are the sessions that solved three handmade changesets,
//! kept as the record of how their truth was made: a test replays each on a scratch copy and
//! holds the result to the committed file, and the ignored `solve_the_handmade_changesets_in_place`
//! (`cargo test --all-features --bin human_solver solve_the_handmade -- --ignored`) writes the
//! truths and stubs into the repository again.

use super::*;
use crate::CaseOrigin;
use crate::events::{compute_frame_state, session_key};
use crossterm::event::{KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;

struct Driver {
    app: App,
    codes: Option<(Arc<Code>, Arc<Code>)>,
    /// Holds the copy the session works on, when it is not the repository's own directory.
    _scratch: Option<tempfile::TempDir>,
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

impl Driver {
    fn over(session: Session, scratch: Option<tempfile::TempDir>) -> Self {
        let mut app = App::new(
            session.name.clone(),
            CaseOrigin::Diffs,
            usize::MAX,
            usize::MAX,
            HumanMapping::default(),
        );
        app.changeset = Some(Box::new(session));
        Driver {
            app,
            codes: None,
            _scratch: scratch,
        }
    }

    /// A copy of the handmade changeset `name` in a scratch directory, as a sample
    /// (`samples/changesets/<name>`) or as a corpus changeset (`changesets/handmade/<name>`), with
    /// its truth removed unless `keep_truth`. Stubs and promotions land in the scratch directory.
    fn scratch(name: &str, keep_truth: bool, as_sample: bool) -> Self {
        let scratch = tempfile::tempdir().unwrap();
        let corpus = scratch.path().join("changesets");
        let dir = if as_sample {
            scratch.path().join("samples").join("changesets").join(name)
        } else {
            corpus.join("handmade").join(name)
        };
        copy_dir(&changeset_dir(name).unwrap(), &dir);
        if !keep_truth {
            let _ = fs::remove_file(dir.join(TRUTH_FILE_NAME));
        }
        let mut session = Session::open(&dir, as_sample).unwrap();
        session.corpus_root = corpus;
        session.fixtures_root = scratch.path().join("fixtures");
        Driver::over(session, Some(scratch))
    }

    fn session(&self) -> &Session {
        self.app.changeset.as_ref().unwrap()
    }

    fn open_pair(&mut self, mut request: PairRequest) {
        loop {
            match enter_pair(&mut self.app, &request) {
                Ok(codes) => self.codes = Some(codes),
                Err(error) => {
                    leave_pair(&mut self.app);
                    self.app.changeset.as_mut().unwrap().status =
                        format!("Cannot open the pair: {error:#}");
                    self.codes = None;
                    return;
                }
            }
            let mut next = None;
            while let Some(code) = take_queued_key(&mut self.app) {
                next = self.pair_press(code);
                if self.codes.is_none() {
                    break;
                }
            }
            match next {
                Some(following) => request = following,
                None => return,
            }
        }
    }

    /// One key in the open pair view. Returns the pair to open next, if the key asked for one.
    fn pair_press(&mut self, code: KeyCode) -> Option<PairRequest> {
        let (before, after) = self.codes.clone().unwrap();
        let target = {
            let frame = compute_frame_state(&before, &after, &self.app).unwrap();
            let key = KeyEvent::new(code, KeyModifiers::NONE);
            session_key(&mut self.app, key, &frame, &before, &after)
                .unwrap()
                .0
        };
        let target = target?;
        leave_pair(&mut self.app);
        self.codes = None;
        match route(&mut self.app, SessionEnd::Open(target)) {
            Next::Board => None,
            Next::Pair(request) => Some(request),
        }
    }

    fn press(&mut self, code: KeyCode) {
        if self.codes.is_some() {
            if let Some(request) = self.pair_press(code) {
                self.open_pair(request);
            }
            return;
        }
        if let BoardEnd::Open(request) = board_key(&mut self.app, code) {
            self.open_pair(request);
        }
    }

    /// Each character as a key; `\n` is Enter, `\t` Tab, `\x1b` Esc and `\x08` Backspace.
    fn keys(&mut self, keys: &str) {
        for key in keys.chars() {
            self.press(match key {
                '\n' => KeyCode::Enter,
                '\t' => KeyCode::Tab,
                '\x1b' => KeyCode::Esc,
                '\x08' => KeyCode::Backspace,
                other => KeyCode::Char(other),
            });
        }
    }

    fn status(&self) -> String {
        if self.codes.is_some() {
            self.app.status.clone().unwrap_or_default()
        } else {
            self.session().status.clone()
        }
    }

    fn screen(&mut self, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        match self.codes.clone() {
            None => {
                terminal
                    .draw(|frame| draw_board(frame, &mut self.app))
                    .unwrap();
            }
            Some((before, after)) => {
                let frame_state = compute_frame_state(&before, &after, &self.app).unwrap();
                let name = self.app.display_name();
                terminal
                    .draw(|frame| {
                        crate::render::draw_ui(
                            frame,
                            &mut self.app,
                            &frame_state.before_flat,
                            &frame_state.after_flat,
                            &frame_state.caches,
                            frame_state.before_src,
                            frame_state.after_src,
                            frame_state.before_unmarked,
                            frame_state.after_unmarked,
                            frame_state.before_mismatches,
                            frame_state.after_mismatches,
                            &name,
                            frame_state.roots().is_none(),
                        )
                    })
                    .unwrap();
            }
        }
        let buffer = terminal.backend().buffer().clone();
        let mut text = String::new();
        for row in 0..height {
            for column in 0..width {
                text.push_str(buffer[(column, row)].symbol());
            }
            text.push('\n');
        }
        text
    }

    /// Saves from the board, then loads what is on disk and checks it.
    fn saved(&mut self) -> (HumanChangeset, Vec<ChangesetViolation>) {
        assert!(self.codes.is_none(), "save from the board");
        self.keys("s");
        let session = self.session();
        let loaded = HumanChangeset::load(&session.dir, &session.trees)
            .unwrap()
            .expect("s writes the truth");
        let violations = validate(&loaded, &session.trees);
        (loaded, violations)
    }
}

/// The match entries between `before` and `after`, as a set.
fn entries_between(truth: &HumanChangeset, before: &str, after: &str) -> BTreeSet<String> {
    truth
        .entries
        .iter()
        .filter_map(|entry| {
            let (b, a) = (entry.before.as_ref()?, entry.after.as_ref()?);
            (b.file == before && a.file == after)
                .then(|| format!("{:?} {:?} {:?}", entry.operation, b.path, a.path))
        })
        .collect()
}

fn repository_truth(name: &str) -> (HumanChangeset, String) {
    let changeset = Changeset::load_by_name(name).unwrap();
    let json = fs::read_to_string(changeset.dir.join(TRUTH_FILE_NAME)).unwrap();
    (changeset.truth.unwrap(), json)
}

const BACKSPACES: &str = "\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08";

// ─── The board ──────────────────────────────────────────────────────────────────────────────

#[test]
fn a_sample_without_a_truth_opens_unsolved_with_dim_proposals_that_are_not_saved() {
    let mut driver = Driver::scratch("python-function-moved-and-renamed", false, true);
    assert!(driver.session().truth.files.is_empty());
    assert_eq!(driver.session().proposals.len(), 2);
    let screen = driver.screen(200, 12);
    assert!(screen.contains("[changeset sample]"), "{screen}");
    assert!(screen.contains("? same (proposed)"), "{screen}");
    assert!(screen.contains("Python"), "{screen}");
    assert!(screen.contains("11 lines"), "{screen}");
    assert!(screen.contains("95 unmarked"), "{screen}");

    // Saving writes what is confirmed, which is nothing: a proposal is not truth.
    let (truth, violations) = driver.saved();
    assert!(truth.files.is_empty() && truth.entries.is_empty());
    assert!(violations.is_empty());
    assert!(driver.session().is_sample);
    assert!(
        driver.status().contains("not confirmed"),
        "{}",
        driver.status()
    );
    assert!(
        driver.status().contains("still a sample"),
        "{}",
        driver.status()
    );
}

#[test]
fn proposals_are_confirmed_changed_or_rejected() {
    let mut driver = Driver::scratch("typescript-file-renamed-unedited", false, false);
    // git's rename hint is proposed, dim; `c` confirms the focused file's.
    let screen = driver.screen(200, 8);
    assert!(
        screen.contains("? renamed -> src/format.ts (proposed)"),
        "{screen}"
    );
    driver.keys("c");
    assert_eq!(driver.session().truth.files.len(), 1);
    assert_eq!(
        driver.session().truth.files[0].relation,
        FileRelationKind::Renamed
    );
    assert_eq!(driver.session().proposals.len(), 1);
    // `u` rejects the other proposal; `p` pairs the files by hand, `same` because the paths agree.
    driver.keys("ju");
    assert!(driver.session().proposals.is_empty());
    driver.keys("\tjp");
    let listing = &driver.session().truth.files[1];
    assert_eq!(listing.relation, FileRelationKind::Same);
    assert_eq!(listing.before.as_deref(), Some("src/listing.ts"));
    // A file takes one counterpart through `p`.
    driver.keys("kp");
    assert!(
        driver.status().contains("already has a relation"),
        "{}",
        driver.status()
    );
    // `x` toggles `solved` on the relation between the two cursors.
    driver.keys("jx");
    assert!(driver.session().truth.files[1].solved);
    assert!(driver.screen(200, 12).contains("[x] same"));
    driver.keys("x");
    assert!(!driver.session().truth.files[1].solved);
    // `f` puts the other cursor on the counterpart.
    driver.keys("\tgf");
    assert_eq!(
        driver.session().selected(AFTER).as_deref(),
        Some("src/format.ts")
    );
}

#[test]
fn split_and_merged_are_marked_one_counterpart_at_a_time() {
    let mut driver = Driver::scratch("python-file-split", false, false);
    driver.keys("S");
    assert!(
        driver.status().contains("press the key again"),
        "{}",
        driver.status()
    );
    driver.keys("\tjS");
    let kinds = |driver: &Driver| -> Vec<(FileRelationKind, String)> {
        driver
            .session()
            .truth
            .files
            .iter()
            .map(|relation| (relation.relation, relation.after.clone().unwrap()))
            .collect()
    };
    assert_eq!(
        kinds(&driver),
        [
            (FileRelationKind::Split, "inventory.py".to_string()),
            (FileRelationKind::Split, "rendering.py".to_string())
        ]
    );
    assert!(
        driver.session().proposals.is_empty(),
        "S replaces the proposals"
    );
    driver.keys("V");
    assert!(driver.status().contains("validates"), "{}", driver.status());
    // Unpairing one counterpart leaves a plain pair.
    driver.keys("u");
    assert_eq!(
        kinds(&driver),
        [(FileRelationKind::Same, "inventory.py".to_string())]
    );
    // `a` marks the other file added.
    driver.keys("a");
    assert_eq!(
        driver.session().truth.files[1].relation,
        FileRelationKind::Added
    );

    let mut driver = Driver::scratch("go-files-merged", false, false);
    driver.keys("MjM");
    let merged: Vec<_> = driver
        .session()
        .truth
        .files
        .iter()
        .map(|relation| (relation.relation, relation.before.clone().unwrap()))
        .collect();
    assert_eq!(
        merged,
        [
            (FileRelationKind::Merged, "cache/get.go".to_string()),
            (FileRelationKind::Merged, "cache/put.go".to_string())
        ]
    );
    let (_, violations) = driver.saved();
    assert!(violations.is_empty(), "{violations:?}");
    // `d` on a related file is refused; unpaired first, it is marked removed.
    driver.keys("d");
    assert!(
        driver.status().contains("already has a relation"),
        "{}",
        driver.status()
    );
    driver.keys("\tud");
    assert!(
        driver
            .session()
            .truth
            .files
            .iter()
            .any(|relation| relation.relation == FileRelationKind::Removed)
    );
}

#[test]
fn a_pair_without_trees_takes_a_verdict() {
    let mut driver = Driver::scratch("python-asset-moved-with-code-change", false, false);
    // Unconfirmed, a pair has nothing to hang a verdict on.
    driver.keys("j\tj\n");
    assert!(
        driver.status().contains("pair them first"),
        "{}",
        driver.status()
    );
    driver.keys("C\n");
    assert!(matches!(
        driver.session().overlay,
        Some(Overlay::Verdict { .. })
    ));
    let screen = driver.screen(160, 30);
    assert!(
        screen.contains("site/img/logo.png -> site/static/logo.png"),
        "{screen}"
    );
    driver.keys("0x");
    let relation = &driver.session().truth.files[1];
    assert_eq!(
        relation.verdict,
        Some(Judgement::Verdict(Verdict::new(Level::ALL[0])))
    );
    assert!(relation.solved);
    let (_, violations) = driver.saved();
    assert!(violations.is_empty(), "{violations:?}");
    // Two files with trees take no verdict.
    driver.keys("k\tke");
    assert!(
        driver.status().contains("both files have trees"),
        "{}",
        driver.status()
    );
}

// ─── The pair view ──────────────────────────────────────────────────────────────────────────

/// The spec's whole path, by hand: pair the files on the board, seed the two files' own pairs,
/// pin the function that left, match it across files with `Y`, paint, save, and load it back.
#[test]
fn a_function_is_pinned_matched_across_files_painted_saved_and_loaded_back() {
    let mut driver = Driver::scratch("python-function-moved-and-renamed", false, false);
    // codec.py with itself first: its own nodes from the engine, the new function left undecided.
    driver.keys("p\nSbj\tjp");
    assert_eq!(driver.session().truth.files.len(), 2);
    assert!(driver.session().proposals.is_empty());

    // handlers.py with itself: the engine's matches, then the function is pinned.
    driver.keys("\n");
    assert!(driver.status().contains("S seeds"), "{}", driver.status());
    assert!(
        driver
            .app
            .display_name()
            .contains("before app/handlers.py -> after app/handlers.py (same)"),
        "{}",
        driver.app.display_name()
    );
    driver.keys("S/parse_body\nhy");
    assert!(
        driver
            .status()
            .starts_with("Pinned before function_definition"),
        "{}",
        driver.status()
    );
    driver.keys("g]D\t]I");
    // Any before file beside any after file: handlers.py beside codec.py, which no relation pairs.
    driver.keys("bk\n");
    assert!(driver.app.display_name().contains("(no relation)"));
    assert!(
        driver
            .app
            .display_name()
            .contains("[pinned: before function_definition")
    );
    driver.keys("S");
    assert!(
        driver.status().contains("same or renamed pair only"),
        "{}",
        driver.status()
    );
    driver.keys(&format!("\t/{BACKSPACES}decode_body\nhY"));
    assert!(
        driver.status().starts_with("Matched 31 node pair(s)"),
        "{}",
        driver.status()
    );
    let cross = entries_between(&driver.session().truth, "app/handlers.py", "app/codec.py");
    assert_eq!(cross.len(), 31);

    // Painted between the two files: the name is an update by word, the rest moves.
    driver.keys("t:4\nvlll\t:8\nvlllm");
    driver.keys("lvlllll\tlvllllm");
    driver.keys("lv$\tlv$m");
    driver.keys("j^v$\tj^v$m\x1bb");
    // From inside handlers.py the function reads as gone, and is drawn as matched elsewhere.
    driver.keys("j\n");
    let screen = driver.screen(200, 30);
    assert!(
        screen.contains("~ function_definition  ~> app/codec.py:8"),
        "{screen}"
    );
    driver.keys("t:1\nv$d\t:1\nv$i");
    driver.keys("\t:9\n^wwuvllll\t:5\n^wwvlllllm\x1bb");
    driver.keys("xk\tkx");

    let (loaded, violations) = driver.saved();
    assert_eq!(violations, [], "{}", driver.status());
    let session = driver.session();
    assert!(loaded.is_solved(&session.trees));
    let on_disk = fs::read_to_string(session.dir.join(TRUTH_FILE_NAME)).unwrap();
    assert_eq!(
        on_disk,
        loaded.to_json(&session.trees).unwrap(),
        "saving is a fixed point"
    );
    assert_eq!(
        on_disk,
        session.truth.to_json(&session.trees).unwrap(),
        "what was saved is what is in memory"
    );
    let painted = &loaded.text_mappings[0];
    assert_eq!(painted.name, "Minimal");
    let between_files = painted
        .entries
        .iter()
        .filter(|entry| {
            entry
                .before
                .iter()
                .any(|span| span.file == "app/handlers.py")
                && entry.after.iter().any(|span| span.file == "app/codec.py")
        })
        .count();
    assert_eq!(between_files, 4);
    // Solved, so the stub is written beside the others (here: in the scratch directory).
    let stub = fs::read_to_string(session.stub_path()).unwrap();
    assert!(stub.contains("assert_solved_ground_truth(\"python-function-moved-and-renamed\")"));
    let mods = fs::read_to_string(session.fixtures_root.join("changesets.rs")).unwrap();
    assert!(
        mods.contains("mod python_function_moved_and_renamed;"),
        "{mods}"
    );

    // Unsolved again, the stub goes: only a solved changeset has one.
    driver.keys("xs");
    assert!(
        driver.status().contains("stub removed"),
        "{}",
        driver.status()
    );
    assert!(!driver.session().stub_path().exists());
}

#[test]
fn a_node_matched_in_another_file_gains_a_partner_only_through_plus() {
    let mut driver = Driver::scratch("rust-function-copied", false, false);
    // units.rs is untouched: its two sides match whole.
    driver.keys("Cj\tj\nMb");
    // Beside report.rs, `round_to` in units.rs is matched elsewhere, not unmatched.
    driver.keys("k\n/round_to\nh\t/\nh");
    let original = ["function_item:1".to_string()];
    let partners = |driver: &Driver| {
        driver
            .session()
            .partners_of(BEFORE, "src/units.rs", &original)
            .iter()
            .map(Partner::place)
            .collect::<Vec<_>>()
    };
    let screen = driver.screen(200, 30);
    assert!(
        screen.contains("~ function_item  ~> src/units.rs:1"),
        "{screen}"
    );
    assert_eq!(partners(&driver), ["src/units.rs:1"]);

    // `m`, the key that replaces a node's partner, changes nothing here; nor does a delete.
    for key in ["m", "M", "\td\t", "\tD\t"] {
        driver.keys(key);
        assert!(
            driver.status().contains("partner") || driver.status().contains("matched in another"),
            "{key}: {}",
            driver.status()
        );
        assert_eq!(partners(&driver), ["src/units.rs:1"], "after {key}");
    }
    assert!(entries_between(&driver.session().truth, "src/units.rs", "src/report.rs").is_empty());

    // `+` adds the copy as a second partner: the entries the hand-written truth has.
    driver.keys("+");
    assert!(driver.status().starts_with("Added"), "{}", driver.status());
    assert!(
        driver.status().contains("2 partner(s)"),
        "{}",
        driver.status()
    );
    assert_eq!(partners(&driver), ["src/report.rs:6", "src/units.rs:1"]);
    let (written, _) = repository_truth("rust-function-copied");
    assert_eq!(
        entries_between(&driver.session().truth, "src/units.rs", "src/report.rs"),
        entries_between(&written, "src/units.rs", "src/report.rs")
    );
    // Here the copy is the pair's match, and the original's own file is the further partner.
    driver.keys("\t/\nh");
    let screen = driver.screen(200, 30);
    assert!(
        screen.contains("M function_item  +> src/units.rs:1"),
        "{screen}"
    );
    driver.keys("+");
    assert!(
        driver.status().contains("already matched"),
        "{}",
        driver.status()
    );

    // `-` lists the partners and removes the selected one, with its subtree's entries.
    driver.keys("-");
    assert!(
        matches!(driver.app.modal, Some(Modal::Partners { ref partners, .. }) if partners.len() == 2)
    );
    let screen = driver.screen(200, 30);
    assert!(screen.contains("after src/report.rs:6"), "{screen}");
    driver.keys("-");
    assert!(
        driver
            .status()
            .starts_with("Removed the partner at src/report.rs:6"),
        "{}",
        driver.status()
    );
    assert_eq!(partners(&driver), ["src/units.rs:1"]);
    assert!(entries_between(&driver.session().truth, "src/units.rs", "src/report.rs").is_empty());
    // Enter in the list opens the pair view on the partner's file.
    driver.keys("-\n");
    assert!(
        driver
            .app
            .display_name()
            .contains("before src/units.rs -> after src/units.rs")
    );
}

#[test]
fn seeding_takes_the_engines_matches_and_leaves_the_rest_undecided() {
    let mut driver = Driver::scratch("python-function-moved-and-renamed", false, false);
    driver.keys("Cj\tj\nS");
    assert!(
        driver
            .status()
            .starts_with("Seeded 60 entr(ies) and 1 painted range(s)"),
        "{}",
        driver.status()
    );
    // What a single-file diff calls deleted may have moved to another file: it stays undecided.
    assert!(
        driver
            .session()
            .truth
            .entries
            .iter()
            .all(|entry| is_match(entry.operation))
    );
    let screen = driver.screen(200, 30);
    assert!(screen.contains("35 unmarked"), "{screen}");
    // A seed is a draft for an empty pair, never laid over work; `U` takes it back.
    driver.keys("S");
    assert!(
        driver.status().contains("already has entries"),
        "{}",
        driver.status()
    );
    driver.press(KeyCode::Char('U'));
    assert!(
        driver.session().truth.entries.is_empty(),
        "{}",
        driver.status()
    );
}

#[test]
fn plus_fills_a_changed_subtree_as_capital_m_would() {
    let mut driver = Driver::scratch("python-function-moved-and-renamed", false, false);
    driver.keys(&format!("j\n/parse_body\nh\t/{BACKSPACES}decode_body\nh+"));
    assert!(
        driver
            .status()
            .starts_with("Added 31 entr(ies) as MatchButNotIdentical"),
        "{}",
        driver.status()
    );
    let cross = entries_between(&driver.session().truth, "app/handlers.py", "app/codec.py");
    assert!(
        cross.iter().any(|entry| entry.starts_with("Update")),
        "{cross:#?}"
    );
    // Different kinds are a question `m` asks; `+` does not guess.
    driver.keys("j+");
    assert!(
        driver.status().contains("the kinds differ"),
        "{}",
        driver.status()
    );
}

// ─── The leftover pool ──────────────────────────────────────────────────────────────────────

#[test]
fn the_pool_pairs_hash_twins_across_files_and_accepting_one_is_the_written_truth() {
    let mut driver = Driver::scratch("rust-function-moved-intact", false, false);
    driver.keys("C\nSbj\tj\nSbL");
    let Some(Overlay::Pool { rows, .. }) = &driver.session().overlay else {
        panic!("L opens the pool");
    };
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let (before, after) = (
        rows[0].before.as_ref().unwrap(),
        rows[0].after.as_ref().unwrap(),
    );
    assert_eq!((before.file.as_str(), before.line), ("src/geometry.rs", 10));
    assert_eq!((after.file.as_str(), after.line), ("src/measure.rs", 7));
    assert_eq!(before.kind, "function_item");
    let screen = driver.screen(220, 20);
    assert!(
        screen.contains("= before src/geometry.rs:10 function_item"),
        "{screen}"
    );
    assert!(screen.contains("<=>  after src/measure.rs:7"), "{screen}");
    // A candidate is a suggestion: nothing is matched until `a`.
    assert!(
        entries_between(&driver.session().truth, "src/geometry.rs", "src/measure.rs").is_empty()
    );

    driver.keys("a");
    assert!(
        driver
            .status()
            .starts_with("Matched before src/geometry.rs:10"),
        "{}",
        driver.status()
    );
    let (written, _) = repository_truth("rust-function-moved-intact");
    let accepted = entries_between(&driver.session().truth, "src/geometry.rs", "src/measure.rs");
    assert!(!accepted.is_empty());
    assert_eq!(
        accepted,
        entries_between(&written, "src/geometry.rs", "src/measure.rs")
    );
    // With the seeds, that is the whole of the hand-written node truth.
    let all = |truth: &HumanChangeset| -> BTreeSet<String> {
        truth
            .entries
            .iter()
            .map(|entry| format!("{entry:?}"))
            .collect()
    };
    assert_eq!(all(&driver.session().truth), all(&written));
    assert!(
        matches!(&driver.session().overlay, Some(Overlay::Pool { rows, .. }) if rows.is_empty())
    );
}

#[test]
fn the_pool_lists_outermost_leftovers_by_size_and_opens_two_of_them_side_by_side() {
    let mut driver = Driver::scratch("python-function-moved-and-renamed", false, false);
    driver.keys("C\nSbj\tj\nSbL");
    let lines = |driver: &Driver| -> Vec<String> {
        let Some(Overlay::Pool { rows, .. }) = &driver.session().overlay else {
            panic!("the pool is open");
        };
        rows.iter().map(pool_line).collect()
    };
    let by_size = lines(&driver);
    assert_eq!(by_size.len(), 4, "{by_size:#?}");
    assert!(
        by_size[0].contains("before app/handlers.py:4 function_definition 2L/31n [?]"),
        "{by_size:#?}"
    );
    assert!(
        by_size[1].contains("after app/codec.py:8 function_definition 2L/31n [?]"),
        "{by_size:#?}"
    );
    // Renamed, so no twin: the two are not offered as a candidate.
    assert!(by_size.iter().all(|line| !line.starts_with('=')));
    driver.keys("s");
    let by_file = lines(&driver);
    assert!(
        by_file[0].contains("before app/handlers.py:1 import_statement"),
        "{by_file:#?}"
    );
    // A deleted subtree is still a leftover, and says so.
    driver.keys("\x1b\n]D");
    driver.keys("bL");
    assert!(
        lines(&driver)
            .iter()
            .any(|line| line.contains("import_statement 1L/4n [-]"))
    );

    // `y` pins one subtree; Enter on one of the other side opens the two files at the two nodes.
    driver.keys("yj\n");
    assert!(
        driver
            .app
            .display_name()
            .contains("before app/handlers.py -> after app/codec.py")
    );
    driver.keys("M");
    assert!(
        driver.status().starts_with("Matched 31 node pair(s)"),
        "{}",
        driver.status()
    );
}

// ─── Saving, promotion, reopening ───────────────────────────────────────────────────────────

#[test]
fn a_solved_changeset_reopens_with_its_state_and_saves_byte_for_byte() {
    for name in ["rust-function-moved-intact", "rust-function-copied"] {
        let mut driver = Driver::scratch(name, true, false);
        let screen = driver.screen(200, 10);
        assert!(screen.contains("[x] same"), "{screen}");
        assert!(screen.contains("all marked"), "{screen}");
        assert!(
            screen.contains("2 relation(s), 2 solved, 0 proposed"),
            "{screen}"
        );
        let (_, violations) = driver.saved();
        assert!(violations.is_empty());
        // The tool's writer is the format's: an untouched truth is written back as it was.
        let (_, written) = repository_truth(name);
        let saved = fs::read_to_string(driver.session().dir.join(TRUTH_FILE_NAME)).unwrap();
        assert_eq!(saved, written, "{name}");
        assert!(driver.status().contains("solved"), "{}", driver.status());
    }

    let mut driver = Driver::scratch("rust-function-moved-intact", true, false);
    driver.keys("\n");
    let screen = driver.screen(200, 100);
    assert!(screen.contains("0 unmarked"), "{screen}");
    assert!(
        screen.contains("~ function_item  ~> src/measure.rs:7"),
        "{screen}"
    );
    // Moving onto it names the partner on the status line.
    driver.keys("G");
    assert!(
        driver.status().contains("matched to src/measure.rs:9"),
        "{}",
        driver.status()
    );
    // Its painting belongs to the pair geometry.rs -> measure.rs: here it is drawn, not edited.
    driver.keys("t");
    assert!(
        driver
            .app
            .mapping
            .text_mappings
            .iter()
            .all(|named| named.mapping.entries.is_empty())
    );
    let foreign = driver.app.layer_paint[0].for_solution("Minimal").unwrap();
    assert_eq!(foreign[BEFORE].len(), 3);
    assert!(
        foreign[BEFORE]
            .iter()
            .all(|(_, verdict)| *verdict == HumanTextVerdict::Move)
    );
    // In its own pair the same ranges are the painting.
    driver.keys("\x1bb\tj\nt");
    assert_eq!(driver.app.mapping.text_mappings[0].mapping.entries.len(), 3);
    // Nothing was edited by looking.
    assert!(!driver.session().dirty);
}

#[test]
fn saving_a_finished_sample_promotes_it_with_its_stub() {
    let mut driver = Driver::scratch("rust-function-moved-intact", true, true);
    let sample = driver.session().dir.clone();
    // A sample with no README cannot enter the corpus; the truth is saved all the same.
    fs::rename(sample.join("README.md"), sample.join("README.kept")).unwrap();
    driver.keys("s");
    assert!(
        driver.status().contains("no README.md"),
        "{}",
        driver.status()
    );
    assert!(sample.join(TRUTH_FILE_NAME).is_file());
    fs::rename(sample.join("README.kept"), sample.join("README.md")).unwrap();

    driver.keys("s");
    assert!(
        driver.status().contains("promoted to changesets/handmade"),
        "{}",
        driver.status()
    );
    let session = driver.session();
    assert!(!session.is_sample && !sample.exists());
    assert_eq!(
        session.dir,
        session.corpus_root.join("handmade").join(&session.name)
    );
    for kept in [
        "README.md",
        SOURCE_FILE_NAME,
        TRUTH_FILE_NAME,
        "before/src/geometry.rs.test",
    ] {
        assert!(session.dir.join(kept).is_file(), "{kept}");
    }
    let repository_stub = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/test/fixtures/changesets/rust_function_moved_intact.rs");
    assert_eq!(
        fs::read_to_string(session.stub_path()).unwrap(),
        fs::read_to_string(repository_stub).unwrap(),
        "the generated stub is the one the format's author wrote"
    );
    let mods = fs::read_to_string(session.fixtures_root.join("changesets.rs")).unwrap();
    assert_eq!(mods, "#[cfg(test)]\nmod rust_function_moved_intact;\n");
}

#[test]
fn violations_are_listed_and_enter_opens_the_pair_one_names() {
    let mut driver = Driver::scratch("rust-function-moved-intact", true, false);
    // Unpainted, the function that left geometry.rs is a removed leaf nobody painted.
    driver.app.changeset.as_mut().unwrap().truth.text_mappings[0]
        .entries
        .clear();
    driver.keys("V");
    assert!(matches!(
        driver.session().overlay,
        Some(Overlay::Violations { selected: 0 })
    ));
    assert!(!driver.session().violations.is_empty());
    let screen = driver.screen(220, 30);
    assert!(
        screen.contains(
            "[single_file_invariants_hold_per_pair] src/geometry.rs -> src/geometry.rs: [11]"
        ),
        "{screen}"
    );
    driver.keys("jk\n");
    assert!(
        driver
            .app
            .display_name()
            .contains("before src/geometry.rs -> after src/geometry.rs")
    );
    let Some(Modal::TextView { state }) = &driver.app.modal else {
        panic!("the violation's place opens in the text view");
    };
    assert_eq!(state.cursor[BEFORE].0, 9, "the row the violation names");

    // A violation about files, not a pair, has nowhere to go; saving reports the count.
    let mut driver = Driver::scratch("python-function-moved-and-renamed", false, false);
    driver.keys("CxV\n");
    assert!(
        driver.status().contains("names no pair"),
        "{}",
        driver.status()
    );
    driver.keys("\x1bs");
    assert!(
        driver.status().contains("violation(s) - V lists them"),
        "{}",
        driver.status()
    );
    assert!(
        driver.status().contains("not solved"),
        "{}",
        driver.status()
    );
}

#[test]
fn the_pair_view_goes_back_to_the_board_and_saves_the_whole_changeset() {
    let mut driver = Driver::scratch("go-three-files-nothing-crossing", false, false);
    driver.keys("C\nS");
    assert!(driver.app.dirty && driver.session().dirty);
    driver.keys("s");
    assert!(
        driver.status().starts_with("Saved human_changeset.json"),
        "{}",
        driver.status()
    );
    assert!(!driver.app.dirty && !driver.session().dirty);
    assert!(driver.session().dir.join(TRUTH_FILE_NAME).is_file());
    for back in ["\x08", "b", "q", "\x1b"] {
        driver.keys(back);
        assert!(driver.codes.is_none(), "{back:?} goes back to the board");
        assert!(driver.session().pair.is_none());
        driver.keys("\n");
        assert!(driver.codes.is_some());
    }
    // Keys that belong to other sessions say so instead of leaving the changeset.
    for key in ["o", "x", "\n"] {
        driver.keys(key);
        assert!(driver.codes.is_some(), "{key:?}");
        assert!(driver.app.modal.is_none(), "{key:?}: {}", driver.status());
    }
    // The board's help is the one sheet, opened at the changeset's own keys.
    driver.keys("b?");
    let screen = driver.screen(160, 40);
    assert!(screen.contains("CHANGESETS"), "{screen}");
    assert!(screen.contains("The board"), "{screen}");
    driver.keys("?\n");
    // Quitting the board with unsaved work takes a second `q`.
    driver.keys("\t]Ib");
    assert!(matches!(
        board_key(&mut driver.app, KeyCode::Char('q')),
        BoardEnd::Stay
    ));
    assert!(
        driver.status().contains("Unsaved changes"),
        "{}",
        driver.status()
    );
    assert!(matches!(
        board_key(&mut driver.app, KeyCode::Char('q')),
        BoardEnd::Quit
    ));
}

#[test]
fn the_key_log_names_the_changeset_views() {
    let mut driver = Driver::scratch("rust-function-copied", true, false);
    assert_eq!(board_mode(driver.session()), "board");
    driver.keys("L");
    assert_eq!(board_mode(driver.session()), "pool");
    driver.keys("\x1b\n-");
    assert_eq!(
        crate::keylog::mode_name(driver.app.modal.as_ref()),
        "partners"
    );
}

// ─── The sessions that solved three handmade changesets ─────────────────────────────────────

/// `parse_body` leaves handlers.py for codec.py as `decode_body`.
fn solve_python_function_moved_and_renamed(driver: &mut Driver) {
    // Both files are themselves; each pair starts from the engine's matches.
    driver.keys("C\nSbj\tj\nS");
    // handlers.py: one import goes, another comes, and the call follows the new name - by word,
    // where the seed painted the whole identifier.
    driver.keys("]D\t]I\t");
    driver.keys("t:1\nv$d\t:1\nv$i");
    driver.keys("\t:9\n^wwuvllll\t:5\n^wwvlllllm\x1bb");
    // The pool has the function on both sides, but renamed, so not as a candidate: pin one,
    // open it beside the other, match with children.
    driver.keys("Lyj\nM");
    // Minimal: `def `, then the changed word, then the rest of the line, then the body.
    driver.keys("t:4\nvlll\t:8\nvlllm");
    driver.keys("lvlllll\tlvllllm");
    driver.keys("lv$\tlv$m");
    driver.keys("j^v$\tj^v$m");
    // Full, branched for the whole changeset; its `def` line updates the whole identifier.
    driver.keys("sj\n");
    driver.keys(":4\nuwuwu0vlll\t:8\nvlllm");
    driver.keys("lve\tlvem");
    driver.keys("lv$\tlv$m\x1bb");
    driver.keys("xk\tkx");
}

/// helpers.ts becomes format.ts unedited; listing.ts imports it under the new name.
fn solve_typescript_file_renamed_unedited(driver: &mut Driver) {
    // git's rename is right here: confirmed, and the engine sees two identical files.
    driver.keys("C\nSb");
    driver.keys("j\tj\nS");
    driver.keys("tsj\n\x1bb");
    driver.keys("xk\tkx");
}

/// The control: three files edited, nothing crossing between them.
fn solve_go_three_files_nothing_crossing(driver: &mut Driver) {
    // len.go gains a comment, queue.go a field.
    driver.keys("C\nS\t]It:3\nv$i\x1bb");
    driver.keys("jj\tjj\nS\t]It:5\n^v$i\x1bb");
    // pop.go: a second result, an early return, and `true` on the old return. The seed paints
    // `string` as moved one column, which it is not.
    driver.keys("k\tk\nS\t]iiiiiIiIii");
    driver.keys("tg/(string\nvilu");
    driver.keys(&format!("/{BACKSPACES}, bool\nvlllllli"));
    driver.keys(":4\n^vjj$i");
    driver.keys(&format!("/{BACKSPACES}, true\nvllllli"));
    // Full: the branch widens the inserted `(` over the blank before it, which was there already.
    driver.keys("sj\n");
    driver.keys(&format!("g/{BACKSPACES}(string\nuvi\x1bb"));
    driver.keys("xj\tjxkk\tkkx");
}

impl Driver {
    /// The repository's own changeset `name`, for the `solve_*` sessions.
    fn in_place(name: &str) -> Self {
        let dir = changeset_dir(name).unwrap();
        let _ = fs::remove_file(dir.join(TRUTH_FILE_NAME));
        Driver::over(Session::open(&dir, false).unwrap(), None)
    }
}

type Script = fn(&mut Driver);

const SOLVED_BY_SCRIPT: [(&str, Script); 3] = [
    (
        "python-function-moved-and-renamed",
        solve_python_function_moved_and_renamed,
    ),
    (
        "typescript-file-renamed-unedited",
        solve_typescript_file_renamed_unedited,
    ),
    (
        "go-three-files-nothing-crossing",
        solve_go_three_files_nothing_crossing,
    ),
];

/// Each scripted session, on a scratch copy, ends solved and valid and writes the truth the
/// repository holds: the committed files are what the tool's keys produce, not hand-edited JSON.
#[test]
fn the_scripted_sessions_solve_their_changesets_as_committed() {
    for (name, script) in SOLVED_BY_SCRIPT {
        let mut driver = Driver::scratch(name, false, false);
        script(&mut driver);
        let (truth, violations) = driver.saved();
        assert_eq!(violations, [], "{name}");
        assert!(
            truth.is_solved(&driver.session().trees),
            "{name}: {}",
            driver.status()
        );
        assert!(driver.session().stub_path().is_file(), "{name}");
        let saved = fs::read_to_string(driver.session().dir.join(TRUTH_FILE_NAME)).unwrap();
        let (_, committed) = repository_truth(name);
        assert_eq!(saved, committed, "{name}");
    }
}

/// Writes the three truths and their stubs into the repository.
#[test]
#[ignore = "writes into src/test/data/changesets and src/test/fixtures/changesets"]
fn solve_the_handmade_changesets_in_place() {
    for (name, script) in SOLVED_BY_SCRIPT {
        let mut driver = Driver::in_place(name);
        script(&mut driver);
        let (_, violations) = driver.saved();
        assert_eq!(violations, [], "{name}");
        assert!(
            driver.status().contains("solved"),
            "{name}: {}",
            driver.status()
        );
        omnidiff::test::helper::human_changeset::assert_solved_ground_truth(name).unwrap();
    }
}
