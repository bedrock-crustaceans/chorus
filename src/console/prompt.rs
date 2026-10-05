use super::complete::{Analysis, Completions};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, queue, terminal};
use indicatif::{ProgressDrawTarget, TermLike};
use std::io::{Write, stdout};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

const PROMPT: &str = "\x1b[1;36m❯\x1b[0m ";
const ECHO_PROMPT: &str = "\x1b[90m❯\x1b[0m ";
const PROMPT_WIDTH: usize = 2;
const MAX_HISTORY: usize = 100;
const BAR_REFRESH_HZ: u8 = 10;

static STATE: Mutex<Option<Prompt>> = Mutex::new(None);
static BARS: Mutex<Vec<Weak<Mutex<Frame>>>> = Mutex::new(Vec::new());
static COMPLETIONS: Mutex<Option<Completions>> = Mutex::new(None);

#[derive(Default)]
struct Prompt {
    visible: bool,
    cycle: Option<Cycle>,
    line: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    browsing: Option<usize>,
    draft: Vec<char>,
    pinned: usize,
    cursor_row: usize,
    panel: Vec<String>,
    scroll: usize,
}

struct Cycle {
    start: usize,
    candidates: Vec<String>,
    selected: Option<usize>,
}

pub enum Input {
    Line(String),
    Interrupt,
}

impl Prompt {
    fn clear(&mut self, out: &mut impl Write) {
        if !self.visible {
            return;
        }
        let up = self.pinned + self.cursor_row;
        if up > 0 {
            let _ = queue!(out, cursor::MoveUp(up as u16));
        }
        let _ = queue!(out, cursor::MoveToColumn(0), terminal::Clear(terminal::ClearType::FromCursorDown));
        self.pinned = 0;
        self.cursor_row = 0;
    }

    fn draw(&mut self, out: &mut impl Write) {
        if !self.visible {
            return;
        }
        let analysis = self.analysis();
        let mut pinned = pinned_lines();
        pinned.extend(self.panel_view(pinned.len()));
        for line in &pinned {
            let _ = write!(out, "{line}\x1b[0m\r\n");
        }
        self.pinned = pinned.len();
        let line: String = self.line.iter().collect();
        let ghost = self.ghost(&analysis).unwrap_or_default();
        let _ = write!(out, "{PROMPT}{line}\x1b[90m{ghost}\x1b[0m");
        let width = terminal::size().map_or(80, |(width, _)| width as usize).max(1);
        let length = PROMPT_WIDTH + self.line.len() + ghost.chars().count();
        if length % width == 0 {
            let _ = write!(out, "\r\n");
        }
        let target = PROMPT_WIDTH + self.cursor;
        let up = length / width - target / width;
        if up > 0 {
            let _ = queue!(out, cursor::MoveUp(up as u16));
        }
        let _ = queue!(out, cursor::MoveToColumn((target % width) as u16));
        self.cursor_row = target / width;
    }

    fn panel_view(&mut self, reserved: usize) -> Vec<String> {
        let (width, height) = terminal::size().map_or((80, 24), |(width, height)| (width as usize, height as usize));
        let lines: Vec<String> = self.panel.iter().flat_map(|line| wrap(line, width.saturating_sub(1).max(1))).collect();
        let room = height.saturating_sub(reserved + 2).max(2);
        if lines.len() <= room {
            self.scroll = 0;
            return lines;
        }
        let shown = room - 1;
        self.scroll = self.scroll.min(lines.len() - shown);
        let mut view = lines[self.scroll..self.scroll + shown].to_vec();
        view.push(format!(
            "\x1b[90m{}-{} of {} lines, pgup/pgdn to scroll, esc to dismiss\x1b[0m",
            self.scroll + 1,
            self.scroll + shown,
            lines.len()
        ));
        view
    }

    fn analysis(&self) -> Analysis {
        let before: String = self.line[..self.cursor].iter().collect();
        let completions = COMPLETIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match completions.as_ref() {
            Some(completions) if !before.trim().is_empty() => completions.analyze(&before),
            _ => Analysis::default(),
        }
    }

    fn ghost(&self, analysis: &Analysis) -> Option<String> {
        let typed = self.cursor - analysis.start;
        if self.cursor != self.line.len() || typed == 0 {
            return None;
        }
        let candidate = analysis.candidates.iter().find(|candidate| candidate.chars().count() > typed)?;
        Some(candidate.chars().skip(typed).collect())
    }

