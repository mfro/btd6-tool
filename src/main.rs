use std::{
    error::Error,
    mem::{size_of, size_of_val},
    thread::sleep,
    time::Duration,
};

use memory::{Object, ProcessMemoryView, TypeInfo};
use windows::Win32::{
    Foundation::{HANDLE, HMODULE},
    System::{
        Diagnostics::Debug::Beep,
        ProcessStatus::{
            EnumProcessModules, EnumProcesses, GetModuleBaseNameA, GetModuleInformation,
            GetProcessImageFileNameA, MODULEINFO,
        },
        Threading::{
            OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_INFORMATION,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
        },
    },
};

mod memory;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

const TYPE_OFFSET_IN_GAME: u64 = 0x32d9b98;

struct Process {
    handle: HANDLE,
}

impl Process {
    pub fn enum_process_ids() -> Vec<u32> {
        let mut processes = [0u32; 4096];
        let mut needed = 0u32;

        unsafe {
            EnumProcesses(
                processes.as_mut_ptr(),
                size_of_val(&processes) as u32,
                &mut needed as _,
            )
            .expect("EnumProcesses");

            processes[0..needed as usize / size_of::<u32>()].to_vec()
        }
    }

    pub fn from_pid(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> Result<Process> {
        let handle = unsafe { OpenProcess(access, false, pid) }?;

        Ok(Process { handle })
    }

    pub fn get_image_file_name(&self) -> Result<String> {
        let mut name = [0u8; 1024];
        let len = unsafe { GetProcessImageFileNameA(self.handle, &mut name) } as usize;

        Ok(String::from_utf8(name[0..len].to_vec())?)
    }
}

fn find_bloons_pid() -> Result<u32> {
    for pid in Process::enum_process_ids() {
        if let Ok(process) = Process::from_pid(pid, PROCESS_QUERY_LIMITED_INFORMATION) {
            let file_name = process.get_image_file_name()?;

            if file_name.ends_with("BloonsTD6.exe") {
                return Ok(pid);
            }
        }
    }

    Err("bloons process not found".into())
}

fn find_bloons_game_module(process: HANDLE) -> Result<u64> {
    let mut modules = [HMODULE::default(); 1024];
    let mut output = 0;

    unsafe {
        EnumProcessModules(
            process,
            modules.as_mut_ptr() as _,
            size_of_val(&modules) as u32,
            &mut output,
        )?;
    }

    let modules = &modules[..output as usize / size_of::<HMODULE>()];

    for module in modules.iter() {
        let mut file_name = [0u8; 1024];
        let len = unsafe { GetModuleBaseNameA(process, *module, &mut file_name) } as usize;

        let string = String::from_utf8(file_name[0..len].to_vec())
            .expect("GetModuleBaseNameA invalid string");

        if string == "GameAssembly.dll" {
            let mut info = MODULEINFO::default();

            unsafe {
                GetModuleInformation(process, *module, &mut info, size_of_val(&info) as u32)?;
            }

            return Ok(info.lpBaseOfDll as u64);
        }
    }

    Err("module not found".into())
}

enum OperationMode {
    Cash(u64),
    Round(u64),
}

fn main() -> Result<()> {
    let input = std::env::args()
        .nth(1)
        .expect("Specify cash or round value");

    let mode = if input.starts_with("$") {
        let target = input[1..].parse()?;
        println!("waiting for cash: ${}", target);
        OperationMode::Cash(target)
    } else {
        let target = input.parse()?;
        println!("waiting for round: {}", target);
        OperationMode::Round(target)
    };

    let pid = find_bloons_pid()?;

    unsafe {
        let access = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ;
        let process = OpenProcess(access, false, pid).expect("OpenProcess");

        let module_base = find_bloons_game_module(process)?;

        let memory_view = ProcessMemoryView::new(process);

        let ingame_type: TypeInfo = memory_view.read(module_base + TYPE_OFFSET_IN_GAME)?;

        // Assets_Scripts_Unity_UI_New_InGame_InGame_o
        let ingame: Object = ingame_type.get_statics().field(0x0)?;
        if ingame.0.address == 0 {
            println!("not in game");
        } else {
            assert_eq!("InGame", ingame.get_type().get_name());

            // Assets_Scripts_Unity_Bridge_UnityToSimulation_o
            let bridge: Object = ingame.field(0xb8)?;
            assert_eq!("UnityToSimulation", bridge.get_type().get_name());

            // Assets_Scripts_Simulation_Simulation_o
            let simulation: Object = bridge.field(0x18)?;
            assert_eq!("Simulation", simulation.get_type().get_name());

            // Assets_Scripts_Simulation_Simulation_o
            let map: Object = simulation.field(0x398)?;
            assert_eq!("Map", map.get_type().get_name());

            // Assets_Scripts_Simulation_Track_Map_o
            let spawner: Object = map.field(0x88)?;
            assert_eq!("Spawner", spawner.get_type().get_name());

            // Assets_Scripts_Utils_KonFuze_NoShuffle_o
            let rounds: Object = spawner.field(0xd8)?;
            assert_eq!("KonFuze_NoShuffle", rounds.get_type().get_name());

            // System_Collections_Generic_Dictionary_TKey__TValue__o
            let cash_managers: Object = simulation.field(0x378)?;
            assert_eq!("Dictionary`2", cash_managers.get_type().get_name());

            // System_Collections_Generic_Dictionary_Entry_TKey__TValue__array
            let cash_manager_entries: Object = cash_managers.field(0x8)?;
            assert_eq!("Entry[]", cash_manager_entries.get_type().get_name());

            // Assets_Scripts_Simulation_Simulation_CashManager_o
            let cash_manager: Object = cash_manager_entries.field(0x20)?;
            assert_eq!("CashManager", cash_manager.get_type().get_name());

            // Assets_Scripts_Utils_KonFuze_NoShuffle_o
            let cash: Object = cash_manager.field(0x0)?;
            assert_eq!("KonFuze", cash.get_type().get_name());

            loop {
                let round: f64 = rounds.field(0x18)?;
                let round = round as u64 + 1;

                let cash: f64 = cash.field(0x18)?;
                let cash = cash as u64 + 1;

                println!("cash: {} round: {}", cash, round);

                let stop = match mode {
                    OperationMode::Cash(target) => cash >= target,
                    OperationMode::Round(target) => round >= target,
                };

                if !stop {
                    sleep(Duration::from_millis(1000));
                } else {
                    Beep(500, 200)?;
                    sleep(Duration::from_millis(100));
                    Beep(500, 200)?;

                    break;
                }
            }
        }

        Ok(())
    }
}
