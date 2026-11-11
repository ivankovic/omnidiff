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

//! omnidiff as lazygit runs it, end to end: the real binary in git's 7-argument
//! `GIT_EXTERNAL_DIFF` form on a pseudo-terminal, which must print text and exit rather than open
//! the viewer, and with `OSC1717=V1`, which must announce the protocol and tag the rows (see
//! `omnidiff::tui::osc1717`) - and change nothing without it.
//!
//! Linux only: the pseudo-terminal comes from util-linux `script`, whose flags BSD `script` (macOS)
//! does not share.
#![cfg(target_os = "linux")]

use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Long enough for a debug build to diff a small file, short enough that a viewer waiting for a
/// key fails the test.
const TIMEOUT: Duration = Duration::from_secs(60);

/// Everything omnidiff writes to a pseudo-terminal when run with `args` (shell words, quoted by
/// the caller) and `env`, and its exit status; panics if it has not exited within [`TIMEOUT`].
fn on_a_terminal(args: &str, env: &[(&str, &OsStr)]) -> (bool, String) {
    let home = tempfile::tempdir().expect("temp dir");
    let mut command = Command::new("script");
    // -q: no banner, -f: flush every write, -e: omnidiff's exit status, -c: the command.
    command
        .args([
            "-qfec",
            &format!(r#"stty cols 120 rows 30; exec "$OMNIDIFF" {args}"#),
            "/dev/null",
        ])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home.path())
        .env("OMNIDIFF_CONFIG", home.path().join("config.toml"))
        .env("TERM", "xterm-256color")
        .env("NO_COLOR", "1")
        .env("OMNIDIFF", env!("CARGO_BIN_EXE_omnidiff"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    for (name, value) in env {
        command.env(name, value);
    }
    let mut child = command
        .spawn()
        .expect("util-linux `script` provides the pseudo-terminal; is it installed?");

    let mut stdout = child.stdout.take().expect("piped stdout");
    let (sender, chunks) = mpsc::channel();
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(read) = stdout.read(&mut chunk) {
            if read == 0 || sender.send(chunk[..read].to_vec()).is_err() {
                break;
            }
        }
    });
    let deadline = Instant::now() + TIMEOUT;
    let mut output = Vec::new();
    loop {
        match chunks.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(chunk) => output.extend(chunk),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                panic!(
                    "omnidiff {args} did not exit within {TIMEOUT:?}; it wrote:\n{}",
                    String::from_utf8_lossy(&output).escape_debug()
                );
            }
        }
    }
    let status = child.wait().expect("wait for script");
    // The terminal turns each `\n` into `\r\n`.
    let text = String::from_utf8_lossy(&output).replace("\r\n", "\n");
    (status.success(), text)
}

/// `before.rs` and `after.rs` in a fresh directory, one line changed.
fn pair() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("before.rs"), "fn main() {\n    a();\n}\n").unwrap();
    std::fs::write(dir.path().join("after.rs"), "fn main() {\n    b();\n}\n").unwrap();
    dir
}

#[test]
fn gits_seven_argument_form_on_a_terminal_prints_text_and_exits() {
    let dir = pair();
    let (before, after) = (dir.path().join("before.rs"), dir.path().join("after.rs"));
    let (success, output) = on_a_terminal(
        r#"src/main.rs "$BEFORE" 0000000 100644 "$AFTER" 0000000 100644"#,
        &[("BEFORE", before.as_os_str()), ("AFTER", after.as_os_str())],
    );
    assert!(success, "{output}");
    assert!(output.contains("=== after: src/main.rs ==="), "{output}");
    assert!(output.contains("b();"), "{output}");
    assert!(!output.contains("\u{1b}]1717"), "{output:?}");
}

const HANDSHAKE: &str = "\u{1b}]1717;1\u{1b}\\";

/// lazygit's render: the 7-argument form on a pseudo-terminal, `OSC1717=V1` set.
#[test]
fn lazygits_render_starts_with_the_handshake_and_tags_the_changed_lines() {
    let dir = pair();
    let (before, after) = (dir.path().join("before.rs"), dir.path().join("after.rs"));
    let (success, output) = on_a_terminal(
        r#"--context 3 src/main.rs "$BEFORE" 0000000 100644 "$AFTER" 0000000 100644"#,
        &[
            ("BEFORE", before.as_os_str()),
            ("AFTER", after.as_os_str()),
            ("OSC1717", OsStr::new("V1")),
        ],
    );
    assert!(success, "{output:?}");
    assert!(output.starts_with(HANDSHAKE), "{output:?}");
    assert_eq!(output.matches(HANDSHAKE).count(), 1, "{output:?}");
    for record in [
        "\u{1b}]1717;1;f;;;src/main.rs\u{1b}\\=== before: src/main.rs ===\n",
        "\u{1b}]1717;1;d;2;2;src/main.rs\u{1b}\\",
        "\u{1b}]1717;1;a;2;;src/main.rs\u{1b}\\",
        "\u{1b}]1717;1;c;3;;src/main.rs\u{1b}\\",
    ] {
        assert!(output.contains(record), "{record:?} in {output:?}");
    }
}

