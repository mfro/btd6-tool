use byteorder::{ByteOrder, NativeEndian};
use windows::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::ReadProcessMemory};

use crate::Result;

macro_rules! pointer_type {
    ($ty:ident) => {
        pub struct $ty(pub Pointer);

        impl MemoryRead for $ty {
            fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
                view.read(address).map($ty)
            }
        }
    };
}

#[derive(Clone, Copy, Debug)]
pub struct ProcessMemoryView {
    handle: HANDLE,
}

impl ProcessMemoryView {
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

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

pub trait MemoryRead: Sized {
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

pub struct Pointer {
    pub memory: ProcessMemoryView,
    pub address: u64,
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
