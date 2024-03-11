use std::{
    iter,
    sync::mpsc::{self, SyncSender},
    thread,
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::*, *},
};
use windows::{
    core::{s, PCSTR},
    Win32::UI::{
        Input::KeyboardAndMouse::{EnableWindow, SetActiveWindow, SetCapture, SetFocus},
        WindowsAndMessaging::{FindWindowA, GetForegroundWindow, SetForegroundWindow},
    },
};

use crate::{
    btd::{
        summary::{GameSummary, InGameSummary, Tower},
        BloonsGame, BloonsHistogram,
    },
    win32_util, Previous, Result,
};

mod tui;

enum AppEvent {
    Summary(GameSummary),
    Exit,
}

struct InputThread {
    out: SyncSender<AppEvent>,
}

impl InputThread {
    fn new(out: SyncSender<AppEvent>) -> Self {
        Self { out }
    }

    fn run(&self) -> Result<()> {
        loop {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    if key_event.code == KeyCode::Esc {
                        self.out.send(AppEvent::Exit)?;
                    }
                }

                _ => {} // e => println!("{:?}", e),
            };
        }
    }
}

struct SummaryThread {
    out: SyncSender<AppEvent>,
    game: BloonsGame,
}

impl SummaryThread {
    fn new(out: SyncSender<AppEvent>, game: BloonsGame) -> Self {
        Self { out, game }
    }

    fn run(&mut self) -> Result<()> {
        let mut previous = Previous::default();

        loop {
            let state = self.game.get_summary();

            if let (GameSummary::InGame(a), Some(GameSummary::InGame(b))) =
                (&state, &previous.value)
            {
                let do_beep = a
                    .upgrades
                    .iter()
                    .filter(|up| match a.selected_index {
                        Some(i) => i == up.tower_index,
                        None => true,
                    })
                    .any(|upgrade| (b.cash..a.cash).contains(&upgrade.cost));

                if do_beep {
                    win32_util::beep();
                }

                if is_pause(a) && !is_pause(b) {
                    unsafe {
                        let hwnd = FindWindowA(PCSTR::null(), s!("BloonsTD6-Epic"));

                        while hwnd != GetForegroundWindow() {
                            SetForegroundWindow(hwnd);
                            SetCapture(hwnd);
                            SetFocus(hwnd);
                            SetActiveWindow(hwnd);
                            thread::sleep(Duration::from_millis(1));
                        }
                    }

                    while !self
                        .game
                        .get_ingame()?
                        .unwrap()
                        .stopped_clock_for_menu_open()?
                    {
                        win32_util::send_input(&win32_util::make_keypress_scancode(0x29));
                        thread::sleep(Duration::from_millis(1));
                    }
                }
            }

            if previous.set(state.clone()) {
                self.out.send(AppEvent::Summary(state))?;
            }

            thread::sleep(Duration::from_millis(25));
        }
    }
}

struct BloonsThread {
    out: SyncSender<AppEvent>,
    game: BloonsGame,
    histogram: BloonsHistogram,
}

impl BloonsThread {
    fn new(out: SyncSender<AppEvent>, game: BloonsGame) -> Self {
        let histogram = BloonsHistogram::new(256);

        Self {
            out,
            game,
            histogram,
        }
    }

    fn run(&mut self) -> Result<()> {
        loop {
            match self.game.try_get_bloons() {
                Err(e) => eprintln!("{:?}", e),
                Ok(None) => continue,
                Ok(Some(info)) => {
                    for bloon in info.bloons {
                        self.histogram.add_one(bloon.distance / info.max_path);
                    }
                }
            }

            thread::sleep(Duration::from_millis(1000));
        }
    }
}

fn is_pause(summary: &InGameSummary) -> bool {
    summary.danger.is_some_and(|d| d < 30.0)
}

#[derive(Debug)]
pub struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self) -> Result<()> {
        let game = BloonsGame::find_game()?;

        let mut terminal = tui::init()?;

        let (send, recv) = mpsc::sync_channel(8);

        let mut game_thread = SummaryThread::new(send.clone(), game.clone());
        let input_thread = InputThread::new(send.clone());

        thread::spawn(move || game_thread.run().unwrap());
        thread::spawn(move || input_thread.run().unwrap());

        while let Ok(event) = recv.recv() {
            match event {
                AppEvent::Summary(state) => self.render(&mut terminal, state)?,

                AppEvent::Exit => break,
            }
        }

        tui::restore()?;

        Ok(())
    }

    fn render(&self, terminal: &mut tui::Tui, summary: GameSummary) -> Result<()> {
        terminal.draw(|frame| frame.render_widget(&summary, frame.size()))?;

        Ok(())
    }
}

impl Widget for &GameSummary {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            GameSummary::None => {
                let title = Title::from(" Not in game ".bold());

