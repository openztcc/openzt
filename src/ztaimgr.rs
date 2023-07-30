
use crate::debug_dll::{get_from_memory, get_string_from_memory, get_zt_string_array_from_memory};
use tracing::info;


#[derive(Debug)]
#[repr(C)]
pub struct cls_4551cb {
    unknown_empty_1: u32,
    unknown_empty_2: u32,
    unknown_empty_3: u32,
    unknown_u32_1: u32,
    unknown_u32_2: u32,
    unknown_u32_3: u32,
    unknown_u32_4: u32,
    unknown_u32_5: u32,
    unknown_u32_6: u32,
    unknown_u8_1: u8,
    unknown_u8_2: u8,
    unknown_u8_3: u8,
    unknown_u8_4: u8,
    unknown_u32_7: u32,
    unknown_u32_8: u32,
    unknown_u32_9: u32,
    unknown_u32_10: u32,
    unknown_u32_11: u32,
    unknown_u32_12: u32,
    unknown_u32_13: u32,
    unknown_u32_14: u32,
    unknown_u32_15: u32,
    unknown_empty_5: u32,
    unknown_u32_16: u32,
    unknown_u32_17: u32,
    unknown_u8_5: u8,
    unknown_u8_6: u8,
    unknown_u8_7: u8,
    unknown_u8_8: u8,
    unknown_u8_9: u8,
}

pub fn read_cls_4551cb_from_memory(cls_4551cb_ptr: u32) -> cls_4551cb {
    cls_4551cb{
        unknown_empty_1: get_from_memory::<u32>(cls_4551cb_ptr + 0x0),
        unknown_empty_2: get_from_memory::<u32>(cls_4551cb_ptr + 0x4),
        unknown_empty_3: get_from_memory::<u32>(cls_4551cb_ptr + 0x8),
        unknown_u32_1: get_from_memory::<u32>(cls_4551cb_ptr + 0xC),
        unknown_u32_2: get_from_memory::<u32>(cls_4551cb_ptr + 0x10),
        unknown_u32_3: get_from_memory::<u32>(cls_4551cb_ptr + 0x14),
        unknown_u32_4: get_from_memory::<u32>(cls_4551cb_ptr + 0x18),
        unknown_u32_5: get_from_memory::<u32>(cls_4551cb_ptr + 0x1C),
        unknown_u32_6: get_from_memory::<u32>(cls_4551cb_ptr + 0x20),
        unknown_u8_1: get_from_memory::<u8>(cls_4551cb_ptr + 0x24),
        unknown_u8_2: get_from_memory::<u8>(cls_4551cb_ptr + 0x25),
        unknown_u8_3: get_from_memory::<u8>(cls_4551cb_ptr + 0x26),
        unknown_u8_4: get_from_memory::<u8>(cls_4551cb_ptr + 0x27),
        unknown_u32_7: get_from_memory::<u32>(cls_4551cb_ptr + 0x28),
        unknown_u32_8: get_from_memory::<u32>(cls_4551cb_ptr + 0x2C),
        unknown_u32_9: get_from_memory::<u32>(cls_4551cb_ptr + 0x30),
        unknown_u32_10: get_from_memory::<u32>(cls_4551cb_ptr + 0x34),
        unknown_u32_11: get_from_memory::<u32>(cls_4551cb_ptr + 0x38),
        unknown_u32_12: get_from_memory::<u32>(cls_4551cb_ptr + 0x3C),
        unknown_u32_13: get_from_memory::<u32>(cls_4551cb_ptr + 0x40),
        unknown_u32_14: get_from_memory::<u32>(cls_4551cb_ptr + 0x44),
        unknown_u32_15: get_from_memory::<u32>(cls_4551cb_ptr + 0x48),
        unknown_empty_5: get_from_memory::<u32>(cls_4551cb_ptr + 0x4C),
        unknown_u32_16: get_from_memory::<u32>(cls_4551cb_ptr + 0x50),
        unknown_u32_17: get_from_memory::<u32>(cls_4551cb_ptr + 0x54),
        unknown_u8_5: get_from_memory::<u8>(cls_4551cb_ptr + 0x58),
        unknown_u8_6: get_from_memory::<u8>(cls_4551cb_ptr + 0x59),
        unknown_u8_7: get_from_memory::<u8>(cls_4551cb_ptr + 0x5A),
        unknown_u8_8: get_from_memory::<u8>(cls_4551cb_ptr + 0x5B),
        unknown_u8_9: get_from_memory::<u8>(cls_4551cb_ptr + 0x5C),
    }
}

pub fn log_cls_4551cb(cls_4551cb: &cls_4551cb) {
    let str_1 = get_string_from_memory(cls_4551cb.unknown_u32_1);
    let str_array_1 = get_zt_string_array_from_memory(cls_4551cb.unknown_u32_4, cls_4551cb.unknown_u32_5);
    let str_array_2 = get_zt_string_array_from_memory(cls_4551cb.unknown_u32_7, cls_4551cb.unknown_u32_8);
    let str_2 = get_string_from_memory(cls_4551cb.unknown_u32_10);
    let str_3 = get_string_from_memory(cls_4551cb.unknown_u32_13);
    let class_ptr = get_from_memory::<u32>(cls_4551cb.unknown_u32_16);
    info!("str_1: {}", str_1);
    info!("str_array_1: {:?}", str_array_1);
    info!("str_array_2: {:?}", str_array_2);
    info!("str_2: {}", str_2);
    info!("str_3: {}", str_3);
    info!("class_ptr: {:#08x}", class_ptr);
    info!("unknown_ptr: {:#08x}", cls_4551cb.unknown_u32_17);
    info!("cls_4551cb: {:#?}", cls_4551cb);
}
