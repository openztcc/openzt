use std::mem;
use std::fmt;
use std::convert::{TryFrom, TryInto};

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

macro_rules! impl_EndianRead_for_ints (( $($int:ident),* ) => {
    $(
        impl EndianRead<[u8; <$int as TypeSize>::SIZE]> for $int {
            fn from_le_bytes(bytes: [u8; <$int as TypeSize>::SIZE]) -> Self { Self::from_le_bytes(bytes) }
            fn from_be_bytes(bytes: [u8; <$int as TypeSize>::SIZE]) -> Self { Self::from_be_bytes(bytes) }
        }
    )*
});

impl_EndianRead_for_ints!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl EndianRead<[u8; 1]> for bool {
    fn from_le_bytes(bytes: [u8; 1]) -> Self { bytes[0] != 0 }
    fn from_be_bytes(bytes: [u8; 1]) -> Self { bytes[0] != 0 }
}

pub fn read_le_primitive<'a, T, TArray> (bytes: &'a [u8], index: &mut usize) -> T
where
    T : EndianRead<TArray>,
    TArray : TryFrom<&'a [u8]>,
    <TArray as TryFrom<&'a [u8]>>::Error : fmt::Debug,
{
    let value_bytes = &bytes[*index..*index + mem::size_of::<T>()];
    *index += mem::size_of::<T>();
    T::from_le_bytes(value_bytes.try_into().unwrap())
}

pub fn read_string(bytes: &[u8], index: &mut usize, length: usize) -> String {
    let string_bytes = &bytes[*index..*index + length - 1];
    *index += length;
    String::from_utf8(string_bytes.to_vec()).unwrap()
}