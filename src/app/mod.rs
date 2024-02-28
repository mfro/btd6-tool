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

use crate::{
    btd::{
        summary::{GameState, InGameState},
        BloonsGame,
    },
    Previous, Result,
};

mod tui;

enum AppEvent {
    Render(GameState),
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

struct GameThread {
    out: SyncSender<AppEvent>,
    bloons: BloonsGame,
}

impl GameThread {
    fn new(out: SyncSender<AppEvent>, bloons: BloonsGame) -> Self {
        Self { out, bloons }
    }

    fn run(&mut self) -> Result<()> {
        let mut previous = Previous::default();

        loop {
            let state = self.bloons.get_state();

            if let (GameState::InGame(a), Some(GameState::InGame(b))) = (&state, &previous.value) {
                let do_beep = a
                    .upgrades
                    .iter()
                    .filter(|up| match a.selected_index {
                        Some(i) => i == up.tower_index,
                        None => true,
                    })
                    .any(|upgrade| (b.cash..a.cash).contains(&upgrade.cost));

                if do_beep {
                    crate::beep();
                }
            }

            if previous.set(state.clone()) {
                self.out.send(AppEvent::Render(state))?;
            }

            thread::sleep(Duration::from_millis(25));
        }
    }
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

        let mut game_thread = GameThread::new(send.clone(), game);
        let input_thread = InputThread::new(send.clone());

        thread::spawn(move || game_thread.run().unwrap());
        thread::spawn(move || input_thread.run().unwrap());

        while let Ok(event) = recv.recv() {
            match event {
                AppEvent::Render(state) => self.render(&mut terminal, state)?,

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
            let style = if Some(i) == state.selected_index {
                Style::new().bold()
            } else {
                Style::new()
            };

            Row::new([
                Text::raw(format!("${}", tower.worth)).alignment(Alignment::Right),
                Text::raw(format!(
                    "{}-{}-{}",
                    tower.tiers[0], tower.tiers[1], tower.tiers[2]
                )),
                Text::raw(format!("{}", tower.name)),
            ])
            .style(style)
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

impl Widget for &InGameState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = Rect::new(area.x, area.y, 80, area.height);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let total = self.towers.iter().map(|t| t.worth).sum::<u64>();

        let towers_table = Block::default()
            .title(format!(" Towers ${total} "))
            .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM);

        render_towers_table(towers_table.inner(layout[0]), buf, &self);
        towers_table.render(layout[0], buf);

        let upgrades_table = Block::default()
            .title(" Upgrades ")
            .border_set(symbols::border::Set {
                top_left: symbols::line::NORMAL.horizontal_down,
                bottom_left: symbols::line::NORMAL.horizontal_up,
                ..symbols::border::PLAIN
            })
            .borders(Borders::ALL);

        render_upgrades_table(upgrades_table.inner(layout[1]), buf, &self);
        upgrades_table.render(layout[1], buf);
    }
}
