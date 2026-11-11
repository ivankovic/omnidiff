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
//! Panic containment: [`contain`] runs work that decodes or diffs attacker-chosen input and turns
//! a panic in it into an error, so one bad file cannot take the rest down - under
//! `GIT_EXTERNAL_DIFF` a non-zero exit makes git abandon the whole multi-file diff, and in the TUI
//! a panic would end the session.
//!
//! The panic hook still runs for a contained panic, before the unwind reaches [`contain`], and
//! its default message would then be printed on top of the one the caller reports - or, in the
//! TUI, straight over the screen. So the hooks ask [`is_contained`] first and stay silent for a
//! contained panic; the caller reports it once. That covers the worker threads the decoders and
//! the diff spawn too: they re-raise a worker's panic on the thread that called them, so while any
//! [`contain`] is running, a panic on any thread but `main` is treated as contained. Set
//! `RUST_BACKTRACE` to get the default message (and backtrace) anyway.

use std::any::Any;
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many [`contain`] calls are running, on any thread.
static RUNNING: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// How many [`contain`] calls are running on this thread.
    static HERE: Cell<usize> = const { Cell::new(0) };
}

/// Decrements both counters when the [`contain`] call ends, however it ends.
struct Running;

impl Running {
    fn start() -> Self {
        RUNNING.fetch_add(1, Ordering::SeqCst);
        HERE.with(|here| here.set(here.get() + 1));
        Running
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        HERE.with(|here| here.set(here.get().saturating_sub(1)));
        RUNNING.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Runs `work`, returning its panic's message as `Err` instead of unwinding further.
pub fn contain<T>(work: impl FnOnce() -> T) -> Result<T, String> {
    let _running = Running::start();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_err(|panic| message(&panic))
}

/// Whether a panic on the current thread now will be caught by a [`contain`] (see the module doc
/// for why that includes every thread but `main` while one runs anywhere).
pub fn is_contained() -> bool {
    if HERE.with(Cell::get) > 0 {
        return true;
    }
    RUNNING.load(Ordering::SeqCst) > 0 && std::thread::current().name() != Some("main")
}

/// Whether the hooks should print a contained panic anyway: when a backtrace was asked for.
fn wants_backtrace() -> bool {
    std::env::var_os("RUST_BACKTRACE").is_some_and(|value| !value.is_empty() && value != "0")
}

/// For a hook: true when it should say nothing about this panic, because [`contain`] will report
/// it (and no backtrace was asked for).
pub fn hook_stays_silent() -> bool {
    is_contained() && !wants_backtrace()
}

/// Installs a hook that leaves contained panics to their [`contain`] and prints the rest as the
/// default hook does. For the non-interactive modes; the TUI's hook (`ui::install_panic_hook`)
/// checks [`hook_stays_silent`] itself.
pub fn install_quiet_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if !hook_stays_silent() {
            default_hook(info);
        }
    }));
}

/// A panic payload's message: the `&str` or `String` that `panic!` carries.
pub fn message(panic: &Box<dyn Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        message.to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_becomes_its_message_and_the_counters_unwind() {
        let caught = contain(|| -> u8 { panic!("boom {}", 7) });
        assert_eq!(caught, Err("boom 7".to_string()));
        assert_eq!(HERE.with(Cell::get), 0);
        assert_eq!(contain(|| 3), Ok(3));
    }

    #[test]
    fn a_worker_threads_panic_is_contained_while_its_caller_is() {
        let inside = contain(|| std::thread::spawn(is_contained).join().expect("joins"));
        assert_eq!(inside, Ok(true));
        let re_raised = contain(|| {
            let worker = std::thread::spawn(|| -> u8 { panic!("in a worker") });
            if let Err(panic) = worker.join() {
                std::panic::resume_unwind(panic);
            }
        });
        assert_eq!(re_raised, Err("in a worker".to_string()));
    }
}
