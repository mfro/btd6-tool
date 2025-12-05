use std::mem::{size_of, size_of_val};

use windows::Win32::{
    Foundation::{HANDLE, HMODULE},
    System::{
        Diagnostics::Debug::ReadProcessMemory,
        ProcessStatus::{
            EnumProcessModules, EnumProcesses, GetModuleBaseNameA, GetModuleInformation,
            GetProcessImageFileNameA, MODULEINFO,
        },
        Threading::{OpenProcess, PROCESS_ACCESS_RIGHTS},
    },
};

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Process {
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

    pub fn read_memory(&self, address: u64, buffer: &mut [u8]) -> Result<usize> {
        let mut count = 0;

        unsafe {
            ReadProcessMemory(
                self.handle,
                address as _,
                buffer.as_mut_ptr() as _,
                buffer.len(),
                Some(&mut count),
            )?;
        }

        Ok(count)
    }

    pub fn get_modules(&'_ self) -> Result<Vec<Module<'_>>> {
        let mut modules = [HMODULE::default(); 1024];
        let mut output = 0;

        unsafe {
            EnumProcessModules(
                self.handle,
                modules.as_mut_ptr() as _,
                size_of_val(&modules) as u32,
                &mut output,
            )?;
        }

        let modules = modules[..output as usize / size_of::<HMODULE>()]
            .into_iter()
            .map(|&handle| Module {
                process: self,
                handle,
            })
            .collect::<Vec<_>>();

        Ok(modules)
    }
}

pub struct Module<'a> {
    process: &'a Process,
    handle: HMODULE,
}

impl<'a> Module<'a> {
    pub fn get_base_name(&self) -> Result<String> {
        let mut file_name = [0u8; 1024];
        let len = unsafe {
            GetModuleBaseNameA(self.process.handle, self.handle, &mut file_name) as usize
        };

        let string = String::from_utf8(file_name[0..len].to_vec())
            .expect("GetModuleBaseNameA invalid string");

        Ok(string)
    }

    pub fn get_bounds(&self) -> Result<(u64, u64)> {
        let mut info = MODULEINFO::default();

        unsafe {
            GetModuleInformation(
                self.process.handle,
                self.handle,
                &mut info,
                size_of_val(&info) as u32,
            )?;
        }

        Ok((info.lpBaseOfDll as u64, info.SizeOfImage as u64))
    }
}
