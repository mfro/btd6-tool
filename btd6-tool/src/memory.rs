use byteorder::{ByteOrder, NativeEndian};

use crate::{Process, Result};

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + crate::memory::count!($($xs)*));
}

macro_rules! pointer_type {
    ($ty:ident) => {
        #[derive(Debug, Clone)]
        pub struct $ty(pub Pointer);

        impl crate::memory::MemoryRead for $ty {
            const SIZE: usize = 8;

            fn read(view: &ProcessMemoryView, address: u64) -> crate::Result<Self> {
                match view.read::<Option<$ty>>(address)? {
                    Some(v) => Ok(v),
                    None => anyhow::bail!(format!("expected {}", stringify!($ty))),
                }
            }
        }

        impl crate::memory::MemoryRead for Option<$ty> {
            const SIZE: usize = 8;

            fn read(view: &ProcessMemoryView, address: u64) -> crate::Result<Self> {
                let value: Pointer = view.read(address)?;
                if value.address == 0 {
                    Ok(None)
                } else {
                    Ok(Some($ty(value)))
                }
            }
        }
    };
}

macro_rules! object_type {
    ($ty:ident) => {
        object_type!($ty<> ; stringify!($ty));
    };

    ($ty:ident ; $name:expr) => {
        object_type!($ty<> ; $name);
    };

    ($ty:ident<$( $generic:ident ),*>) => {
        object_type!($ty<$( $generic ),*> ; stringify!($ty));
    };

    ($ty:ident<$( $generic:ident ),*> ; $name:expr) => {
        #[derive(Debug)]
        #[allow(unused_parens)]
        pub struct $ty<$( $generic: MemoryRead ),*>(pub crate::memory::Pointer, std::marker::PhantomData<($( $generic ),*)>);

        impl<$( $generic: MemoryRead ),*> Clone for $ty<$( $generic ),*> {
            fn clone(&self) -> Self {
                Self(self.0.clone(), std::default::Default::default())
            }
        }

        impl<$( $generic: MemoryRead ),*> crate::memory::MemoryRead for $ty<$( $generic ),*> {
            const SIZE: usize = crate::memory::Pointer::SIZE;

            fn read(view: &ProcessMemoryView, address: u64) -> crate::Result<Self> {
                view.read::<Pointer>(address).and_then($ty::try_from)
            }
        }

        impl<$( $generic: MemoryRead ),*> crate::memory::MemoryRead for Option<$ty<$( $generic ),*>> {
            const SIZE: usize = crate::memory::Pointer::SIZE;

            fn read(view: &ProcessMemoryView, address: u64) -> crate::Result<Self> {
                let pointer: crate::memory::Pointer = view.read(address)?;
                (pointer.address != 0).then(|| $ty::try_from(pointer)).transpose()
            }
        }

        impl<$( $generic: MemoryRead ),*> From<$ty<$( $generic ),*>> for Object {
            fn from(value: $ty<$( $generic ),*>) -> Object {
                Object(value.0)
            }
        }

        impl<$( $generic: MemoryRead ),*> From<$ty<$( $generic ),*>> for Pointer {
            fn from(value: $ty<$( $generic ),*>) -> Pointer {
                value.0
            }
        }

        impl<$( $generic: MemoryRead ),*> TryFrom<Pointer> for $ty<$( $generic ),*> {
            type Error = anyhow::Error;

            fn try_from(value: Pointer) -> crate::Result<Self> {
                let expected_type_name = match crate::memory::count!( $( $generic )* ) {
                    0 => $name.to_string(),
                    _ => format!("{}`{}", $name, crate::memory::count!( $( $generic )* )),
                };

                if value.address == 0 {
                    anyhow::bail!("Expected {} got null", expected_type_name);
                }

                let value = Self(value, std::default::Default::default());

                let correct =
                if $name == "Array" {
                    // todo gross array type checking
                    value.get_type()?.get_name()?.ends_with("[]")
                } else {
                    value.get_type()?.get_name()? == expected_type_name
                };

                if !correct {
                    anyhow::bail!(
                        "Expected {} got {}",
                        expected_type_name,
                        value.get_type()?.get_name()?
                    );
                }

                Ok(value)
            }
        }

        impl<$( $generic: MemoryRead ),*> AsRef<Pointer> for $ty<$( $generic ),*> {
            fn as_ref(&self) -> &Pointer {
                &self.0
            }
        }

        impl<$( $generic: MemoryRead ),*> crate::memory::ObjectPointer for $ty<$( $generic ),*> { }
    };
}

