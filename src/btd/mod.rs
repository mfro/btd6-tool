use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;

use crate::{
    process::{Module, Process},
    Result,
};

pub mod types;

pub fn find_pid() -> Result<u32> {
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

pub fn find_game_module(process: &Process) -> Result<Module> {
    for module in process.get_modules()? {
        let module_name = module.get_base_name()?;

        if module_name == "GameAssembly.dll" {
            return Ok(module);
        }
    }

    Err("module not found".into())
}
