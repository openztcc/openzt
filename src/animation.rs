use std::mem;

use crate::binary_parsing::{read_le_primitive, read_string, write_le_primitive, write_string};

/// Top level animation struct, contains an optional header, animation speed, palette filename, number of frames and the frames themselves
#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Animation {
    /// An optional header, generally not present in simpler animations
    pub header: Option<Header>,
    /// Animation speed, in milliseconds. The time between each frame
    pub animation_speed: u32,
    /// Length of the palette filename
    pub palette_filename_length: u32,
    /// The filepath to the palette file used in the animation
    pub palette_filename: String,
    /// The number of frames in the animation, not including the extra frame if present
    pub num_frames: u32,
    /// The frames in the animation, including the extra frame if present
    pub frames: Vec<Frame>,
}

/// Optional header, not present in all animations
#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Header {
    /// The string ZTAF (FATZ when written in little endian)
    pub ztaf_string: u32,
    pub empty_4_bytes: u32,
    /// Whether the animation has an extra (background) frame, not included in the [Animation::num_frames] count
    pub extra_frame: bool,
}

/// A single frame in an animation, contains the number of bytes, pixel height, pixel width, vertical offset, horizontal offset, a mystery u16 and the lines
#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Frame {
    /// Number of bytes in the grame
    pub num_bytes: u32,
    /// Number of vertical pixels
    pub pixel_height: u16,
    /// Number of horizontal pixels, also the number of lines in the frame
    pub pixel_width: u16,

    pub vertical_offset_y: u16,
    pub horizontal_offset_x: u16,
    /// Unknown 2 bytes
    pub mystery_u16: u16,
    /// Lines in the frame, each line contains a number of draw instructions
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

/// A single line in a frame, contains the number of draw instructions and the draw instructions themselves
#[derive(Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Line {
    /// The number of individual draw instructions in the line
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
    /// The offset from the previous draw instruction, skipped pixels are transparent
    pub offset: u8,
    /// The number of colors in the draw instruction
    pub num_colors: u8,
    /// The colors in the draw instruction, each color is a palette index
    pub colors: Vec<u8>,
}

