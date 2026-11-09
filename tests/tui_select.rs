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

//! A mouse drag copies the text under it, end to end: the real binary on a pseudo-terminal gets
//! SGR mouse reports for a press, a drag and a release, and writes the selected code as an OSC 52
//! clipboard sequence (issue #71). With mouse capture on, a plain drag never reaches the
//! terminal's own selection, so before this nothing could be copied without a modifier.
//!
//! Linux only: the pseudo-terminal comes from util-linux `script`, whose flags BSD `script` (macOS)
//! does not share.
#![cfg(target_os = "linux")]

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// The last text of the viewer's footer, as in `tui_color_depth`: once it is out, the whole first
/// frame is.
const FRAME_DRAWN: &str = "q:quit";

/// Long enough for a debug build to diff the fixture and settle on a loaded machine.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Kills the viewer however the test ends. A failed assertion would otherwise leave `script` and
/// omnidiff running, holding the inherited stderr open, and whatever reads the test's output would
/// wait on it forever.
struct Viewer(Child);

impl Drop for Viewer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The viewer on the README's example diff, its output arriving on the receiver in chunks.
fn start_viewer() -> (Viewer, Receiver<Vec<u8>>, tempfile::TempDir) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/test/data/diffs/handmade/python-refactoring");
    let home = tempfile::tempdir().expect("temp dir");
    let mut child = Command::new("script")
        // -q: no banner, -f: flush every write, -e: omnidiff's exit status, -c: the command.
        .args([
            "-qfec",
            r#"stty cols 200 rows 40; exec "$OMNIDIFF" "$BEFORE" "$AFTER""#,
            "/dev/null",
        ])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home.path())
        .env("OMNIDIFF_CONFIG", home.path().join("config.toml"))
        .env("TERM", "xterm-256color")
        .env("OMNIDIFF", env!("CARGO_BIN_EXE_omnidiff"))
        .env("BEFORE", fixture.join("before.py.test"))
        .env("AFTER", fixture.join("after.py.test"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
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
    (Viewer(child), chunks, home)
}

fn send(viewer: &mut Viewer, keys: &[u8]) {
    let stdin = viewer.0.stdin.as_mut().expect("piped stdin");
    stdin.write_all(keys).expect("send keys");
    stdin.flush().expect("flush keys");
}

/// Reads until `wanted` has arrived; panics at `deadline`.
fn read_until(chunks: &Receiver<Vec<u8>>, output: &mut Vec<u8>, wanted: &str, deadline: Instant) {
    while !String::from_utf8_lossy(output).contains(wanted) {
        match chunks.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(chunk) => output.extend(chunk),
            Err(error) => panic!(
                "no {wanted:?} ({error:?}); omnidiff wrote:\n{}",
                String::from_utf8_lossy(output).escape_debug()
            ),
        }
    }
}

/// Inverse of `omnidiff::tui::clipboard::base64`, for what the terminal would decode.
fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::new();
    for byte in text.bytes().filter(|&b| b != b'=') {
        let value = ALPHABET.iter().position(|&a| a == byte).expect("base64") as u32;
        bits = bits << 6 | value;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    out
}

#[test]
fn a_drag_in_a_panel_puts_its_text_on_the_clipboard() {
    let (mut viewer, chunks, _home) = start_viewer();
    let deadline = Instant::now() + TIMEOUT;
    let mut output = Vec::new();
    read_until(&chunks, &mut output, FRAME_DRAWN, deadline);

    // SGR reports, 1-based: press in the before panel's gutter, drag three rows down and into
    // the code, release there.
    send(&mut viewer, b"\x1b[<0;2;5M\x1b[<32;30;8M\x1b[<0;30;8m");
    const OSC52: &str = "\x1b]52;c;";
    let encoded = loop {
        let text = String::from_utf8_lossy(&output).into_owned();
        if let Some((encoded, _)) = text
            .split_once(OSC52)
            .and_then(|(_, rest)| rest.split_once('\x07'))
        {
            break encoded.to_string();
        }
        read_until(&chunks, &mut output, "\x07", deadline);
    };
    let copied = String::from_utf8(base64_decode(&encoded)).expect("UTF-8 text");

    let before = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/test/data/diffs/handmade/python-refactoring/before.py.test"),
    )
    .unwrap();
    assert_eq!(copied.matches('\n').count(), 3, "four rows: {copied:?}");
    assert!(
        before.contains(&copied),
        "the copy is a piece of the before file: {copied:?}"
    );
    read_until(&chunks, &mut output, "copied", deadline);
}