pub(crate) use count;
pub(crate) use object_type;

#[derive(Debug, Clone, Copy)]
pub struct ProcessMemoryView {
    process: Process,
}

impl ProcessMemoryView {
    pub fn new(process: Process) -> Self {
        Self { process }
    }

    pub fn read<T: MemoryRead>(&self, address: u64) -> Result<T> {
        T::read(self, address)
    }

    pub fn read_bytes(&self, address: u64, buffer: &mut [u8]) -> Result<usize> {
        self.process.read_memory(address, buffer)
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
    const SIZE: usize;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self>;
}

impl MemoryRead for bool {
    const SIZE: usize = 1;

    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 1];
        view.read_exact(address, &mut buffer)?;

        Ok(buffer[0] != 0)
    }
}

impl MemoryRead for f32 {
    const SIZE: usize = 4;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 4];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_f32(&buffer))
    }
}

impl MemoryRead for f64 {
    const SIZE: usize = 7;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 8];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_f64(&buffer))
    }
}

impl MemoryRead for u64 {
    const SIZE: usize = 8;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 8];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_u64(&buffer))
    }
}

impl MemoryRead for u32 {
    const SIZE: usize = 4;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 4];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_u32(&buffer))
    }
}

impl MemoryRead for i64 {
    const SIZE: usize = 8;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 8];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_i64(&buffer))
    }
}

impl MemoryRead for i32 {
    const SIZE: usize = 4;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let mut buffer = [0; 4];
        view.read_exact(address, &mut buffer)?;

        Ok(NativeEndian::read_i32(&buffer))
    }
}

impl MemoryRead for String {
    const SIZE: usize = 8;
    fn read(view: &ProcessMemoryView, address: u64) -> Result<Self> {
        let address = view.read(address)?;

        let mut buffer = vec![0; 1024];
        view.read_exact(address, &mut buffer)?;

        let len = buffer
            .iter()
            .position(|&b| b == 0)
            .expect("no null terminator");

        let value = String::from_utf8(buffer[0..len].to_vec())?;

        Ok(value)
    }
}

#[derive(Debug, Clone)]
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
    const SIZE: usize = 8;
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
    pub fn get_name(&self) -> Result<String> {
        self.0.read(0x10)
    }

    pub fn get_statics(&self) -> Result<TypeStatics> {
        self.0.read(0xb8)
    }
}

pointer_type!(TypeStatics);
impl TypeStatics {
    pub fn field<T: MemoryRead>(&self, offset: u64) -> Result<T> {
        self.0.read(offset)
    }
}

pointer_type!(Object);
impl TryFrom<Pointer> for Object {
    type Error = anyhow::Error;

    fn try_from(value: Pointer) -> Result<Self> {
        Ok(Self(value))
    }
}

impl Into<Pointer> for Object {
    fn into(self) -> Pointer {
        self.0
    }
}

impl AsRef<Pointer> for Object {
    fn as_ref(&self) -> &Pointer {
        &self.0
    }
}

impl ObjectPointer for Object {}

pub trait ObjectPointer:
    Sized + TryFrom<Pointer, Error = anyhow::Error> + AsRef<Pointer> + Into<Pointer>
{
    fn cast<T: ObjectPointer>(self) -> Result<T> {
        T::try_from(self.as_ref().clone())
    }

    fn get_type(&self) -> Result<TypeInfo> {
        self.as_ref().read(0x0)
    }

    unsafe fn field<T: MemoryRead>(&self, offset: u64) -> Result<T> {
        self.as_ref().read(0x10 + offset)
    }
}
