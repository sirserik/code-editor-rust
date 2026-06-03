//! Output panel: captures stdout/stderr of external commands (cargo, npm, make, …)
//! and parses `path:line[:col]` patterns into clickable jump targets.
//!
//! Runs the child on background threads — never blocks the render loop.
//! `stop()` only sends SIGKILL; a polling waiter detects exit and reports the code.

use regex::Regex;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const MAX_LINES: usize = 10_000;
const WAIT_POLL_MS: u64 = 80;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputKind {
    System, // banner lines like "$ cargo build" or "exited (0)"
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct FileLink {
    pub path: PathBuf, // absolute (resolved against cwd if relative)
    pub line: usize,   // 0-indexed
    pub col: usize,    // 0-indexed
}

#[derive(Debug, Clone)]
pub struct OutputLine {
    pub text: String,
    pub kind: OutputKind,
    pub link: Option<FileLink>,
}

enum Event {
    Line(OutputLine),
    Exit(Option<i32>),
}

pub struct OutputConsole {
    pub lines: Vec<OutputLine>,
    pub auto_scroll: bool,
    pub running: bool,
    pub exit_code: Option<i32>,
    pub last_command: Option<LastCommand>,
    rx: Option<mpsc::Receiver<Event>>,
    child: Arc<Mutex<Option<Child>>>,
}

#[derive(Debug, Clone)]
pub struct LastCommand {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

impl Default for OutputConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputConsole {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            auto_scroll: true,
            running: false,
            exit_code: None,
            last_command: None,
            rx: None,
            child: Arc::new(Mutex::new(None)),
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.exit_code = None;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Best-effort kill of the running child. Safe to call when nothing is running.
    /// The waiter thread will pick up the exit on its next poll and emit `Exit(...)`.
    pub fn stop(&mut self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(child) = guard.as_mut() {
                let _ = child.kill();
            }
        }
    }

    /// Spawn `command` with `args`, capturing stdout + stderr line-by-line on background
    /// threads. Replaces any previous run. Returns Err only on spawn failure.
    pub fn run(
        &mut self,
        command: &str,
        args: &[String],
        cwd: Option<&str>,
    ) -> Result<(), String> {
        // Reap the previous child if any. We don't block waiting for exit —
        // the kill is asynchronous, but we drop our reference here so the
        // next run gets a fresh state slot.
        self.stop();
        // Drop the previous receiver — its waiter thread may still be running
        // briefly; that's fine, its Exit message just gets discarded.
        self.rx = None;

        let label = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };
        self.push_line(OutputLine {
            text: format!("$ {}", label),
            kind: OutputKind::System,
            link: None,
        });

        let cwd_path: Option<PathBuf> = cwd.map(PathBuf::from);

        let mut cmd = Command::new(command);
        cmd.args(args);
        if let Some(dir) = cwd_path.as_ref() {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("Failed to start `{}`: {}", command, e);
                self.push_line(OutputLine {
                    text: msg.clone(),
                    kind: OutputKind::System,
                    link: None,
                });
                return Err(msg);
            }
        };
        let stdout = child.stdout.take().ok_or("no stdout pipe")?;
        let stderr = child.stderr.take().ok_or("no stderr pipe")?;

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        if let Ok(mut g) = self.child.lock() {
            *g = Some(child);
        }
        self.running = true;
        self.exit_code = None;
        self.last_command = Some(LastCommand {
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.map(String::from),
        });

        let cwd_for_stdout = cwd_path.clone();
        let tx_stdout = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let link = parse_file_link(&line, cwd_for_stdout.as_deref());
                if tx_stdout
                    .send(Event::Line(OutputLine {
                        text: line,
                        kind: OutputKind::Stdout,
                        link,
                    }))
                    .is_err()
                {
                    break;
                }
            }
        });

        let cwd_for_stderr = cwd_path;
        let tx_stderr = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let link = parse_file_link(&line, cwd_for_stderr.as_deref());
                if tx_stderr
                    .send(Event::Line(OutputLine {
                        text: line,
                        kind: OutputKind::Stderr,
                        link,
                    }))
                    .is_err()
                {
                    break;
                }
            }
        });

        // Waiter: polls `try_wait()` so `stop()` can grab the mutex briefly
        // and call `kill()` without deadlocking on a blocking wait.
        let child_arc = Arc::clone(&self.child);
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(WAIT_POLL_MS));
                let result = {
                    let mut guard = match child_arc.lock() {
                        Ok(g) => g,
                        Err(_) => {
                            let _ = tx.send(Event::Exit(None));
                            return;
                        }
                    };
                    let Some(child) = guard.as_mut() else {
                        // Someone (next `run()`) already cleared us. Bail silently.
                        return;
                    };
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            *guard = None;
                            Some(status.code())
                        }
                        Ok(None) => None,
                        Err(_) => {
                            *guard = None;
                            Some(None)
                        }
                    }
                };
                if let Some(code) = result {
                    let _ = tx.send(Event::Exit(code));
                    return;
                }
            }
        });

        Ok(())
    }

    /// Drain pending events from the background threads. Call every render frame
    /// while the panel is visible. Returns true if anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        // Take rx out temporarily so we can borrow self mutably for push_line.
        if let Some(rx) = self.rx.take() {
            loop {
                match rx.try_recv() {
                    Ok(Event::Line(line)) => {
                        changed = true;
                        self.push_line(line);
                    }
                    Ok(Event::Exit(code)) => {
                        changed = true;
                        self.running = false;
                        self.exit_code = code;
                        let msg = match code {
                            Some(0) => "Process finished (exit 0)".to_string(),
                            Some(n) => format!("Process exited with code {}", n),
                            None => "Process terminated".to_string(),
                        };
                        self.push_line(OutputLine {
                            text: msg,
                            kind: OutputKind::System,
                            link: None,
                        });
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        self.rx = Some(rx);
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        // All senders dropped — process is fully done.
                        break;
                    }
                }
            }
        }
        changed
    }

    fn push_line(&mut self, line: OutputLine) {
        self.lines.push(line);
        if self.lines.len() > MAX_LINES {
            let drop = self.lines.len() - MAX_LINES;
            self.lines.drain(..drop);
        }
    }
}

