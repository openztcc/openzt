use std::mem;

use crate::parsing::{read_le_primitive, read_string, write_le_primitive, write_string};

#[derive(Clone, PartialEq, Debug)]
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

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Header {
    ZTAF_string: u32,
    empty_4_bytes: u32,
    extra_frame: bool,
}

#[derive(Clone, PartialEq, Debug)]
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

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Line {
    num_draw_instructions: u8,
    draw_instructions: Vec<DrawInstruction>,
}

#[derive(Clone, PartialEq, Debug)]
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
                    empty_4_bytes: read_le_primitive(data, &mut index),
                    extra_frame: read_le_primitive(data, &mut index),
                });
                read_le_primitive(data, &mut index)
            },
            _ =>  {
                maybe_header
            }
        };

        let pallette_filename_length = read_le_primitive(data, &mut index);
        let pallette_filename = read_string(data, &mut index, pallette_filename_length as usize);
        let num_frames = read_le_primitive(data, &mut index);
        let mut frames = Vec::new();
        for i in 0..num_frames + {if header.clone().is_some_and(|h| h.extra_frame) {1} else {0}} {
            let num_bytes = read_le_primitive(data, &mut index);
            let pixel_height = read_le_primitive(data, &mut index);
            let pixel_width = read_le_primitive(data, &mut index);
            let vertical_offset_y = read_le_primitive(data, &mut index);
            let horizontal_offset_x = read_le_primitive(data, &mut index);
            let mystery_u16 = read_le_primitive(data, &mut index);
            let mut lines = Vec::new();
            for j in 0..pixel_height {
                let num_draw_instructions = read_le_primitive(data, &mut index);
                let mut draw_instructions = Vec::new();
                for k in 0..num_draw_instructions {
                    let offset = read_le_primitive(data, &mut index);
                    let num_colors = read_le_primitive(data, &mut index);
                    let mut colors = Vec::new();
                    for l in 0..num_colors {
                        colors.push(read_le_primitive(data, &mut index));
                    }
                    draw_instructions.push(DrawInstruction {
                        offset,
                        num_colors,
                        colors,
                    });
                }
                lines.push(Line {
                    num_draw_instructions,
                    draw_instructions,
                });
            }
            frames.push(Frame {
                num_bytes,
                pixel_height,
                pixel_width,
                vertical_offset_y,
                horizontal_offset_x,
                mystery_u16,
                lines,
            });
        } 

        Animation {
            header: header,
            animation_speed: animation_speed,
            pallette_filename_length,
            pallette_filename,
            num_frames,
            frames,
        }
    }

    pub fn write(self) -> (Vec<u8>, usize) {
        let mut accumulator: usize = 0;
        let mut bytes = Vec::new();
        match self.header {
            Some(header) => {
                write_le_primitive(&mut bytes, header.ZTAF_string, &mut accumulator);
                write_le_primitive(&mut bytes, header.empty_4_bytes, &mut accumulator);
                write_le_primitive(&mut bytes, header.extra_frame, &mut accumulator);
            },
            None => ()
        }
        write_le_primitive(&mut bytes, self.animation_speed, &mut accumulator);
        // write_le_primitive(&mut bytes, self.pallette_filename_length, &mut accumulator);
        write_string(&mut bytes, &self.pallette_filename, &mut accumulator);
        write_le_primitive(&mut bytes, self.num_frames, &mut accumulator);

        for frame in self.frames {
            write_le_primitive(&mut bytes, frame.num_bytes, &mut accumulator);
            write_le_primitive(&mut bytes, frame.pixel_height, &mut accumulator);
            write_le_primitive(&mut bytes, frame.pixel_width, &mut accumulator);
            write_le_primitive(&mut bytes, frame.vertical_offset_y, &mut accumulator);
            write_le_primitive(&mut bytes, frame.horizontal_offset_x, &mut accumulator);
            write_le_primitive(&mut bytes, frame.mystery_u16, &mut accumulator);

            for line in frame.lines {
                write_le_primitive(&mut bytes, line.num_draw_instructions, &mut accumulator);
                for draw_instruction in line.draw_instructions {
                    write_le_primitive(&mut bytes, draw_instruction.offset, &mut accumulator);
                    write_le_primitive(&mut bytes, draw_instruction.num_colors, &mut accumulator);
                    for color in draw_instruction.colors {
                        write_le_primitive(&mut bytes, color, &mut accumulator);
                    }
                }
            }
        }
        bytes.shrink_to_fit();
        (bytes, accumulator)
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
        assert_eq!(animation.pallette_filename, "ui/sharedui/listbk/ltb.pal".to_string());
        assert_eq!(animation.num_frames, 1);
        assert_eq!(animation.frames.len(), 1);

    }

    #[test]
    fn test_parse_simple_anim_with_header() {
        let animation = Animation::parse(include_bytes!("../resources/test/N"));
        assert!(animation.header.is_some());
        assert_eq!(animation.header.unwrap().extra_frame, false);
        assert_eq!(animation.pallette_filename, "ANIMALS/01BFCC32/ICMEIOLA/ICMEIOLA.PAL".to_string());
        assert_eq!(animation.num_frames, 1);
        assert_eq!(animation.frames.len(), 1);
    }

    #[test]
    fn test_parse_and_write() {
        let animation = Animation::parse(include_bytes!("../resources/test/N"));
        let animation_to_write = animation.clone();
        let (animation_bytes, length) = animation_to_write.write();
        let animation_2 = Animation::parse(&animation_bytes[..]);
        assert_eq!(animation, animation_2);
    }
}