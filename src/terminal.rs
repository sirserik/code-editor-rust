use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct TerminalManager {
    terminals: HashMap<u32, TerminalInstance>,
    next_id: u32,
    pub output_buffer: Arc<Mutex<HashMap<u32, Vec<u8>>>>,
    pub grids: HashMap<u32, TerminalGrid>,
}

struct TerminalInstance {
    writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
    alive: Arc<Mutex<bool>>,
}

/// Simple terminal character grid (Zed uses alacritty_terminal)
#[derive(Clone)]
pub struct TerminalGrid {
    pub cells: Vec<Vec<TermCell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub rows: usize,
    pub cols: usize,
    scroll_back: Vec<Vec<TermCell>>,
}

#[derive(Clone, Default)]
pub struct TermCell {
    pub ch: char,
    pub bold: bool,
    pub fg_ansi: Option<u8>, // ANSI color index (0-15)
}

impl TerminalGrid {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            cells: vec![vec![TermCell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            rows,
            cols,
            scroll_back: Vec::new(),
        }
    }

    /// Process raw bytes from PTY through a simple ANSI parser
    pub fn process_bytes(&mut self, data: &[u8]) {
        for &byte in data {
            match byte {
                b'\n' => {
                    self.cursor_row += 1;
                    if self.cursor_row >= self.rows {
                        self.scroll_up();
                    }
                }
                b'\r' => {
                    self.cursor_col = 0;
                }
                0x08 => { // backspace
                    if self.cursor_col > 0 { self.cursor_col -= 1; }
                }
                0x07 => {} // bell — ignore
                0x1b => {} // ESC — start of escape sequence (simplified: skip)
                b if b >= 0x20 => {
                    if self.cursor_col < self.cols && self.cursor_row < self.rows {
                        self.cells[self.cursor_row][self.cursor_col] = TermCell {
                            ch: byte as char,
                            bold: false,
                            fg_ansi: None,
                        };
                        self.cursor_col += 1;
                        if self.cursor_col >= self.cols {
                            self.cursor_col = 0;
                            self.cursor_row += 1;
                            if self.cursor_row >= self.rows {
                                self.scroll_up();
                            }
                        }
                    }
                }
                _ => {} // other control chars
            }
        }
    }

    fn scroll_up(&mut self) {
        if !self.cells.is_empty() {
            self.scroll_back.push(self.cells.remove(0));
            // Limit scrollback to 1000 lines
            if self.scroll_back.len() > 1000 {
                self.scroll_back.remove(0);
            }
        }
        self.cells.push(vec![TermCell::default(); self.cols]);
        self.cursor_row = self.rows - 1;
    }

