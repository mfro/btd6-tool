use std::time::Duration;

use btd::BloonsGame;
use windows::{
    core::{s, PCSTR},
    Win32::{
        Foundation::{LPARAM, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{VK_ESCAPE, VK_OEM_3},
            WindowsAndMessaging::{
                FindWindowA, PostMessageA, SendMessageA, WM_CHAR, WM_KEYDOWN, WM_KEYUP,
            },
        },
    },
};

mod app;
mod btd;
mod memory;
mod process;
mod win32_util;

use app::App;
use process::Process;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone)]
struct Previous<T> {
    value: Option<T>,
}

impl<T> Default for Previous<T> {
    fn default() -> Self {
        Self { value: None }
    }
}

impl<T: PartialEq> Previous<T> {
    pub fn set(&mut self, value: T) -> bool {
        let is_update = match &self.value {
            Some(v) => *v != value,
            None => true,
        };

        self.value = Some(value);
        is_update
    }
}

enum GameDifficulty {
    Easy,
    Medium,
    Hard,
    Impoppable,
}

enum GameType {
    Standard,
    Unknown(String),
}

enum GameMode {
    Chimps,
}

fn main() -> Result<()> {
    let mut app = App::new();
    app.run()?;

    Ok(())
}
