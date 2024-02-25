use std::{thread::sleep, time::Duration};

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

fn main() -> Result<()> {
    let input = std::env::args()
        .nth(1)
        .expect("Specify cash or round value");

    let target_value = input.parse()?;
    if target_value > 100 {
        println!("waiting for cash: ${}", target_value);
    } else {
        println!("waiting for round: {}", target_value);
    }

    let pid = btd::find_pid()?;

    let process = Process::from_pid(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;

    let memory_view = ProcessMemoryView::new(process);
    let module = btd::find_game_module(&process)?;

    loop {
        let ingame = InGame::get_instance(&memory_view, module.get_bounds()?.0);

        if let Some(ingame) = ingame {
            let simulation = ingame.unity_to_simulation().simulation();

            let health = simulation.health().get() as u64;
            let round = simulation.map().spawner().round().get() as u64 + 1;
            let cash = simulation.cash_manager().cash().get() as u64;

            println!("health: {health} cash: {} round: {}", cash, round);

            let stop = if target_value > 100 {
                cash >= target_value
            } else {
                round >= target_value
            };

            if stop {
                unsafe { Beep(500, 200) }?;
                sleep(Duration::from_millis(100));
                unsafe { Beep(500, 200) }?;

                break;
            }
        } else {
            println!("not in game");
        }

        sleep(Duration::from_millis(100));
    }

    Ok(())
}
