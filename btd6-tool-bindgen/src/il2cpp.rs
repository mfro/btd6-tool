use std::{fs::File, io::Read, mem::size_of, path::Path, ptr::read};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    combinator::{map_res, opt},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded},
    IResult,
};

use byteorder::{ByteOrder, LittleEndian, NativeEndian};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn next<T: ReadMetadata>(src: &mut &[u8]) -> T {
    let value = T::read(&src[0..T::SIZE]);
    *src = &src[T::SIZE..];
    value
}

trait ReadMetadata {
    const SIZE: usize;

    fn read(raw: &[u8]) -> Self;
}

impl ReadMetadata for u16 {
    const SIZE: usize = 2;

    fn read(raw: &[u8]) -> Self {
        NativeEndian::read_u16(raw)
    }
}

impl ReadMetadata for u32 {
    const SIZE: usize = 4;

    fn read(raw: &[u8]) -> Self {
        NativeEndian::read_u32(raw)
    }
}

macro_rules! metadata {
    ( $typename:ident { $name1:ident: $ty1:ty, $($name:ident: $ty:ty,)* } ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct $typename {
            $name1: $ty1,
            $( $name: $ty, )*
        }

        impl ReadMetadata for $typename {
            const SIZE: usize = (<$ty1>::SIZE) $( + (<$ty>::SIZE) )*;

            fn read(mut raw: &[u8]) -> Self {
                let $name1 = next::<$ty1>(&mut raw) as _;

                $(
                    let $name = next::<$ty>(&mut raw) as _;
                )*

                Self { $name1 $(, $name )* }
            }
        }
    };
}

metadata! {
    FileRange {
        offset: u32,
        len: u32,
    }
}

metadata! {
    MetadataHeader {
        magic: u32,
        version: u32,
        string_literal: FileRange,
        string_literal_data: FileRange,
        string: FileRange,
        events: FileRange,
        properties: FileRange,
        methods: FileRange,
        parameter_default_values: FileRange,
        field_default_values: FileRange,
        field_and_poarameter_default_value_data: FileRange,
        field_marshaled_sizes: FileRange,
        parameters: FileRange,
        fields: FileRange,
        generic_parameters: FileRange,
        generic_parameter_contraints: FileRange,
        generic_containers: FileRange,
        nested_types: FileRange,
        interfaces: FileRange,
        vtable_methods: FileRange,
        interface_offsets: FileRange,
        type_definitions: FileRange,
    }
}

metadata! {
    TypeDefinition {
        name_offset: u32,
        namespace_offset: u32,
        value_type_offset: u32,
        declaring_type_offset: u32,
        parent_type_offset: u32,
        elemnt_type_offset: u32,
        generic_container_offset: u32,
        flags: u32,

        fields_start: u32,
        methods_start: u32,
        events_start: u32,
        properties_start: u32,
        nested_types_start: u32,
        interfaces_start: u32,
        vtable_start: u32,
        interface_offsets_start: u32,

        methods_count: u16,
        properties_count: u16,
        fields_count: u16,
        events_count: u16,
        nested_types_count: u16,
        vtable_count: u16,
        interfaces_count: u16,
        interface_offsets_count: u16,

        bitfield: u32,
        token: u32,
    }
}

metadata! {
    FieldDefinition {
        name_offset: u32,
        type_index: u32,
        token: u32,
    }
}

struct MetadataReader {
    raw: Vec<u8>,
    header: MetadataHeader,
}

impl MetadataReader {
    pub fn open(src: impl AsRef<Path>) -> Result<MetadataReader> {
        let mut file = File::open(
            r"E:\ephemeral\games\Epic Games\BloonsTD6\BloonsTD6_Data\il2cpp_data\Metadata\global-metadata.dat",
        )?;
        let mut raw = Vec::with_capacity(file.metadata()?.len() as usize);
        file.read_to_end(&mut raw)?;

        let header = MetadataHeader::read(&raw);

        Ok(Self { header, raw })
    }

    pub fn string(&self, offset: u32) -> Result<String> {
        let offset = self.header.string.offset as usize + offset as usize;
        let len = self.raw[offset..].iter().position(|&ch| ch == 0).unwrap();

        let str = String::from_utf8(self.raw[offset..offset + len].to_vec())?;

        Ok(str)
    }

    pub fn field(&self, index: u32) -> Result<FieldDefinition> {
        let offset = self.header.fields.offset as usize + index as usize * FieldDefinition::SIZE;
        let data = FieldDefinition::read(&self.raw[offset..offset + FieldDefinition::SIZE]);

        Ok(data)
    }

    pub fn ty(&self, index: u32) -> Result<TypeDefinition> {
        let offset =
            self.header.type_definitions.offset as usize + index as usize * TypeDefinition::SIZE;
        let data = TypeDefinition::read(&self.raw[offset..offset + TypeDefinition::SIZE]);

        Ok(data)
    }
}

fn main() -> Result<()> {
    let reader = MetadataReader::open(
        r"E:\ephemeral\games\Epic Games\BloonsTD6\BloonsTD6_Data\il2cpp_data\Metadata\global-metadata.dat",
    )?;

    // fields
    let count = reader.header.type_definitions.len / TypeDefinition::SIZE as u32;

    for i in 0..count {
        let ty = reader.ty(i)?;

        println!(
            "  {i}:  {}.{} {} {}",
            reader.string(ty.namespace_offset)?,
            reader.string(ty.name_offset)?,
            ty.fields_start,
            ty.fields_count
        );

        for i in ty.fields_start..ty.fields_start + ty.fields_count as u32 {
            let field = reader.field(i)?;

            println!("    {}", reader.string(field.name_offset)?);
        }
    }

    // fields
    // let count = reader.header.fields.len / FieldDefinition::SIZE as u32;

    // for i in 0..count {
    //     let field = reader.field(i)?;

    //     println!(
    //         "  {i}:  {}.{}",
    //         field.type_index,
    //         reader.string(field.name_offset)?
    //     );

    // let ty = reader.ty(field.type_index)?;

    // println!(
    //     "  {i}:  {}.{}",
    //     reader.string(ty.name_offset)?,
    //     reader.string(field.name_offset)?
    // );
    // }

    // let count = reader.header.type_definitions.len as usize / TypeDefinition::SIZE;

    // for i in 0..count {
    //     let offset = reader.header.type_definitions.offset as usize + i * TypeDefinition::SIZE;
    //     let value = TypeDefinition::read(&contents[offset..offset + TypeDefinition::SIZE]);

    //     println!(
    //         "  {:?}",
    //         read_string(&contents, reader.header.string.offset + value.name_offset)?
    //     );
    //     println!(
    //         "  {:?}",
    //         read_string(
    //             &contents,
    //             reader.header.string.offset + value.namespace_offset
    //         )?
    //     );

    //     // let len = contents[header.string.offset  + name_index..]
    //     //     .iter()
    //     //     .position(|&ch| ch == 0)
    //     //     .unwrap();

    //     // let str = String::from_utf8(
    //     //     contents[header.string.offset  + name_index..header.string.offset  + name_index + len].to_vec(),
    //     // )?;

    //     // println!("{:?}", str);

    //     // let len = contents[header.string.offset  + namespace_index..]
    //     //     .iter()
    //     //     .position(|&ch| ch == 0)
    //     //     .unwrap();

    //     // let str = String::from_utf8(
    //     //     contents[header.string.offset  + namespace_index..header.string.offset  + namespace_index + len]
    //     //         .to_vec(),
    //     // )?;

    //     // println!("{:?}", str);
    // }

    Ok(())
}
