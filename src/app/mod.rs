use std::{
    fs::File,
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
        Input::KeyboardAndMouse::{SetActiveWindow, SetCapture, SetFocus},
        WindowsAndMessaging::{FindWindowA, GetForegroundWindow, SetForegroundWindow},
    },
};

use crate::{
    btd::{
        interface::{GameState, InGameState},
        log::{GameLog, GameLogState},
        types::TowerSet,
        BloonsGame, BloonsHistogram,
    },
    win32_util, Previous, Result,
};

mod tui;

enum AppEvent {
    State(GameState),
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
            let state = self.game.get_state();

            if let (GameState::InGame(a), Some(GameState::InGame(b))) = (&state, &previous.value) {
                let do_beep = a
                    .towers
                    .iter()
                    .enumerate()
                    .filter(|(i, tower)| match a.selected_tower {
                        Some(v) => v == *i,
                        None => true,
                    })
                    .any(|(_, tower)| {
                        a.model.towers.towers[tower.kind]
                            .available_upgrades
                            .iter()
                            .any(|i| (b.cash..a.cash).contains(&a.model.upgrades.upgrades[*i].cost))
                    });

                if do_beep {
                    win32_util::beep();
                }

                if should_pause(a) && !should_pause(b) {
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
                self.out.send(AppEvent::State(state))?;
            }

            thread::sleep(Duration::from_millis(25));
        }
    }
}

struct GameLogThread {
    out: SyncSender<AppEvent>,
    game: BloonsGame,
}

impl GameLogThread {
    fn new(out: SyncSender<AppEvent>, game: BloonsGame) -> Self {
        Self { out, game }
    }

    fn run(&mut self) -> Result<()> {
        let mut state = GameLogState::default();
        let mut log = GameLog::default();

        loop {
            std::thread::sleep(Duration::from_millis(100));

            if let Ok(new_state) = self.get_state() {
                if new_state.towers.is_empty() {
                    log = GameLog::default();
                }

                log.update(&state, &new_state);
                state = new_state;

                let out = File::create(format!("log/{}.json", state.label))?;
                serde_json::to_writer_pretty(out, &log)?;
            }
        }
    }

    fn get_state(&self) -> Result<GameLogState> {
        let Some(ingame) = self.game.get_ingame()? else {
            return Err("not ingame".into());
        };

        let simulation = ingame.unity_to_simulation()?.simulation()?;

        Ok(GameLogState::load(&simulation)?)
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

fn should_pause(summary: &InGameState) -> bool {
    summary
        .paths
        .iter()
        .any(|p| p.bloons.iter().any(|b| b.leak_distance < 50.0))
        && summary.model.game_mode != "Clicks"
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
        let mut log_thread = GameLogThread::new(send.clone(), game.clone());

        let input_thread = InputThread::new(send.clone());

        thread::spawn(move || log_thread.run().unwrap());
        thread::spawn(move || game_thread.run().unwrap());
        thread::spawn(move || input_thread.run().unwrap());

        while let Ok(event) = recv.recv() {
            match event {
                AppEvent::State(state) => self.render(&mut terminal, state)?,

                AppEvent::Exit => break,
            }
        }

        tui::restore()?;

        Ok(())
    }

    fn render(&self, terminal: &mut tui::Tui, summary: GameState) -> Result<()> {
        terminal.draw(|frame| frame.render_widget(&summary, frame.size()))?;

        Ok(())
    }
}

impl Widget for &GameState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            GameState::None => {
                let title = Title::from(" Not in game ".bold());

                Block::default()
                    .title(title.alignment(Alignment::Center))
                    .borders(Borders::ALL)
                    .border_set(border::THICK)
                    .render(area, buf);
            }

            GameState::InGame(state) => {
                state.render(area, buf);
            }
        }
    }
}

fn render_towers_table(area: Rect, buf: &mut Buffer, state: &InGameState) {
    let rows: Vec<Row<'_>> = state
        .towers
        .iter()
        .enumerate()
        .map(|(i, tower)| {
            let style = if Some(i) == state.selected_tower {
                Style::new().bold()
            } else {
                Style::new()
            };

            let kind = &state.model.towers.towers[tower.kind];

            let row = match kind.set {
                TowerSet::HERO => Row::new([
                    Text::raw(format!("${}", tower.worth)).alignment(Alignment::Right),
                    Text::raw(format!("{}", kind.tiers[0])).alignment(Alignment::Right),
                    Text::raw(format!("{}", kind.id)),
                ]),

                _ => Row::new([
                    Text::raw(format!("${}", tower.worth)).alignment(Alignment::Right),
                    Text::raw(format!(
                        "{}-{}-{}",
                        kind.tiers[0], kind.tiers[1], kind.tiers[2]
                    )),
                    Text::raw(format!("{}", kind.id)),
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

fn render_upgrades_table(area: Rect, buf: &mut Buffer, state: &InGameState) {
    let upgrades = state
        .towers
        .iter()
        .enumerate()
        .flat_map(|(i, tower)| {
            state.model.towers.towers[tower.kind]
                .available_upgrades
                .iter()
                .map(move |i2| (i, &state.model.upgrades.upgrades[*i2]))
        })
        .collect::<Vec<_>>();

    let index = upgrades
        .iter()
        .position(|s| s.1.cost > state.cash)
        .unwrap_or(upgrades.len());

    let rows: Vec<Row<'_>> = upgrades
        .iter()
        .map(|upgrade| {
            let style = if Some(upgrade.0) == state.selected_tower {
                Style::new().bold()
            } else {
                Style::new()
            };

            Row::new([
                // Text::raw(format!("{}", upgrade.tower_index)),
                Text::raw(format!("${}", upgrade.1.cost)).alignment(Alignment::Right),
                Text::raw(format!("{}", upgrade.1.id)),
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

fn render_danger(area: Rect, buf: &mut Buffer, state: &InGameState) {
    let danger = state
        .paths
        .iter()
        .filter_map(|p| {
            p.bloons
                .iter()
                .map(|b| b.leak_distance)
                .max_by(f32::total_cmp)
        })
        .max_by(f32::total_cmp);

    let max_path = state
        .paths
        .iter()
        .map(|p| p.leak_distance)
        .max_by(f32::total_cmp);

    match (danger, max_path) {
        (Some(danger), Some(max_path)) => {
            let width = area.width - 1;

            let right = (danger / max_path * width as f32) as usize;
            let left = width as usize - right;

            let text = "-".repeat(left) + "O" + &" ".repeat(right);

            Line::from(text).render(area, buf);
        }
        _ => {}
    }
}

impl Widget for &InGameState {
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

        Line::from(format!(
            "{} {} {}",
            self.model.map_name, self.model.game_mode, self.model.seed
        ))
        .render(layout[0], buf);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[1]);

        let total = self.towers.iter().map(|t| t.worth).sum::<f32>() as u64;

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
