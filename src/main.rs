use std::{env, fs};

use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use pins::Pins;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::*, *},
};

pub mod pins;
pub mod tui;

pub struct App {
    file: String,
    bytes: Vec<u8>,
    state: AppState,
    exit: bool,
}

impl App {
    pub fn new(file: String, bytes: Vec<u8>) -> Self {
        Self {
            file,
            bytes,
            state: AppState::locked(),
            exit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
        match (&mut self.state, key_event.code) {
            (AppState::Locked { input }, KeyCode::Enter) if !input.is_empty() => {
                let master: u32 = input.parse().unwrap();
                let pins = Pins::load(&self.bytes, master);
                self.state = AppState::Unlocked(UnlockedState::new(pins));
            }
            (AppState::Locked { input }, KeyCode::Char(c @ '0'..='9')) if input.len() < 9 => {
                input.push(c);
            }
            (AppState::Locked { input }, KeyCode::Backspace) if !input.is_empty() => {
                input.pop();
            }
            (AppState::Unlocked(unlocked), KeyCode::Char('s')) if ctrl => {
                let bytes = unlocked.pins.save();
                fs::write(&self.file, &bytes)?;
                self.bytes = bytes;
            }
            (AppState::Unlocked { .. }, KeyCode::Esc) => {
                self.state = AppState::locked();
            }
            (AppState::Unlocked(unlocked), KeyCode::Char('k') | KeyCode::Up) => {
                unlocked.previous();
            }
            (AppState::Unlocked(unlocked), KeyCode::Char('j') | KeyCode::Down) => {
                unlocked.next();
            }
            (AppState::Unlocked(unlocked), KeyCode::Char('+')) => {
                unlocked.pins.add(0);
            }
            (AppState::Unlocked(unlocked), KeyCode::Char(c @ '0'..='9')) => 'blk: {
                let Some(i) = unlocked.state.selected() else {
                    break 'blk;
                };
                let pin = unlocked.pins.get(i).pin;
                if pin < 100_000_000 {
                    let digit = c as u32 - '0' as u32;
                    unlocked.pins.set(i, pin * 10 + digit);
                }
            }
            (AppState::Unlocked(unlocked), KeyCode::Backspace) => 'blk: {
                let Some(i) = unlocked.state.selected() else {
                    break 'blk;
                };
                let pin = unlocked.pins.get(i).pin;
                unlocked.pins.set(i, pin / 10);
            }
            (AppState::Unlocked(unlocked), KeyCode::Delete) => 'blk: {
                let Some(i) = unlocked.state.selected() else {
                    break 'blk;
                };
                unlocked.pins.remove(i);
                if unlocked.pins.is_empty() {
                    unlocked.unselect();
                } else if unlocked.pins.len() <= i {
                    unlocked.state.select(Some(i - 1));
                }
            }
            (_, KeyCode::Char('q')) => self.exit(),
            _ => (),
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from("Vault Crypt".bold());
        let instructions = Title::from(Line::from({
            let mut instructions = Vec::new();
            match &self.state {
                AppState::Locked { .. } => {
                    instructions.push(" Unlock".into());
                    instructions.push("<Enter>".blue().bold());
                }
                AppState::Unlocked { .. } => {
                    instructions.push(" Save".into());
                    instructions.push("<Ctrl-S>".blue().bold());
                    instructions.push(" New pin".into());
                    instructions.push("<+>".blue().bold());
                    instructions.push(" Remove pin".into());
                    instructions.push("<Del>".blue().bold());
                    instructions.push(" Lock".into());
                    instructions.push("<Esc>".blue().bold());
                }
            }
            instructions.push(" Quit".into());
            instructions.push("<Q>".blue().bold());
            instructions
        }));
        let block = Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::ALL)
            .border_set(border::THICK);

        match &mut self.state {
            AppState::Locked { input } => Paragraph::new(format!("Master Pin: {input:_<9}"))
                .centered()
                .block(block)
                .render(area, buf),
            AppState::Unlocked(unlocked) => {
                StatefulWidget::render(
                    List::new(
                        unlocked
                            .pins
                            .iter()
                            .map(|pin| format!("Pin {:2}: {:-<9}", pin.id, pin.pin)),
                    )
                    .highlight_style(Style::default().green())
                    .highlight_symbol(">>")
                    .repeat_highlight_symbol(true)
                    .direction(ListDirection::TopToBottom)
                    .block(block),
                    area,
                    buf,
                    &mut unlocked.state,
                );
            }
        }
    }
}

pub enum AppState {
    Locked { input: String },
    Unlocked(UnlockedState),
}

impl AppState {
    pub fn locked() -> Self {
        Self::Locked {
            input: String::new(),
        }
    }
}

pub struct UnlockedState {
    pins: Pins,
    state: ListState,
    last_selected: Option<usize>,
}

impl UnlockedState {
    pub fn new(pins: Pins) -> Self {
        Self {
            pins,
            state: ListState::default(),
            last_selected: None,
        }
    }

    pub fn next(&mut self) {
        if self.pins.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.pins.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.pins.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.pins.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.last_selected.unwrap_or(0),
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        let offset = self.state.offset();
        self.last_selected = self.state.selected();
        self.state.select(None);
        *self.state.offset_mut() = offset;
    }
}

fn main() -> Result<()> {
    let Some(file) = env::args().nth(1) else {
        bail!("Provide an input file");
    };
    let path = std::path::Path::new(&file);
    let bytes = if path.exists() {
        fs::read(&file)?
    } else {
        vec![0x00]
    };
    Pins::verify(&bytes)?;
    let mut terminal = tui::init()?;
    let app_result = App::new(file, bytes).run(&mut terminal);
    tui::restore()?;
    app_result
}
