//! CLI-only console IO for job handlers (`tasks run`).
//!
//! Outside an interactive CLI scope, prompts fail / use defaults and output is quiet.

use std::cell::Cell;
use std::fmt::Display;
use std::io::{self, BufRead, IsTerminal, Write};

thread_local! {
    static INTERACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard: enables [`is_interactive`] until dropped.
pub struct ConsoleGuard {
    _priv: (),
}

impl Drop for ConsoleGuard {
    fn drop(&mut self) {
        INTERACTIVE.with(|c| c.set(false));
    }
}

/// Enable interactive console for the current thread (CLI `tasks run`).
pub fn enter_cli() -> ConsoleGuard {
    INTERACTIVE.with(|c| c.set(true));
    ConsoleGuard { _priv: () }
}

/// True while inside [`enter_cli`].
pub fn is_interactive() -> bool {
    INTERACTIVE.with(|c| c.get())
}

/// Print an info line when interactive; otherwise no-op.
pub fn info(msg: impl Display) {
    if is_interactive() {
        println!("{msg}");
    }
}

/// Print a warning when interactive.
pub fn warn(msg: impl Display) {
    if is_interactive() {
        eprintln!("warn: {msg}");
    }
}

/// Print an error when interactive.
pub fn error(msg: impl Display) {
    if is_interactive() {
        eprintln!("error: {msg}");
    }
}

/// Print a blank or custom line when interactive.
pub fn line(msg: impl Display) {
    if is_interactive() {
        println!("{msg}");
    }
}

/// Print a simple ASCII table when interactive.
pub fn table(headers: &[&str], rows: &[Vec<String>]) {
    if !is_interactive() {
        return;
    }
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if let Some(w) = widths.get_mut(i) {
                *w = (*w).max(cell.len());
            }
        }
    }
    let fmt_row = |cols: &[&str]| {
        cols.iter()
            .enumerate()
            .map(|(i, c)| format!("{:<width$}", c, width = widths.get(i).copied().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join("  ")
    };
    println!("{}", fmt_row(headers));
    let sep: String = widths
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("  ");
    println!("{sep}");
    for row in rows {
        let refs: Vec<&str> = row.iter().map(String::as_str).collect();
        println!("{}", fmt_row(&refs));
    }
}

/// Prompt for a line of input. Errors when not interactive.
pub fn ask(prompt: impl Display) -> Result<String, String> {
    if !is_interactive() {
        return Err("console ask is only available during `tasks run`".into());
    }
    print!("{prompt} ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    // Piped stdin does not echo the newline; keep subsequent output on its own line.
    if !io::stdin().is_terminal() {
        println!();
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// Yes/no prompt. When not interactive, returns `default`.
pub fn confirm(prompt: impl Display, default: bool) -> bool {
    if !is_interactive() {
        return default;
    }
    let hint = if default { "Y/n" } else { "y/N" };
    print!("{prompt} [{hint}] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).is_err() {
        return default;
    }
    if !io::stdin().is_terminal() {
        println!();
    }
    let t = line.trim().to_ascii_lowercase();
    if t.is_empty() {
        return default;
    }
    matches!(t.as_str(), "y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_outside_cli() {
        assert!(!is_interactive());
        assert!(ask("x").is_err());
        assert!(!confirm("x", false));
        assert!(confirm("x", true));
        info("should not panic");
        table(&["a"], &[vec!["1".into()]]);
    }

    #[test]
    fn guard_enables_interactive() {
        {
            let _g = enter_cli();
            assert!(is_interactive());
        }
        assert!(!is_interactive());
    }
}