    /// Get display lines as strings
    pub fn visible_lines(&self) -> Vec<String> {
        self.cells.iter().map(|row| {
            let s: String = row.iter().map(|c| if c.ch == '\0' { ' ' } else { c.ch }).collect();
            s.trim_end().to_string()
        }).collect()
    }
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            next_id: 1,
            output_buffer: Arc::new(Mutex::new(HashMap::new())),
            grids: HashMap::new(),
        }
    }

    pub fn spawn(&mut self, working_dir: Option<&str>) -> Result<u32, String> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let shell = detect_shell();
        let mut cmd = CommandBuilder::new(&shell);
        if let Some(dir) = working_dir {
            cmd.cwd(dir);
        }

        let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

        let id = self.next_id;
        self.next_id += 1;

        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

        let alive = Arc::new(Mutex::new(true));
        let alive_clone = alive.clone();
        let output_buffer = self.output_buffer.clone();

        // Initialize buffer and grid
        {
            let mut buf = output_buffer.lock().map_err(|e| format!("Mutex poisoned: {}", e))?;
            buf.insert(id, Vec::new());
        }
        self.grids.insert(id, TerminalGrid::new(24, 80));

        // Reader thread
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut buffers) = output_buffer.lock() {
                            if let Some(output) = buffers.get_mut(&id) {
                                output.extend_from_slice(&buf[..n]);
                                // Cap buffer at 1MB
                                const MAX_BUFFER: usize = 1024 * 1024;
                                if output.len() > MAX_BUFFER {
                                    let drain_to = output.len() - MAX_BUFFER;
                                    output.drain(..drain_to);
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut a) = alive_clone.lock() {
                *a = false;
            }
        });

        self.terminals.insert(
            id,
            TerminalInstance {
                writer,
                _master: pair.master,
                alive,
            },
        );

        Ok(id)
    }

    pub fn write(&mut self, id: u32, data: &[u8]) -> Result<(), String> {
        if let Some(term) = self.terminals.get_mut(&id) {
            term.writer.write_all(data).map_err(|e| e.to_string())?;
            term.writer.flush().map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Terminal not found".to_string())
        }
    }

    /// Read new output and process into grid
    pub fn update_grid(&mut self, id: u32) {
        let data = {
            if let Ok(mut buffers) = self.output_buffer.lock() {
                if let Some(output) = buffers.get_mut(&id) {
                    let data = output.clone();
                    output.clear();
                    data
                } else {
                    return;
                }
            } else {
                return;
            }
        };
        if !data.is_empty() {
            if let Some(grid) = self.grids.get_mut(&id) {
                grid.process_bytes(&data);
            }
        }
    }

    pub fn kill(&mut self, id: u32) {
        self.terminals.remove(&id);
        self.grids.remove(&id);
        if let Ok(mut buf) = self.output_buffer.lock() {
            buf.remove(&id);
        }
    }

    pub fn is_alive(&self, id: u32) -> bool {
        self.terminals
            .get(&id)
            .and_then(|t| t.alive.lock().ok())
            .map(|a| *a)
            .unwrap_or(false)
    }
}

fn detect_shell() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_grid_new() {
        let grid = TerminalGrid::new(24, 80);
        assert_eq!(grid.rows, 24);
        assert_eq!(grid.cols, 80);
        assert_eq!(grid.cursor_row, 0);
        assert_eq!(grid.cursor_col, 0);
    }

    #[test]
    fn terminal_grid_process_text() {
        let mut grid = TerminalGrid::new(24, 80);
        grid.process_bytes(b"Hello");
        assert_eq!(grid.cursor_col, 5);
        let lines = grid.visible_lines();
        assert!(lines[0].starts_with("Hello"));
    }

    #[test]
    fn terminal_grid_newline() {
        let mut grid = TerminalGrid::new(24, 80);
        grid.process_bytes(b"Line1\r\nLine2");
        let lines = grid.visible_lines();
        assert_eq!(lines[0], "Line1");
        assert!(lines[1].starts_with("Line2"));
    }

    #[test]
    fn terminal_grid_carriage_return() {
        let mut grid = TerminalGrid::new(24, 80);
        grid.process_bytes(b"AAAA\rBB");
        let lines = grid.visible_lines();
        assert!(lines[0].starts_with("BBAA"));
    }

    #[test]
    fn terminal_grid_scroll() {
        let mut grid = TerminalGrid::new(3, 80);
        grid.process_bytes(b"line1\nline2\nline3\nline4");
        // After 4 lines in a 3-row grid, should have scrolled
        assert_eq!(grid.cursor_row, 2); // last row
        assert_eq!(grid.scroll_back.len(), 1);
    }

    #[test]
    fn terminal_grid_backspace() {
        let mut grid = TerminalGrid::new(24, 80);
        grid.process_bytes(b"ABC\x08"); // ABC then backspace
        assert_eq!(grid.cursor_col, 2);
    }

    #[test]
    fn terminal_grid_wrap_at_end() {
        let mut grid = TerminalGrid::new(24, 5); // 5-column grid
        grid.process_bytes(b"12345X");
        // "12345" fills row, "X" wraps to next row
        assert_eq!(grid.cursor_row, 1);
        assert_eq!(grid.cursor_col, 1);
    }

    #[test]
    fn terminal_manager_new() {
        let tm = TerminalManager::new();
        assert!(tm.grids.is_empty());
    }

    #[test]
    fn detect_shell_not_empty() {
        let shell = detect_shell();
        assert!(!shell.is_empty());
    }
}
