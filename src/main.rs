use std::{str::FromStr, thread::sleep, time::Duration};

use windows::Win32::System::{
    Diagnostics::Debug::Beep,
    Threading::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
};

mod btd;
mod memory;
mod process;

use btd::types::InGame;
use memory::ProcessMemoryView;
use process::Process;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy)]
enum Condition {
    Cash(u64),
    Round(u64),
}

impl FromStr for Condition {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Condition> {
        let target_value = s.parse()?;

        if target_value > 100 {
            Ok(Self::Cash(target_value))
        } else {
            Ok(Self::Round(target_value))
        }
    }
}

fn main() -> Result<()> {
    let mut conditions = std::env::args()
        .skip(1)
        .map(|s| s.parse())
        .collect::<Result<Vec<_>>>()?;

    conditions.reverse();

    let pid = btd::find_pid()?;

    let process = Process::from_pid(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;

    let memory_view = ProcessMemoryView::new(process);
    let module = btd::find_game_module(&process)?;

    while let Some(condition) = conditions.last() {
        let ingame = InGame::get_instance(&memory_view, module.get_bounds()?.0);

        if let Some(ingame) = ingame {
            let simulation = ingame.unity_to_simulation().simulation();

            simulation.tower_manager().tower_history();
            simulation.tower_manager().towers();

            let health = simulation.health().get() as u64;
            let round = simulation.map().spawner().round().get() as u64 + 1;
            let cash = simulation.cash_manager().cash().get() as u64;

            println!("health: {health} cash: {cash} round: {round} / {condition:?}");

            let stop = match condition {
                Condition::Cash(v) => cash >= *v,
                Condition::Round(v) => round >= *v,
            };

            if stop {
                unsafe { Beep(500, 200) }?;
                sleep(Duration::from_millis(100));
                unsafe { Beep(500, 200) }?;

                conditions.pop();
            }
        } else {
            println!("not in game");
        }

        sleep(Duration::from_millis(100));
    }

    Ok(())
}
