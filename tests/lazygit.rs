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
//! the viewer.
//!
//! Linux only: the pseudo-terminal comes from util-linux `script`, whose flags BSD `script` (macOS)
//! does not share.
#![cfg(target_os = "linux")]

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
fn on_a_terminal(args: &str, env: &[(&str, &Path)]) -> (bool, String) {
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
        &[("BEFORE", &before), ("AFTER", &after)],
    );
    assert!(success, "{output}");
    assert!(output.contains("=== after: src/main.rs ==="), "{output}");
    assert!(output.contains("b();"), "{output}");
}
