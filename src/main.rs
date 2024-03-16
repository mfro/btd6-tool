mod app;
mod btd;
mod memory;
mod process;
mod win32_util;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    time::{Duration, Instant},
};

use app::App;
use btd::{
    types::{ObjectId, Simulation},
    BloonModelCache, BloonsGame,
};
use process::Process;
use serde::{Deserialize, Serialize};
use windows::Win32::System::{
    Memory::{
        MemoryRegionInfo, QueryVirtualMemoryInformation, MEMORY_BASIC_INFORMATION,
        WIN32_MEMORY_REGION_INFORMATION,
    },
    Threading::PROCESS_QUERY_INFORMATION,
};

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

pub trait TryMap<T1, E> {
    fn and_then_map<T2>(
        self,
        f: impl FnMut(T1) -> std::result::Result<T2, E>,
    ) -> impl Iterator<Item = std::result::Result<T2, E>>;
}

impl<T1, E, I> TryMap<T1, E> for I
where
    I: Iterator<Item = std::result::Result<T1, E>>,
{
    fn and_then_map<T2>(
        self,
        mut f: impl FnMut(T1) -> std::result::Result<T2, E>,
    ) -> impl Iterator<Item = std::result::Result<T2, E>> {
        self.map(move |v| v.and_then(|v| f(v)))
    }
}

fn main() -> Result<()> {
    if std::env::args().nth(1).is_some_and(|v| v == "test") {
        // unsafe {
        //     let pid = btd::find_pid()?;
        //     let process = Process::from_pid(pid, PROCESS_QUERY_INFORMATION)?;

        //     let mut info = MEMORY_BASIC_INFORMATION::default();

        //     // 1865361014784
        //     // 1865492201472

        //     windows::Win32::System::Memory::VirtualQueryEx(
        //         process.handle,
        //         Some(1865361014784usize as _),
        //         &mut info as *mut _ as _,
        //         std::mem::size_of_val(&info),
        //     );

        //     println!("{:#?}", info);
        // }

        let mut game = BloonsGame::find_game()?;

        let t0 = Instant::now();
        std::hint::black_box(game.try_get_state()?);
        println!("{:?}", t0.elapsed());

        // let Some(ingame) = game.get_ingame()? else {
        //     return Err("not ingame".into());
        // };

        // let simulation = ingame.unity_to_simulation()?.simulation()?;

        // println!("{}", simulation.model()?.random_seed()?);

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
