use std::mem;

use crate::parsing::read_le_primitive;

#[repr(C)]
pub struct Animation {
    header: Option<Header>,
    animation_speed: u32, //May be limited to u16
    pallette_filename_length: u32,
    pallette_filename: String,
    num_frames: u32,
    frames: Vec<Frame>,
    //Random extra bytes?
}

#[repr(C)]
pub struct Header {
    ZTAF_string: u32,
    empty_byte: u32,
    extra_frame: bool,
}

#[repr(C)]
pub struct Frame {
    num_bytes: u32,
    pixel_height: u16,
    pixel_width: u16,
    vertical_offset_y: u16,
    horizontal_offset_x: u16,
    mystery_u16: u16,
    lines: Vec<Line>,
}

#[repr(C)]
pub struct Line {
    num_draw_instructions: u32,
    draw_instructions: Vec<DrawInstruction>,
}

#[repr(C)]
pub struct DrawInstruction {
    offset: u8,
    num_colors: u8,
    colors: Vec::<u8>,
}

// TODO: Write test based on mod data
impl Animation {
    pub fn parse(data : &[u8]) -> Animation {
            let mut index = 0;
            let mut header = None;
            let maybe_header = read_le_primitive(data, &mut index);
            let animation_speed = match maybe_header {
                0x5A544146 => {
                    header = Some(Header {
                        ZTAF_string: maybe_header,
                        empty_byte: read_le_primitive(data, &mut index),
                        extra_frame: read_le_primitive(data, &mut index),
                    });
                    read_le_primitive(data, &mut index)
                },
                _ =>  {
                    maybe_header
                }
            };
            // if maybe_header == 0x5A544146 {
            //     header = Some(Header {
            //         ZTAF_string: maybe_header,
            //         empty_byte: u32::from_le_bytes(data[4..8].try_into().unwrap()),
            //         extra_frame: u8::from_le_bytes(data[8..9].try_into().unwrap()) == 1,
            //     });
            //     let animation_speed = u32::from_le_bytes(data[9..13].try_into().unwrap());
            //     index = 13;
            // } else {
            //     let animation_speed = maybe_header;
            //     index = 4;
            // }
        
        // let mut cursor = Cursor::new(data);
        // let header = Header::parse(&mut cursor);
        // let animation_speed = cursor.read_u32::<LittleEndian>().unwrap();
        // let pallette_filename_length = cursor.read_u32::<LittleEndian>().unwrap();
        // let pallette_filename = cursor.read_string(pallette_filename_length).unwrap();
        // let num_frames = cursor.read_u32::<LittleEndian>().unwrap();
        // let mut frames = Vec::new();
        // for _ in 0..num_frames {
        //     frames.push(Frame::parse(&mut cursor));
        // }
        Animation {
            header: header,
            animation_speed: animation_speed,
            pallette_filename_length: 5,
            pallette_filename: "Test".to_string(),
            num_frames: 0,
            frames: Vec::new(),
            // pallette_filename,
            // num_frames,
            // frames,
        }
    }
}

#[cfg(test)]
mod parsing_tests {

    use super::Animation;

    use std::path::PathBuf;

    fn get_test_dir() -> PathBuf {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test");
        d
    }

    #[test]
    fn test_parse_simple_anim_no_header() {
        let animation = Animation::parse(include_bytes!("../resources/test/N-noheader"));
        assert!(animation.header.is_none());
    }

    #[test]
    fn test_parse_simple_anim_with_header() {
        let animation = Animation::parse(include_bytes!("../resources/test/n"));
        assert!(animation.header.is_some());
    }
}