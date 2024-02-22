#![allow(dead_code, unused_imports)]

use std::{
    error::Error,
    fs::File,
    io::Write,
    mem::{size_of, size_of_val},
    ptr::null,
    thread::sleep,
    time::Duration,
};

use byteorder::{ByteOrder, LittleEndian, NativeEndian};

use windows::{
    core::{s, w, PCSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, HMODULE, LPARAM, WPARAM},
        System::{
            Diagnostics::Debug::{Beep, ReadProcessMemory},
            LibraryLoader::GetProcAddress,
            Memory::{
                VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_READWRITE,
            },
            ProcessStatus::{
                EnumProcessModules, EnumProcesses, GetModuleBaseNameA, GetModuleFileNameExA,
                GetModuleInformation, GetProcessImageFileNameA, MODULEINFO,
            },
            Threading::{
                OpenProcess, PROCESS_ACCESS_RIGHTS, PROCESS_QUERY_INFORMATION,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
            },
        },
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
                KEYEVENTF_KEYUP, VK_ESCAPE,
            },
            WindowsAndMessaging::{FindWindowA, SendMessageA, WM_KEYDOWN, WM_KEYUP},
        },
    },
};

mod ocr;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const TYPE_OFFSET_IN_GAME: u64 = 0x32d9b98;

macro_rules! pointer_type {
    ($ty:ident) => {
        struct $ty(Pointer);

        impl MemoryRead for $ty {
            fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
                view.read(address).map($ty)
            }
        }
    };
}
const TYPE_OFFSET_UNITY_TO_SIMULATION: u64 = 0x32e0ea0;
const TYPE_OFFSET_SIMULATION: u64 = 0x333be00;

trait MemoryRead: Sized {
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self>;
}

impl MemoryRead for f64 {
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 8];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_f64(&buffer))
    }
}

impl MemoryRead for u64 {
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 8];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_u64(&buffer))
    }
}

impl MemoryRead for u32 {
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 4];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_u32(&buffer))
    }
}

impl MemoryRead for String {
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let address = view.read(address)?;

        let mut buffer = vec![0; 1024];
        view.read_exact(address, &mut buffer)?;

        let len = buffer.iter().position(|&b| b == 0).unwrap();
        let value = String::from_utf8(buffer[0..len].to_vec())?;

        Ok(value)
    }
}

struct Pointer {
    memory: ProcessMemoryView,
    address: u64,
}

impl Pointer {
    pub fn read<T: MemoryRead>(&self, offset: u64) -> Result<T> {
        self.memory.read(self.address + offset)
    }
}

impl MemoryRead for Pointer {
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let address = view.read(address)?;

        Ok(Self {
            memory: view.clone(),
            address,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct ProcessMemoryView {
    handle: HANDLE,
}

impl ProcessMemoryView {
    pub fn read<T: MemoryRead>(&self, address: u64) -> Result<T> {
        T::read(self, address)
    }

    pub fn read_bytes(&self, address: u64, out: &mut [u8]) -> Result<usize> {
        let mut count = 0;

        unsafe {
            ReadProcessMemory(
                self.handle,
                address as _,
                out.as_mut_ptr() as _,
                out.len(),
                Some(&mut count),
            )?;
        }

        Ok(count)
    }

    pub fn read_exact(&self, address: u64, out: &mut [u8]) -> Result<()> {
        let mut index = 0;

        while index < out.len() {
            index += self.read_bytes(address + index as u64, &mut out[index..])?;
        }

        Ok(())
    }
}

pointer_type!(TypeInfo);
impl TypeInfo {
    pub fn get_name(&self) -> String {
        self.0.read(0x10).unwrap()
    }

    pub fn get_statics(&self) -> TypeStatics {
        self.0.read(0xb8).unwrap()
    }
}

pointer_type!(TypeStatics);
impl TypeStatics {
    pub fn field<T: MemoryRead>(&self, offset: u64) -> Result<T> {
        self.0.read(offset)
    }
}

pointer_type!(Object);
impl Object {
    pub fn get_type(&self) -> TypeInfo {
        self.0.read(0x0).unwrap()
    }

    pub fn field<T: MemoryRead>(&self, offset: u64) -> Result<T> {
        self.0.read(0x10 + offset)
    }
}

fn enum_processes() -> Vec<u32> {
    let mut processes = [0u32; 4096];
    let mut needed = 0u32;

    unsafe {
        EnumProcesses(
            processes.as_mut_ptr(),
            size_of_val(&processes) as u32,
            &mut needed as _,
        )
        .unwrap();

        processes[0..needed as usize / size_of::<u32>()].to_vec()
    }
}

struct Process {
    handle: HANDLE,
}

impl Process {
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

fn find_process() -> Result<Option<u32>> {
    for pid in enum_processes() {
        if let Ok(process) = Process::from_pid(pid, PROCESS_QUERY_LIMITED_INFORMATION) {
            let file_name = process.get_image_file_name()?;

            if file_name.ends_with("BloonsTD6.exe") {
                return Ok(Some(pid));
            }
        }
    }

    Ok(None)
}

fn find_game_module(process: HANDLE) -> Result<u64> {
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

        let string = String::from_utf8(file_name[0..len].to_vec()).unwrap();

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

fn main() -> Result<()> {
    let pid = find_process()?.unwrap();

    unsafe {
        let access = PROCESS_QUERY_INFORMATION | PROCESS_VM_READ;

        let process = OpenProcess(access, false, pid).unwrap();
        let module_base = find_game_module(process)?;

        let view = ProcessMemoryView { handle: process };
        let ingame_type: TypeInfo = view.read(module_base + TYPE_OFFSET_IN_GAME)?;

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

                let input = std::env::args().nth(1).unwrap();

                let stop = if input.starts_with("$") {
                    let target = input[1..].parse()?;
                    cash >= target
                } else {
                    let target = input.parse()?;
                    round >= target
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
