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

//! An idle viewer is silent, end to end: the real binary on a pseudo-terminal writes nothing while
//! nothing changes, and still answers a key with a frame (issue #72).
//!
//! The frame clock fires 60 times a second. Drawn on every tick, an unchanged screen still sent
//! each frame's synchronized-update markers and cursor position, about 225 writes a second, and a
//! terminal that repaints on every synchronized update repainted 60 times a second for nothing.
//! The property is byte-level, like `tui_color_depth`'s, so it holds whatever the machine's speed:
//! after the first frame the output must fall silent, and a key must break the silence.
//!
//! Linux only: the pseudo-terminal comes from util-linux `script`, whose flags BSD `script` (macOS)
//! does not share.
#![cfg(target_os = "linux")]

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// The last text of the viewer's footer, as in `tui_color_depth`: once it is out, the whole first
/// frame is.
const FRAME_DRAWN: &str = "q:quit";

/// Long enough for a debug build to diff the fixture and settle on a loaded machine.
const TIMEOUT: Duration = Duration::from_secs(30);

/// How long the output must stay empty to count as silent: dozens of frame-clock ticks.
const SILENCE: Duration = Duration::from_millis(1500);

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

/// Waits for `SILENCE` with no output at all and returns how many bytes arrived before it; panics
/// if the output never falls silent by `deadline`.
fn wait_for_silence(chunks: &Receiver<Vec<u8>>, deadline: Instant) -> usize {
    let started = Instant::now();
    let mut bytes = 0usize;
    loop {
        match chunks.recv_timeout(SILENCE) {
            Ok(chunk) => bytes += chunk.len(),
            Err(RecvTimeoutError::Timeout) => return bytes,
            Err(RecvTimeoutError::Disconnected) => panic!("omnidiff exited while idle"),
        }
        assert!(
            Instant::now() < deadline,
            "an idle viewer never fell silent: {bytes} bytes in {:?}, never a {SILENCE:?} gap",
            started.elapsed()
        );
    }
}

#[test]
fn an_idle_viewer_writes_nothing_and_a_key_still_draws() {
    let (mut viewer, chunks, _home) = start_viewer();
    let deadline = Instant::now() + TIMEOUT;
    let mut output = Vec::new();
    read_until(&chunks, &mut output, FRAME_DRAWN, deadline);

    // Whatever follows the first frame settles; then nothing more may arrive.
    wait_for_silence(&chunks, deadline);
    match chunks.recv_timeout(SILENCE) {
        Err(RecvTimeoutError::Timeout) => {}
        Ok(chunk) => panic!(
            "an idle viewer wrote {:?} with nothing changed",
            String::from_utf8_lossy(&chunk).escape_debug()
        ),
        Err(RecvTimeoutError::Disconnected) => panic!("omnidiff exited while idle"),
    }

    // A key moves the cursor, and the footer's `Ln 1` becomes `Ln 2`, so a frame must follow. Only
    // the changed cells are sent, so any output at all is the frame.
    send(&mut viewer, b"j");
    match chunks.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(chunk) => assert!(!chunk.is_empty()),
        Err(error) => panic!("no frame after a key ({error:?})"),
    }

    send(&mut viewer, b"q");
    let status = viewer.0.wait().expect("wait for script");
    assert!(status.success(), "omnidiff exited with {status}");
}
