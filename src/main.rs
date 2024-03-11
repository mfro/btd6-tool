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

fn main() -> Result<()> {
    let mut app = App::new();
    app.run()?;

    // let game = BloonsGame::find_game()?;
    // let ingame = game.get_ingame()?.expect("ingame");

    Ok(())
}
