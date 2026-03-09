use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct TerminalManager {
    terminals: HashMap<u32, TerminalInstance>,
    next_id: u32,
    pub output_buffer: Arc<Mutex<HashMap<u32, Vec<u8>>>>,
}

struct TerminalInstance {
    writer: Box<dyn Write + Send>,
    _master: Box<dyn MasterPty + Send>,
    alive: Arc<Mutex<bool>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            next_id: 1,
            output_buffer: Arc::new(Mutex::new(HashMap::new())),
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

        // Initialize buffer
        {
            let mut buf = output_buffer.lock().unwrap();
            buf.insert(id, Vec::new());
        }

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

    pub fn resize(&mut self, id: u32, cols: u16, rows: u16) -> Result<(), String> {
        if let Some(term) = self.terminals.get(&id) {
            term._master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Terminal not found".to_string())
        }
    }

    pub fn kill(&mut self, id: u32) {
        self.terminals.remove(&id);
        if let Ok(mut buf) = self.output_buffer.lock() {
            buf.remove(&id);
        }
    }

    pub fn read_output(&self, id: u32) -> Vec<u8> {
        if let Ok(mut buffers) = self.output_buffer.lock() {
            if let Some(output) = buffers.get_mut(&id) {
                let data = output.clone();
                output.clear();
                return data;
            }
        }
        Vec::new()
    }

    pub fn is_alive(&self, id: u32) -> bool {
        self.terminals
            .get(&id)
            .and_then(|t| t.alive.lock().ok())
            .map(|a| *a)
            .unwrap_or(false)
    }

    pub fn active_ids(&self) -> Vec<u32> {
        self.terminals.keys().cloned().collect()
    }
}

fn detect_shell() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}