/// lazygit's render with `--layout side-by-side`: the handshake first, then two columns, each
/// half of a row starting with its own record (§6.2). With `--width` the layout is that wide;
/// without it, as wide as the pseudo-terminal lazygit gives its renderer.
#[test]
fn lazygits_side_by_side_render_tags_each_half_of_a_row() {
    let dir = pair();
    let (before, after) = (dir.path().join("before.rs"), dir.path().join("after.rs"));
    let env = [
        ("BEFORE", before.as_os_str()),
        ("AFTER", after.as_os_str()),
        ("OSC1717", OsStr::new("V1")),
    ];
    let git_form = r#"src/main.rs "$BEFORE" 0000000 100644 "$AFTER" 0000000 100644"#;
    for (width_flag, width) in [("--width 100", 100), ("", 120)] {
        let (success, output) = on_a_terminal(
            &format!("--context 3 --layout side-by-side {width_flag} {git_form}"),
            &env,
        );
        assert!(success, "{output:?}");
        assert!(output.starts_with(HANDSHAKE), "{output:?}");
        let record = |kind: &str, new: &str, old: &str| {
            format!("\u{1b}]1717;1;{kind};{new};{old};src/main.rs\u{1b}\\")
        };
        let rows: Vec<&str> = output[HANDSHAKE.len()..].lines().collect();
        let header = rows[0]
            .strip_prefix(record("f", "", "").as_str())
            .expect("the header row carries the f");
        assert!(
            header.starts_with("── before: src/main.rs ─")
                && header.contains("┬─ after: src/main.rs ─"),
            "{header}"
        );
        assert_eq!(header.chars().count(), width, "{header}");
        // `fn main() {` is context on both halves; `a();` -> `b();` one row, `d` then `a`.
        assert!(
            rows[1].starts_with(&record("c", "1", ""))
                && rows[1].contains(&format!("│{}", record("c", "1", ""))),
            "{:?}",
            rows[1]
        );
        assert!(
            rows[2].starts_with(&record("d", "2", "2"))
                && rows[2].contains(&format!("│{}", record("a", "2", ""))),
            "{:?}",
            rows[2]
        );
        assert!(rows[2].contains("a();") && rows[2].contains("b();"));
    }
}

/// Under lazygit the `M` panel's saved options decide the painting, as in any headless run, and the
/// records stay what git's patch says: the patch lazygit stages from has every whitespace change
/// whatever its own ignore-whitespace toggle, so a line Minimal leaves unpainted is still one it
/// can stage.
#[test]
fn the_saved_render_options_change_the_painting_and_never_the_records() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (before, after) = (dir.path().join("before.py"), dir.path().join("after.py"));
    std::fs::write(
        &before,
        "def f(x):\n    if x:\n        return 1\n    return 0\n",
    )
    .unwrap();
    std::fs::write(
        &after,
        "def f(x):\n    if x:\n        if x > 1:\n            return 1\n    return 0\n",
    )
    .unwrap();
    let minimal = dir.path().join("minimal.toml");
    std::fs::write(
        &minimal,
        "[render_options]\nleading_whitespace = false\nstructural_punctuation = false\n\
         whole_pair_updates = false\npaint_reindent_only_moves = false\n\
         paint_displaced_moves = false\npaint_resized_moves = false\n\
         whole_identifier_updates = false\ninterior_whitespace = false\n",
    )
    .unwrap();
    let run = |config: &Path| {
        let output = Command::new(env!("CARGO_BIN_EXE_omnidiff"))
            .args([
                OsStr::new("w.py"),
                before.as_os_str(),
                OsStr::new("0000000"),
            ])
            .args([
                OsStr::new("100644"),
                after.as_os_str(),
                OsStr::new("0000000"),
            ])
            .arg("100644")
            .current_dir(dir.path())
            .env("OMNIDIFF_CONFIG", config)
            .env("OSC1717", "V1")
            .env_remove("NO_COLOR")
            .output()
            .expect("omnidiff runs");
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).expect("UTF-8");
        let records: Vec<String> = text
            .split("\u{1b}]1717;")
            .skip(1)
            .map(|rest| rest.split('\u{1b}').next().unwrap_or_default().to_string())
            .collect();
        (text, records)
    };
    let (full, full_records) = run(&dir.path().join("none.toml"));
    let (minimal, minimal_records) = run(&minimal);
    assert_ne!(full, minimal, "the saved options change the painting");
    assert_eq!(full_records, minimal_records);
    assert!(
        full_records.contains(&"1;a;3;;w.py".to_string()),
        "{full_records:?}"
    );
}