/// Cached regex: `path/to/file.ext:LINE[:COL]`. Skips URLs (`://`).
fn link_regex() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Path: at least one `.` followed by a typical-looking extension (2–8 alnum chars).
        // This avoids matching things like `example.com:8080` (com would need >=2 chars
        // alnum, which it has, but the regex requires `.ext:digit+` so it still matches —
        // the URL guard below handles that case).
        Regex::new(r"([A-Za-z0-9_./\\-]+\.[A-Za-z0-9]{1,8}):(\d+)(?::(\d+))?")
            .expect("output link regex compiles")
    })
}

fn parse_file_link(line: &str, cwd: Option<&Path>) -> Option<FileLink> {
    if line.contains("://") {
        return None; // skip URLs
    }
    let caps = link_regex().captures(line)?;
    let path_str = caps.get(1)?.as_str();
    let line_num: usize = caps.get(2)?.as_str().parse().ok()?;
    if line_num == 0 {
        return None;
    }
    let col_num: usize = caps
        .get(3)
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(1);
    let path = Path::new(path_str);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(c) = cwd {
        c.join(path)
    } else {
        path.to_path_buf()
    };
    Some(FileLink {
        path: resolved,
        line: line_num.saturating_sub(1),
        col: col_num.saturating_sub(1),
    })
}

/// Detect a sensible default build command for the project root.
/// Returns (label, command, args).
pub fn detect_build(root: &Path) -> Option<(&'static str, String, Vec<String>)> {
    if root.join("Cargo.toml").is_file() {
        return Some(("Cargo build", "cargo".into(), vec!["build".into()]));
    }
    if root.join("package.json").is_file() {
        if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
            if text.contains("\"build\"") {
                return Some(("npm build", "npm".into(), vec!["run".into(), "build".into()]));
            }
        }
        return Some(("npm install", "npm".into(), vec!["install".into()]));
    }
    if root.join("Makefile").is_file() || root.join("makefile").is_file() {
        return Some(("make", "make".into(), Vec::new()));
    }
    if root.join("composer.json").is_file() {
        return Some(("composer install", "composer".into(), vec!["install".into()]));
    }
    None
}

/// Detect a sensible default test command. Same idea as `detect_build`.
pub fn detect_test(root: &Path) -> Option<(&'static str, String, Vec<String>)> {
    if root.join("Cargo.toml").is_file() {
        return Some(("Cargo test", "cargo".into(), vec!["test".into()]));
    }
    if root.join("package.json").is_file() {
        if let Ok(text) = std::fs::read_to_string(root.join("package.json")) {
            if text.contains("\"test\"") {
                return Some(("npm test", "npm".into(), vec!["test".into()]));
            }
        }
    }
    if root.join("pytest.ini").is_file()
        || root.join("pyproject.toml").is_file()
        || root.join("setup.py").is_file()
    {
        return Some(("pytest", "pytest".into(), Vec::new()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cargo_style_path() {
        let link = parse_file_link("   --> src/main.rs:42:5", None).unwrap();
        assert!(link.path.ends_with("src/main.rs"));
        assert_eq!(link.line, 41);
        assert_eq!(link.col, 4);
    }

    #[test]
    fn parses_path_with_line_only() {
        let link = parse_file_link("error in src/foo.rs:7", None).unwrap();
        assert_eq!(link.line, 6);
        assert_eq!(link.col, 0);
    }

    #[test]
    fn resolves_relative_against_cwd() {
        let cwd = PathBuf::from("/tmp/proj");
        let link = parse_file_link("src/lib.rs:1:1", Some(&cwd)).unwrap();
        assert_eq!(link.path, PathBuf::from("/tmp/proj/src/lib.rs"));
    }

    #[test]
    fn keeps_absolute_path() {
        let link = parse_file_link("at /usr/local/x.rs:3", None).unwrap();
        assert_eq!(link.path, PathBuf::from("/usr/local/x.rs"));
        assert_eq!(link.line, 2);
    }

    #[test]
    fn skips_urls() {
        assert!(parse_file_link("http://example.com:8080/x", None).is_none());
        assert!(parse_file_link("see https://x.org:443/y.rs:1", None).is_none());
    }

    #[test]
    fn skips_line_zero() {
        assert!(parse_file_link("a.rs:0:0", None).is_none());
    }

    #[test]
    fn output_console_new_clean_state() {
        let oc = OutputConsole::new();
        assert!(!oc.is_running());
        assert!(oc.lines.is_empty());
        assert!(oc.exit_code.is_none());
    }

    #[test]
    fn clear_resets_lines_and_exit() {
        let mut oc = OutputConsole::new();
        oc.lines.push(OutputLine {
            text: "x".into(),
            kind: OutputKind::Stdout,
            link: None,
        });
        oc.exit_code = Some(1);
        oc.clear();
        assert!(oc.lines.is_empty());
        assert!(oc.exit_code.is_none());
    }

    #[test]
    fn detect_build_finds_cargo() {
        let dir = std::env::temp_dir().join(format!("oc_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let detected = detect_build(&dir);
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().1, "cargo");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
