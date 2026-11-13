//! Starting a program the app does not wait for, without leaving a zombie.
//!
//! A child that is spawned and never waited on stays in the process table as
//! a zombie until the app exits. Long-running TUIs that fire `notify-send` or
//! `xdg-open` would pile them up. [`detached`] reaps each one on its own thread.

use std::process::Command;

/// Runs `command` in the background and reaps it when it exits.
pub fn detached(mut command: Command) -> std::io::Result<()> {
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_program_is_an_error_and_a_real_one_is_not() {
        assert!(detached(Command::new("vibekit-no-such-program")).is_err());
        assert!(detached(Command::new("true")).is_ok());
    }
}
