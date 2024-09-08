use std::mem;

use crate::parsing::{read_le_primitive, read_string, write_le_primitive, write_string};

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Animation {
    pub header: Option<Header>,
    pub animation_speed: u32,
    pub palette_filename_length: u32,
    pub palette_filename: String,
    pub num_frames: u32,
    pub frames: Vec<Frame>,
}

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Header {
    pub ztaf_string: u32,
    pub empty_4_bytes: u32,
    pub extra_frame: bool,
}

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Frame {
    pub num_bytes: u32,
    pub pixel_height: u16,
    pub pixel_width: u16,
    pub vertical_offset_y: u16,
    pub horizontal_offset_x: u16,
    pub mystery_u16: u16,
    pub lines: Vec<Line>,
}

impl Frame {
    pub fn calc_byte_size(&self) -> usize {
        let mut size = mem::size_of::<u16>() * 5;
        for line in &self.lines {
            size += line.calc_byte_size()
        }
        size
    }
}

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Line {
    pub num_draw_instructions: u8,
    pub draw_instructions: Vec<DrawInstruction>,
}

impl Line {
    pub fn calc_byte_size(&self) -> usize {
        let mut size = mem::size_of::<u8>();
        for draw_instruction in &self.draw_instructions {
            size += mem::size_of::<u8>() + mem::size_of::<u8>() + (draw_instruction.colors.len() * mem::size_of::<u8>());
        }
        size
    }
}

#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct DrawInstruction {
    pub offset: u8,
    pub num_colors: u8,
    pub colors: Vec<u8>,
}