    fn replace_token(&mut self, start: usize, text: &str, finish: bool) {
        self.line.splice(start..self.cursor, text.chars());
        self.cursor = start + text.chars().count();
        if finish && self.line.get(self.cursor) != Some(&' ') {
            self.line.insert(self.cursor, ' ');
        }
        if finish {
            self.cursor += 1;
        }
    }

    fn complete(&mut self, forward: bool) {
        if let Some(cycle) = &mut self.cycle {
            let count = cycle.candidates.len();
            let selected = match (cycle.selected, forward) {
                (None, true) => 0,
                (None, false) => count - 1,
                (Some(index), true) => (index + 1) % count,
                (Some(index), false) => (index + count - 1) % count,
            };
            cycle.selected = Some(selected);
            let (start, candidate) = (cycle.start, cycle.candidates[selected].clone());
            self.replace_token(start, &candidate, false);
            return;
        }
        let analysis = self.analysis();
        let typed = self.cursor - analysis.start;
        match analysis.candidates.as_slice() {
            [] => {}
            [only] => self.replace_token(analysis.start, only, true),
            many => {
                let common = common_prefix(many);
                if common.chars().count() > typed {
                    self.replace_token(analysis.start, &common, false);
                } else {
                    self.cycle = Some(Cycle {
                        start: analysis.start,
                        candidates: many.to_vec(),
                        selected: None,
                    });
                    self.complete(forward);
                }
            }
        }
    }

    fn redraw(&mut self, out: &mut impl Write) {
        self.clear(out);
        self.draw(out);
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
        if !matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            self.cycle = None;
        }
        match key.code {
            KeyCode::Char('c') if control => return Some(Input::Interrupt),
            KeyCode::Tab => self.complete(true),
            KeyCode::Esc => {
                self.panel.clear();
                self.scroll = 0;
            }
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(terminal::size().map_or(10, |(_, height)| height as usize / 2)),
            KeyCode::PageDown => self.scroll += terminal::size().map_or(10, |(_, height)| height as usize / 2),
            KeyCode::BackTab => self.complete(false),
            KeyCode::Right if self.cursor == self.line.len() => {
                let analysis = self.analysis();
                if let Some(ghost) = self.ghost(&analysis) {
                    self.line.extend(ghost.chars());
                    self.cursor = self.line.len();
                }
            }
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
                self.clear(out);
                if !line.trim().is_empty() {
                    self.panel = vec![format!("{ECHO_PROMPT}\x1b[1m{}\x1b[0m", line.trim())];
                    self.scroll = 0;
                }
                self.draw(out);
                let line = line.trim().to_owned();
                if line.is_empty() {
                    return None;
                }
                if self.history.last() != Some(&line) {
                    self.history.push(line.clone());
                }
                if self.history.len() > MAX_HISTORY {
                    self.history.remove(0);
                }
                return Some(Input::Line(line));
            }
            _ => {}
        }
        self.redraw(out);
        None
    }
}

pub fn start(visible: bool) -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut prompt = Prompt { visible, ..Default::default() };
    let mut out = stdout().lock();
    prompt.draw(&mut out);
    let _ = out.flush();
    *STATE.lock().expect("console prompt lock poisoned") = Some(prompt);
    Ok(())
}

pub fn stop() {
    let Some(mut prompt) = STATE.lock().map(|mut state| state.take()).ok().flatten() else { return };
    let mut out = stdout().lock();
    prompt.clear(&mut out);
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
                prompt.redraw(&mut out);
            }
            Ok(Event::Resize(..)) => prompt.redraw(&mut out),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = out.flush();
    inputs
}

pub fn print(bytes: &[u8]) {
    let mut state = STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut out = stdout().lock();
    let Some(prompt) = state.as_mut().filter(|prompt| prompt.visible) else {
        let _ = out.write_all(bytes);
        return;
    };
    prompt.clear(&mut out);
    for line in bytes.split_inclusive(|&byte| byte == b'\n') {
        let _ = out.write_all(line.strip_suffix(b"\n").unwrap_or(line));
        if line.ends_with(b"\n") {
            let _ = out.write_all(b"\r\n");
        }
    }
    prompt.draw(&mut out);
    let _ = out.flush();
}

pub fn set_completions(completions: Completions) {
    *COMPLETIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(completions);
    refresh();
}

