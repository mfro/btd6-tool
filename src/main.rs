mod app;
mod btd;
mod memory;
mod process;
mod win32_util;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    time::Duration,
};

use app::App;
use btd::{
    types::{ObjectId, Simulation},
    BloonModelCache, BloonsGame,
};
use process::Process;
use serde::{Deserialize, Serialize};

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
    if std::env::args().nth(1).is_some_and(|v| v == "test") {
        let game = BloonsGame::find_game()?;

        let Some(ingame) = game.get_ingame()? else {
            return Err("not ingame".into());
        };

        let simulation = ingame.unity_to_simulation()?.simulation()?;

        println!("{}", simulation.model()?.random_seed()?);

        // println!(
        //     "{} {} {}",
        //     simulation.map()?.spawner()?.current_round()?.get()?,
        //     simulation.time()?.elapsed()?,
        //     simulation.round_time()?.elapsed()?
        // );

        // let model = simulation.model()?;
        // let cache = BloonModelCache::load(&model)?;

        // for (i, round) in model.round_set()?.rounds()?.iter()?.enumerate() {
        //     let round = round?;

        //     let mut worth = 0.0;

        //     for group in round.groups()?.iter()? {
        //         let group = group?;

        //         let bloon = cache.get(group.bloon()?.to_string()).unwrap();

        //         worth += group.count()? as f32 * bloon.worth(i as u64 + 1);
        //     }

        //     println!("{: >3}: {: >6.1}", i + 1, worth);
        // }
    } else {
        let mut app = App::new();
        app.run()?;
    }

    Ok(())
}
