use std::{ffi::CString, fmt, mem};

trait TypeSize {
    const SIZE: usize;
}

macro_rules! impl_TypeSize (( $($int:ident),* ) => {
    $(
        impl TypeSize for $int {
            const SIZE: usize = std::mem::size_of::<$int>();
        }
    )*
});

impl_TypeSize!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

pub trait EndianRead<TArray> {
    fn from_le_bytes(bytes: TArray) -> Self;
    fn from_be_bytes(bytes: TArray) -> Self;
}

pub trait EndianWrite {
    fn to_le_bytes(self) -> Vec<u8>;
    fn to_be_bytes(self) -> Vec<u8>;
}

macro_rules! impl_EndianRead_for_ints (( $($int:ident),* ) => {
    $(
        impl EndianRead<[u8; <$int as TypeSize>::SIZE]> for $int {
            fn from_le_bytes(bytes: [u8; <$int as TypeSize>::SIZE]) -> Self { Self::from_le_bytes(bytes) }
            fn from_be_bytes(bytes: [u8; <$int as TypeSize>::SIZE]) -> Self { Self::from_be_bytes(bytes) }
        }
    )*
});

macro_rules! impl_EndianWrite_for_ints (( $($int:ident),* ) => {
    $(
        impl EndianWrite for $int {
            fn to_le_bytes(self) -> Vec<u8> { self.to_le_bytes().to_vec() }
            fn to_be_bytes(self) -> Vec<u8> { self.to_be_bytes().to_vec() }
        }
    )*
});

impl_EndianRead_for_ints!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl_EndianWrite_for_ints!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl EndianRead<[u8; 1]> for bool {
    fn from_le_bytes(bytes: [u8; 1]) -> Self {
        bytes[0] != 0
    }
    fn from_be_bytes(bytes: [u8; 1]) -> Self {
        bytes[0] != 0
    }
}

impl EndianWrite for bool {
    fn to_le_bytes(self) -> Vec<u8> {
        vec![self as u8]
    }
    fn to_be_bytes(self) -> Vec<u8> {
        vec![self as u8]
    }
}

pub fn read_le_primitive<'a, T, TArray>(bytes: &'a [u8], index: &mut usize) -> Result<T, <TArray as TryFrom<&'a [u8]>>::Error>
where
    T: EndianRead<TArray>,
    TArray: TryFrom<&'a [u8]>,
    <TArray as TryFrom<&'a [u8]>>::Error: fmt::Debug,
{
    let value_bytes = &bytes[*index..*index + mem::size_of::<T>()];
    *index += mem::size_of::<T>();
    Ok(T::from_le_bytes(value_bytes.try_into()?))
}

pub fn write_le_primitive<T: EndianWrite>(vec: &mut Vec<u8>, value: T, accumulator: &mut usize) {
    *accumulator += mem::size_of::<T>();
    vec.extend(value.to_le_bytes());
}

pub fn read_string(bytes: &[u8], index: &mut usize, length: usize) -> String {
    let string_bytes = &bytes[*index..*index + length - 1];
    *index += length;
    String::from_utf8(string_bytes.to_vec()).unwrap()
}

pub fn write_string(vec: &mut Vec<u8>, string: &str, accumulator: &mut usize) -> Result<(), std::ffi::NulError> {
    let length = string.len() + 1;
    let c_string = CString::new(string)?;
    *accumulator += length;
    write_le_primitive(vec, (length) as u32, accumulator);
    vec.extend_from_slice(c_string.as_bytes_with_nul());
    Ok(())
}