// TODO: Write test based on mod data
impl Animation {
    pub fn parse(data: &[u8]) -> Animation {
        let mut index = 0;
        let mut header = None;
        let maybe_header = read_le_primitive(data, &mut index);
        let animation_speed = match maybe_header {
            0x5A544146 => {
                header = Some(Header {
                    ztaf_string: maybe_header,
                    empty_4_bytes: read_le_primitive(data, &mut index),
                    extra_frame: read_le_primitive(data, &mut index),
                });
                read_le_primitive(data, &mut index)
            }
            _ => maybe_header,
        };

        let palette_filename_length = read_le_primitive(data, &mut index);
        let palette_filename = read_string(data, &mut index, palette_filename_length as usize);
        let num_frames = read_le_primitive(data, &mut index);
        let mut frames = Vec::new();
        for _ in 0..num_frames + {
            if header.clone().is_some_and(|h| h.extra_frame) {
                1
            } else {
                0
            }
        } {
            let num_bytes = read_le_primitive(data, &mut index);
            let pixel_height = read_le_primitive(data, &mut index);
            let pixel_width = read_le_primitive(data, &mut index);
            let vertical_offset_y = read_le_primitive(data, &mut index);
            let horizontal_offset_x = read_le_primitive(data, &mut index);
            let mystery_u16 = read_le_primitive(data, &mut index);
            let mut lines = Vec::new();
            for _ in 0..pixel_height {
                let num_draw_instructions = read_le_primitive(data, &mut index);
                let mut draw_instructions = Vec::new();
                for _ in 0..num_draw_instructions {
                    let offset = read_le_primitive(data, &mut index);
                    let num_colors = read_le_primitive(data, &mut index);
                    let mut colors = Vec::new();
                    for _ in 0..num_colors {
                        colors.push(read_le_primitive(data, &mut index));
                    }
                    draw_instructions.push(DrawInstruction { offset, num_colors, colors });
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
            header,
            animation_speed,
            palette_filename_length,
            palette_filename,
            num_frames,
            frames,
        }
    }

    pub fn write(self) -> (Vec<u8>, usize) {
        let mut accumulator: usize = 0;
        let mut bytes = Vec::new();
        if let Some(header) = self.header {
            write_le_primitive(&mut bytes, header.ztaf_string, &mut accumulator);
            write_le_primitive(&mut bytes, header.empty_4_bytes, &mut accumulator);
            write_le_primitive(&mut bytes, header.extra_frame, &mut accumulator);
        }
        write_le_primitive(&mut bytes, self.animation_speed, &mut accumulator);
        write_string(&mut bytes, &self.palette_filename, &mut accumulator);
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

    pub fn duplicate_pixel_rows(&mut self, frame: usize, start_index: usize, end_index: usize) -> Result<&mut Self, &'static str> {
        let mut additional_bytes: usize = 0;
        if start_index > end_index {
            return Err("Start index must be less than end index");
        }
        if self.frames.len() < frame {
            return Err("Frame index out of bounds");
        }
        if self.frames[frame].lines.len() < end_index {
            return Err("End index out of bounds");
        }
        let mut new_lines = Vec::new();
        for line in self.frames[frame].lines[start_index..end_index].iter() {
            new_lines.push(line.clone());
            additional_bytes += line.calc_byte_size();
        }

        self.frames[frame].lines.splice(end_index..end_index, new_lines);

        self.frames[frame].num_bytes += additional_bytes as u32;

        self.frames[frame].pixel_height += (end_index - start_index) as u16;

        Ok(self)
    }

    pub fn set_palette_filename(&mut self, palette_filename: String) {
        self.palette_filename = palette_filename;
        self.palette_filename_length = self.palette_filename.len() as u32 + 1;
    }
}

#[cfg(test)]
mod parsing_tests {

    use std::path::PathBuf;

    use super::Animation;

    #[test]
    fn test_parse_simple_anim_no_header() {
        let animation = Animation::parse(include_bytes!("../resources/test/N-noheader"));
        assert!(animation.header.is_none());
        assert_eq!(animation.palette_filename, "ui/sharedui/listbk/ltb.pal".to_string());
        assert_eq!(animation.num_frames, 1);
        assert_eq!(animation.frames.len(), 1);
    }

    #[test]
    fn test_parse_simple_anim_with_header() {
        let animation = Animation::parse(include_bytes!("../resources/test/N"));
        assert!(animation.header.is_some());
        assert!(!animation.header.unwrap().extra_frame);
        assert_eq!(animation.palette_filename, "ANIMALS/01BFCC32/ICMEIOLA/ICMEIOLA.PAL".to_string());
        assert_eq!(animation.num_frames, 1);
        assert_eq!(animation.frames.len(), 1);
    }

    #[test]
    fn test_parse_and_write() {
        let animation = Animation::parse(include_bytes!("../resources/test/N"));
        let animation_to_write = animation.clone();
        let (animation_bytes, _) = animation_to_write.write();
        let animation_2 = Animation::parse(&animation_bytes[..]);
        assert!(animation == animation_2);
    }

    #[test]
    fn test_calc_byte_size() {
        let animation = Animation::parse(include_bytes!("../resources/test/N"));
        assert_eq!(animation.frames[0].num_bytes, animation.frames[0].calc_byte_size() as u32);
    }

    #[test]
    fn test_parse_modify_and_write() {
        let animation = Animation::parse(include_bytes!("../resources/test/N"));
        let mut animation_to_modify = animation.clone();
        animation_to_modify.duplicate_pixel_rows(0, 0, 1).unwrap();
        assert_eq!(animation.frames[0].pixel_height + 1, animation_to_modify.frames[0].pixel_height);
        animation_to_modify.set_palette_filename(animation.palette_filename.clone());
        assert_eq!(animation.palette_filename_length, animation_to_modify.palette_filename_length);
        let (animation_bytes, _) = animation_to_modify.write();
        let animation_2 = Animation::parse(&animation_bytes[..]);
        assert_eq!(animation.frames[0].pixel_height + 1, animation_2.frames[0].pixel_height);
        assert_eq!(animation.palette_filename, animation_2.palette_filename);
        assert_eq!(animation.palette_filename_length, animation_2.palette_filename_length);
    }
}