fn common_prefix(values: &[String]) -> String {
    let first: Vec<char> = values[0].chars().collect();
    let length = values[1..].iter().fold(first.len(), |length, value| {
        first.iter().zip(value.chars()).take(length).take_while(|(a, b)| a.eq_ignore_ascii_case(b)).count()
    });
    first[..length].iter().collect()
}

pub fn reply(text: &str) {
    let mut state = STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(prompt) = state.as_mut().filter(|prompt| prompt.visible) else {
        drop(state);
        print(format!("{text}\n").as_bytes());
        return;
    };
    let mut out = stdout().lock();
    prompt.clear(&mut out);
    prompt.panel.extend(text.lines().map(str::to_owned));
    prompt.draw(&mut out);
    let _ = out.flush();
}

fn wrap(line: &str, width: usize) -> Vec<String> {
    let mut rows = vec![String::new()];
    let mut style = String::new();
    let mut used = 0;
    let mut chars = line.chars().peekable();
    while let Some(char) = chars.next() {
        if char == '\x1b' {
            let mut sequence = String::from(char);
            for next in chars.by_ref() {
                sequence.push(next);
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            style = if sequence == "\x1b[0m" { String::new() } else { format!("{style}{sequence}") };
            rows.last_mut().expect("rows is never empty").push_str(&sequence);
            continue;
        }
        if used == width {
            rows.push(style.clone());
            used = 0;
        }
        rows.last_mut().expect("rows is never empty").push(char);
        used += 1;
    }
    rows
}

fn refresh() {
    let mut state = STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(prompt) = state.as_mut().filter(|prompt| prompt.visible) else { return };
    let mut out = stdout().lock();
    prompt.redraw(&mut out);
    let _ = out.flush();
}

pub fn draw_target() -> ProgressDrawTarget {
    let visible = STATE.lock().ok().is_some_and(|state| state.as_ref().is_some_and(|prompt| prompt.visible));
    if !visible {
        return ProgressDrawTarget::hidden();
    }
    let frame = Arc::new(Mutex::new(Frame::default()));
    BARS.lock().expect("console bars lock poisoned").push(Arc::downgrade(&frame));
    ProgressDrawTarget::term_like_with_hz(Box::new(Capture(frame)), BAR_REFRESH_HZ)
}

fn pinned_lines() -> Vec<String> {
    let mut bars = BARS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    bars.retain(|frame| frame.strong_count() > 0);
    bars.iter()
        .filter_map(Weak::upgrade)
        .flat_map(|frame| {
            let frame = frame.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let lines: Vec<String> = frame.lines.iter().map(|line| line.trim_end().to_owned()).collect();
            let end = lines.iter().rposition(|line| !line.is_empty()).map_or(0, |index| index + 1);
            lines.into_iter().take(end).collect::<Vec<_>>()
        })
        .collect()
}

#[derive(Debug, Default)]
struct Frame {
    lines: Vec<String>,
    row: usize,
}

impl Frame {
    fn line(&mut self) -> &mut String {
        if self.lines.len() <= self.row {
            self.lines.resize(self.row + 1, String::new());
        }
        &mut self.lines[self.row]
    }
}

#[derive(Debug)]
struct Capture(Arc<Mutex<Frame>>);

impl Capture {
    fn with(&self, f: impl FnOnce(&mut Frame)) -> std::io::Result<()> {
        f(&mut self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
        Ok(())
    }
}

impl TermLike for Capture {
    fn width(&self) -> u16 {
        terminal::size().map_or(80, |(width, _)| width.saturating_sub(1))
    }

    fn move_cursor_up(&self, n: usize) -> std::io::Result<()> {
        self.with(|frame| frame.row = frame.row.saturating_sub(n))
    }

    fn move_cursor_down(&self, n: usize) -> std::io::Result<()> {
        self.with(|frame| frame.row += n)
    }

    fn move_cursor_right(&self, _: usize) -> std::io::Result<()> {
        Ok(())
    }

    fn move_cursor_left(&self, _: usize) -> std::io::Result<()> {
        Ok(())
    }

    fn write_line(&self, s: &str) -> std::io::Result<()> {
        self.with(|frame| {
            frame.line().push_str(&s.replace('\r', ""));
            frame.row += 1;
        })
    }

    fn write_str(&self, s: &str) -> std::io::Result<()> {
        self.with(|frame| frame.line().push_str(&s.replace('\r', "")))
    }

    fn clear_line(&self) -> std::io::Result<()> {
        self.with(|frame| frame.line().clear())
    }

    fn flush(&self) -> std::io::Result<()> {
        refresh();
        Ok(())
    }
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