/// omnidiff with `args`, stdout a pipe, and `OSC1717` set to `osc` (unset for `None`).
fn piped(args: &[&OsStr], osc: Option<&str>) -> (bool, Vec<u8>) {
    let home = tempfile::tempdir().expect("temp dir");
    let mut command = Command::new(env!("CARGO_BIN_EXE_omnidiff"));
    command
        .args(args)
        .env("OMNIDIFF_CONFIG", home.path().join("config.toml"))
        .env_remove("OSC1717")
        .env_remove("GIT_EXTERNAL_DIFF")
        .env_remove("NO_COLOR");
    if let Some(osc) = osc {
        command.env("OSC1717", osc);
    }
    let output = command.output().expect("omnidiff runs");
    (output.status.success(), output.stdout)
}

/// lazygit's probe: two empty files, git's seven arguments with abbreviated hashes, stdout a
/// pipe. It looks for the handshake and nothing else.
#[test]
fn the_probe_is_answered_with_the_handshake() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = (dir.path().join("old"), dir.path().join("new"));
    std::fs::write(&old, "").unwrap();
    std::fs::write(&new, "").unwrap();
    for extra in [&[][..], &["--headless"][..]] {
        let mut args: Vec<&OsStr> = vec![OsStr::new("--context"), OsStr::new("3")];
        args.extend(extra.iter().map(OsStr::new));
        args.extend([
            OsStr::new("probe"),
            old.as_os_str(),
            OsStr::new("0000000"),
            OsStr::new("100644"),
            new.as_os_str(),
            OsStr::new("0000000"),
            OsStr::new("100644"),
        ]);
        let (success, stdout) = piped(&args, Some("V1"));
        assert!(success);
        assert!(stdout.starts_with(HANDSHAKE.as_bytes()), "{extra:?}");
    }
}

/// Without `V1` in `OSC1717` - unset, empty, another version, the wrong case - the output is the
/// same bytes, and carries no record; with it, every path that prints starts with the handshake.
#[test]
fn without_v1_the_output_does_not_change() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/test/data/diffs/handmade");
    let text = fixtures.join("python-refactoring");
    let dir = tempfile::tempdir().unwrap();
    let png = |name: &str, pixel: u8| {
        let path = dir.path().join(name);
        // A 1x1 grey PNG is bytes omnidiff reads as a picture; any two differing blobs do.
        std::fs::write(&path, [0x89, b'P', b'N', b'G', 0, pixel, 0xff, 0]).unwrap();
        path
    };
    let (blob_a, blob_b) = (png("a.bin", 1), png("b.bin", 2));
    let cases: Vec<Vec<&OsStr>> = vec![
        vec![
            text.join("before.py.test").into_os_string().leak(),
            text.join("after.py.test").into_os_string().leak(),
        ],
        vec![
            OsStr::new("calc.py"),
            text.join("before.py.test").into_os_string().leak(),
            OsStr::new("0000000"),
            OsStr::new("100644"),
            text.join("after.py.test").into_os_string().leak(),
            OsStr::new("0000000"),
            OsStr::new("100644"),
        ],
        vec![blob_a.as_os_str(), blob_b.as_os_str()],
        vec![blob_a.as_os_str(), blob_a.as_os_str()],
    ];
    for args in cases {
        let (_, unset) = piped(&args, None);
        assert!(!unset.is_empty(), "{args:?}");
        for osc in ["", "V2", "v1", " ,V0"] {
            let (_, other) = piped(&args, Some(osc));
            assert_eq!(other, unset, "OSC1717={osc:?} for {args:?}");
        }
        assert!(
            !String::from_utf8_lossy(&unset).contains("\u{1b}]1717"),
            "{args:?}"
        );
        let (_, negotiated) = piped(&args, Some("V2,V1"));
        assert!(negotiated.starts_with(HANDSHAKE.as_bytes()), "{args:?}");
        let negotiated = String::from_utf8_lossy(&negotiated).into_owned();
        assert!(negotiated.contains("\u{1b}]1717;1;f;;;"), "{negotiated:?}");
    }
}

/// A path that could end the OSC early gets no records, only the handshake.
#[test]
fn a_path_with_a_control_character_gets_no_records() {
    let dir = pair();
    let (before, after) = (dir.path().join("before.rs"), dir.path().join("after.rs"));
    let args = [
        OsStr::new("--color=never"),
        OsStr::new("src/\u{1b}\\x\u{7}.rs"),
        before.as_os_str(),
        OsStr::new("0000000"),
        OsStr::new("100644"),
        after.as_os_str(),
        OsStr::new("0000000"),
        OsStr::new("100644"),
    ];
    let (success, stdout) = piped(&args, Some("V1"));
    assert!(success);
    let stdout = String::from_utf8_lossy(&stdout);
    assert!(stdout.starts_with(HANDSHAKE));
    assert_eq!(stdout.matches("\u{1b}]1717").count(), 1, "{stdout:?}");
    assert!(stdout.contains("b();"));
}