                Block::default()
                    .title(title.alignment(Alignment::Center))
                    .borders(Borders::ALL)
                    .border_set(border::THICK)
                    .render(area, buf);
            }

            GameSummary::InGame(state) => {
                state.render(area, buf);
            }
        }
    }
}

fn render_towers_table(area: Rect, buf: &mut Buffer, state: &InGameSummary) {
    let rows: Vec<Row<'_>> = state
        .towers
        .iter()
        .enumerate()
        .map(|(i, tower)| {
            let style = if Some(i) == state.selected_index {
                Style::new().bold()
            } else {
                Style::new()
            };

            let row = match tower {
                Tower::Basic(tower) => Row::new([
                    Text::raw(format!("${}", tower.worth)).alignment(Alignment::Right),
                    Text::raw(format!(
                        "{}-{}-{}",
                        tower.tiers[0], tower.tiers[1], tower.tiers[2]
                    )),
                    Text::raw(format!("{}", tower.name)),
                ]),

                Tower::Hero(hero) => Row::new([
                    Text::raw(format!("${}", hero.worth)).alignment(Alignment::Right),
                    Text::raw(format!("{}", hero.level)).alignment(Alignment::Right),
                    Text::raw(format!("{}", hero.name)),
                ]),
            };

            row.style(style)
        })
        .collect::<Vec<_>>();

    let columns = [
        // Constraint::Min(2),
        Constraint::Min(7),
        Constraint::Min(5),
        Constraint::Percentage(100),
    ];

    let table = Table::new(rows, columns);

    Widget::render(table, area, buf);
}

fn render_upgrades_table(area: Rect, buf: &mut Buffer, state: &InGameSummary) {
    let index = state
        .upgrades
        .iter()
        .position(|s| s.cost > state.cash)
        .unwrap_or(state.upgrades.len());

    let rows: Vec<Row<'_>> = state
        .upgrades
        .iter()
        .map(|upgrade| {
            let style = if Some(upgrade.tower_index) == state.selected_index {
                Style::new().bold()
            } else {
                Style::new()
            };

            Row::new([
                // Text::raw(format!("{}", upgrade.tower_index)),
                Text::raw(format!("${}", upgrade.cost)).alignment(Alignment::Right),
                Text::raw(format!("{}", upgrade.name)),
            ])
            .style(style)
        })
        .collect::<Vec<_>>();

    let (before, after) = rows.split_at(index);

    let cost_row = Row::new([
        // Text::raw(""),
        Text::raw(format!("${}", state.cash)).alignment(Alignment::Right),
        Text::raw(""),
    ]);

    let rows = before
        .iter()
        .cloned()
        .chain(iter::once(cost_row))
        .chain(after.iter().cloned())
        .collect::<Vec<_>>();

    let columns = [
        // Constraint::Min(2),
        Constraint::Min(8),
        Constraint::Percentage(100),
    ];

    let table = Table::new(rows, columns);

    Widget::render(table, area, buf);
}

fn render_danger(area: Rect, buf: &mut Buffer, state: &InGameSummary) {
    match state.danger {
        Some(danger) => {
            let width = area.width - 1;

            let right = (danger / state.max_path * width as f32) as usize;
            let left = width as usize - right;

            let text = "-".repeat(left) + "O" + &" ".repeat(right);

            Line::from(text).render(area, buf);
        }
        None => {}
    }
}

impl Widget for &InGameSummary {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = Rect::new(area.x, area.y, 80, area.height);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Percentage(100),
                Constraint::Length(2),
            ])
            .split(area);

        Line::from(format!("{} {}", self.map_name, self.mode)).render(layout[0], buf);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[1]);

        let total = self
            .towers
            .iter()
            .map(|t| match t {
                Tower::Basic(t) => t.worth,
                Tower::Hero(t) => t.worth,
            })
            .sum::<u64>();

        let danger_track =
            Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT);

        render_danger(danger_track.inner(layout[2]), buf, &self);
        danger_track.render(layout[2], buf);

        let towers_table = Block::default()
            .title(format!(" Towers ${total} "))
            .border_set(symbols::border::Set {
                bottom_left: symbols::line::NORMAL.vertical_right,
                ..symbols::border::PLAIN
            })
            .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM);

        render_towers_table(towers_table.inner(top[0]), buf, &self);
        towers_table.render(top[0], buf);

        let upgrades_table = Block::default()
            .title(" Upgrades ")
            .border_set(symbols::border::Set {
                top_left: symbols::line::NORMAL.horizontal_down,
                bottom_left: symbols::line::NORMAL.horizontal_up,
                bottom_right: symbols::line::NORMAL.vertical_left,
                ..symbols::border::PLAIN
            })
            .borders(Borders::ALL);

        render_upgrades_table(upgrades_table.inner(top[1]), buf, &self);
        upgrades_table.render(top[1], buf);
    }
}
