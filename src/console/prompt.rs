use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, queue, terminal};
use std::io::{Write, stdout};
use std::sync::Mutex;
use std::time::Duration;

const PROMPT: &str = "> ";
const MAX_HISTORY: usize = 100;

static STATE: Mutex<Option<Prompt>> = Mutex::new(None);

#[derive(Default)]
struct Prompt {
    line: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    browsing: Option<usize>,
    draft: Vec<char>,
}

pub enum Input {
    Line(String),
    Interrupt,
}

impl Prompt {
    fn draw(&self, out: &mut impl Write) {
        let line: String = self.line.iter().collect();
        let _ = queue!(out, cursor::MoveToColumn(0), terminal::Clear(terminal::ClearType::CurrentLine));
        let _ = write!(out, "{PROMPT}{line}");
        let _ = queue!(out, cursor::MoveToColumn((PROMPT.len() + self.cursor) as u16));
    }

    fn recall(&mut self, index: Option<usize>) {
        if self.browsing.is_none() {
            self.draft = self.line.clone();
        }
        self.browsing = index;
        self.line = match index {
            Some(index) => self.history[index].chars().collect(),
            None => std::mem::take(&mut self.draft),
        };
        self.cursor = self.line.len();
    }

    fn handle(&mut self, key: KeyEvent, out: &mut impl Write) -> Option<Input> {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('c') if control => return Some(Input::Interrupt),
            KeyCode::Char('u') if control => {
                self.line.drain(..self.cursor);
                self.cursor = 0;
            }
            KeyCode::Char(char) if !control => {
                self.line.insert(self.cursor, char);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.line.remove(self.cursor);
            }
            KeyCode::Delete if self.cursor < self.line.len() => {
                self.line.remove(self.cursor);
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.line.len()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.line.len(),
            KeyCode::Up if !self.history.is_empty() => {
                let index = self.browsing.map_or(self.history.len() - 1, |index| index.saturating_sub(1));
                self.recall(Some(index));
            }
            KeyCode::Down if self.browsing.is_some() => {
                let next = self.browsing.map(|index| index + 1).filter(|&index| index < self.history.len());
                self.recall(next);
            }
            KeyCode::Enter => {
                let line: String = std::mem::take(&mut self.line).into_iter().collect();
                self.cursor = 0;
                self.browsing = None;
                self.draft.clear();
                let _ = queue!(out, cursor::MoveToColumn(0), terminal::Clear(terminal::ClearType::CurrentLine));
                let _ = write!(out, "{PROMPT}{line}\r\n");
                let line = line.trim().to_owned();
                if !line.is_empty() {
                    if self.history.last() != Some(&line) {
                        self.history.push(line.clone());
                    }
                    if self.history.len() > MAX_HISTORY {
                        self.history.remove(0);
                    }
                    self.draw(out);
                    return Some(Input::Line(line));
                }
            }
            _ => {}
        }
        self.draw(out);
        None
    }
}

pub fn start() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let prompt = Prompt::default();
    let mut out = stdout().lock();
    prompt.draw(&mut out);
    let _ = out.flush();
    *STATE.lock().expect("console prompt lock poisoned") = Some(prompt);
    Ok(())
}

pub fn stop() {
    let Some(_) = STATE.lock().map(|mut state| state.take()).ok().flatten() else { return };
    let mut out = stdout().lock();
    let _ = queue!(out, cursor::MoveToColumn(0), terminal::Clear(terminal::ClearType::CurrentLine));
    let _ = out.flush();
    let _ = terminal::disable_raw_mode();
}

pub fn read() -> Vec<Input> {
    let mut inputs = Vec::new();
    let mut state = STATE.lock().expect("console prompt lock poisoned");
    let Some(prompt) = state.as_mut() else { return inputs };
    let mut out = stdout().lock();
    while event::poll(Duration::ZERO).unwrap_or(false) {
        match event::read() {
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => inputs.extend(prompt.handle(key, &mut out)),
            Ok(Event::Paste(text)) => {
                for char in text.chars().filter(|char| !char.is_control()) {
                    prompt.line.insert(prompt.cursor, char);
                    prompt.cursor += 1;
                }
                prompt.draw(&mut out);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = out.flush();
    inputs
}

pub fn print(bytes: &[u8]) {
    let state = STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut out = stdout().lock();
    let Some(prompt) = state.as_ref() else {
        let _ = out.write_all(bytes);
        return;
    };
    let _ = queue!(out, cursor::MoveToColumn(0), terminal::Clear(terminal::ClearType::CurrentLine));
    for line in bytes.split_inclusive(|&byte| byte == b'\n') {
        let _ = out.write_all(line.strip_suffix(b"\n").unwrap_or(line));
        if line.ends_with(b"\n") {
            let _ = out.write_all(b"\r\n");
        }
    }
    prompt.draw(&mut out);
    let _ = out.flush();
}

#[derive(Default)]
pub struct ConsoleWriter(Vec<u8>);

impl Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for ConsoleWriter {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            print(&self.0);
        }
    }
}
