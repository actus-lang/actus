//! Terminal output of the command line interface and what happens when the
//! reader of a stream goes away.
//!
//! `print!` and `println!` panic when the stream is closed, and Rust leaves
//! `SIGPIPE` ignored, so `actus --version | head -1` used to end in a panic and
//! a backtrace. The `actus` binary enables the closed-stream policy: a write
//! that fails because the reader closed the pipe stops the process quietly with
//! [`CLOSED_STREAM_STATUS`], like `cat` or `grep` do, with no message. Code that
//! owns resources uses the `try_` macros and stops through its normal path so
//! that it can clean up first.
//!
//! The policy is off for in-process callers of `run_with_args` (tests), which
//! keep the capture of the test harness.

use std::fmt::Arguments;
use std::io::{self, ErrorKind, Write};
use std::sync::atomic::{AtomicBool, Ordering};

/// The exit status when the reader of a stream closed it: `128 + SIGPIPE`, the
/// status a shell reports for a process that `SIGPIPE` terminated. It is the
/// same on every platform so that scripts see one value.
pub(super) const CLOSED_STREAM_STATUS: i32 = 141;

static CLOSED_STREAM_POLICY: AtomicBool = AtomicBool::new(false);

/// Makes a closed or failing stream stop the process instead of panicking.
pub(super) fn enable_closed_stream_policy() {
    CLOSED_STREAM_POLICY.store(true, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
pub(super) enum Stream {
    Stdout,
    Stderr,
}

/// A write that must stop the command, with the exit status to stop with.
#[derive(Debug)]
pub(super) struct OutputStopped(pub(super) i32);

/// Writes `arguments` to `stream`, followed by a newline when `newline`.
///
/// Without the policy this is `print!`/`println!`. With it, a closed stream is
/// `Err` with [`CLOSED_STREAM_STATUS`] and any other failure of stdout is
/// reported once on stderr and is `Err` with status 1. A failing stderr cannot
/// report itself, so it only stops.
pub(super) fn write(
    stream: Stream,
    arguments: Arguments<'_>,
    newline: bool,
) -> Result<(), OutputStopped> {
    if !CLOSED_STREAM_POLICY.load(Ordering::Relaxed) {
        match (stream, newline) {
            (Stream::Stdout, true) => println!("{arguments}"),
            (Stream::Stdout, false) => print!("{arguments}"),
            (Stream::Stderr, true) => eprintln!("{arguments}"),
            (Stream::Stderr, false) => eprint!("{arguments}"),
        }
        return Ok(());
    }
    let result = match stream {
        Stream::Stdout => write_locked(&mut io::stdout().lock(), arguments, newline),
        Stream::Stderr => write_locked(&mut io::stderr().lock(), arguments, newline),
    };
    let Err(error) = result else { return Ok(()) };
    if error.kind() == ErrorKind::BrokenPipe {
        return Err(OutputStopped(CLOSED_STREAM_STATUS));
    }
    if matches!(stream, Stream::Stdout) {
        let _ = writeln!(io::stderr(), "error: cannot write to stdout: {error}");
    }
    Err(OutputStopped(1))
}

fn write_locked(
    stream: &mut impl Write,
    arguments: Arguments<'_>,
    newline: bool,
) -> io::Result<()> {
    if newline {
        writeln!(stream, "{arguments}")?
    } else {
        write!(stream, "{arguments}")?
    }
    stream.flush()
}

/// Ends the process for a stopped write; for commands that own nothing to
/// clean up.
pub(super) fn exit_if_stopped(result: Result<(), OutputStopped>) {
    if let Err(OutputStopped(status)) = result {
        std::process::exit(status);
    }
}

macro_rules! try_outln {
    ($($argument:tt)*) => {
        $crate::cli::terminal_output::write(
            $crate::cli::terminal_output::Stream::Stdout,
            format_args!($($argument)*),
            true,
        )
    };
}

macro_rules! outln {
    ($($argument:tt)*) => {
        $crate::cli::terminal_output::exit_if_stopped(
            $crate::cli::terminal_output::try_outln!($($argument)*),
        )
    };
}

macro_rules! out {
    ($($argument:tt)*) => {
        $crate::cli::terminal_output::exit_if_stopped($crate::cli::terminal_output::write(
            $crate::cli::terminal_output::Stream::Stdout,
            format_args!($($argument)*),
            false,
        ))
    };
}

macro_rules! errln {
    ($($argument:tt)*) => {
        $crate::cli::terminal_output::exit_if_stopped($crate::cli::terminal_output::write(
            $crate::cli::terminal_output::Stream::Stderr,
            format_args!($($argument)*),
            true,
        ))
    };
}

pub(super) use {errln, out, outln, try_outln};