// TODO: Write test based on mod data
impl Animation {
    /// Parses a ZTAF animation file into an [Animation] struct, assumes little endian
    /// If the file has extra data at the end, it will be ignored
    pub fn parse(data: &[u8]) -> anyhow::Result<Animation> {
        let mut index = 0;
        let mut header = None;
        let maybe_header = read_le_primitive(data, &mut index)?;
        let animation_speed = match maybe_header {
            0x5A544146 => {
                header = Some(Header {
                    ztaf_string: maybe_header,
                    empty_4_bytes: read_le_primitive(data, &mut index)?,
                    extra_frame: read_le_primitive(data, &mut index)?,
                });
                read_le_primitive(data, &mut index)?
            }
            _ => maybe_header,
        };

        let palette_filename_length = read_le_primitive(data, &mut index)?;
        let palette_filename = read_string(data, &mut index, palette_filename_length as usize);
        let num_frames = read_le_primitive(data, &mut index)?;
        let mut frames = Vec::new();
        for _ in 0..num_frames + {
            if header.clone().is_some_and(|h| h.extra_frame) {
                1
            } else {
                0
            }
        } {
            let num_bytes = read_le_primitive(data, &mut index)?;
            let pixel_height = read_le_primitive(data, &mut index)?;
            let pixel_width = read_le_primitive(data, &mut index)?;
            let vertical_offset_y = read_le_primitive(data, &mut index)?;
            let horizontal_offset_x = read_le_primitive(data, &mut index)?;
            let mystery_u16 = read_le_primitive(data, &mut index)?;
            let mut lines = Vec::new();
            for _ in 0..pixel_height {
                let num_draw_instructions = read_le_primitive(data, &mut index)?;
                let mut draw_instructions = Vec::new();
                for _ in 0..num_draw_instructions {
                    let offset = read_le_primitive(data, &mut index)?;
                    let num_colors = read_le_primitive(data, &mut index)?;
                    let mut colors = Vec::new();
                    for _ in 0..num_colors {
                        colors.push(read_le_primitive(data, &mut index)?);
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

        Ok(Animation {
            header,
            animation_speed,
            palette_filename_length,
            palette_filename,
            num_frames,
            frames,
        })
    }

    /// Writes the [Animation] struct to a byte array, little endian
    pub fn write(self) -> anyhow::Result<(Vec<u8>, usize)> {
        let mut accumulator: usize = 0;
        let mut bytes = Vec::new();
        if let Some(header) = self.header {
            write_le_primitive(&mut bytes, header.ztaf_string, &mut accumulator);
            write_le_primitive(&mut bytes, header.empty_4_bytes, &mut accumulator);
            write_le_primitive(&mut bytes, header.extra_frame, &mut accumulator);
        }
        write_le_primitive(&mut bytes, self.animation_speed, &mut accumulator);
        write_string(&mut bytes, &self.palette_filename, &mut accumulator)?;
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
        Ok((bytes, accumulator))
    }

    /// Duplicates a range of pixel rows in a frame, increasing the pixel height and number of bytes in the frame
    ///
    /// Useful for resizing UI elements, only works on an individual frame
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

    /// Changes the pallette filename to a new value, updating the length of the filename
    pub fn set_palette_filename(&mut self, palette_filename: String) {
        self.palette_filename = palette_filename;
        self.palette_filename_length = self.palette_filename.len() as u32 + 1;
    }
}

#[cfg(test)]
mod parsing_tests {

    use super::Animation;

    #[test]
    fn test_parse_simple_anim_no_header() {
        let animation = Animation::parse(include_bytes!("../resources/test/N-noheader")).unwrap();
        assert!(animation.header.is_none());
        assert_eq!(animation.palette_filename, "ui/sharedui/listbk/ltb.pal".to_string());
        assert_eq!(animation.num_frames, 1);
        assert_eq!(animation.frames.len(), 1);
    }

    #[test]
    fn test_parse_simple_anim_with_header() {
        let animation = Animation::parse(include_bytes!("../resources/test/N")).unwrap();
        assert!(animation.header.is_some());
        assert!(!animation.header.unwrap().extra_frame);
        assert_eq!(animation.palette_filename, "ANIMALS/01BFCC32/ICMEIOLA/ICMEIOLA.PAL".to_string());
        assert_eq!(animation.num_frames, 1);
        assert_eq!(animation.frames.len(), 1);
    }

    #[test]
    fn test_parse_and_write() {
        let animation = Animation::parse(include_bytes!("../resources/test/N")).unwrap();
        let animation_to_write = animation.clone();
        let (animation_bytes, _) = animation_to_write.write().unwrap();
        let animation_2 = Animation::parse(&animation_bytes[..]).unwrap();
        assert!(animation == animation_2);
    }

    #[test]
    fn test_calc_byte_size() {
        let animation = Animation::parse(include_bytes!("../resources/test/N")).unwrap();
        assert_eq!(animation.frames[0].num_bytes, animation.frames[0].calc_byte_size() as u32);
    }

    #[test]
    fn test_parse_modify_and_write() {
        let animation = Animation::parse(include_bytes!("../resources/test/N")).unwrap();
        let mut animation_to_modify = animation.clone();
        animation_to_modify.duplicate_pixel_rows(0, 0, 1).unwrap();
        assert_eq!(animation.frames[0].pixel_height + 1, animation_to_modify.frames[0].pixel_height);
        animation_to_modify.set_palette_filename(animation.palette_filename.clone());
        assert_eq!(animation.palette_filename_length, animation_to_modify.palette_filename_length);
        let (animation_bytes, _) = animation_to_modify.write().unwrap();
        let animation_2 = Animation::parse(&animation_bytes[..]).unwrap();
        assert_eq!(animation.frames[0].pixel_height + 1, animation_2.frames[0].pixel_height);
        assert_eq!(animation.palette_filename, animation_2.palette_filename);
        assert_eq!(animation.palette_filename_length, animation_2.palette_filename_length);
    }
}
