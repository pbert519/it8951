use crate::{serialization_helper::get_entires_per_row, AreaImgInfo};
use alloc::vec::Vec;
use embedded_graphics_core::{
    pixelcolor::{Gray4, GrayColor},
    primitives::Rectangle,
};

/// Converts a rectangle with a uniform color to frame buffer segments with area information.
pub struct AreaSerializer {
    area: Rectangle,
    rows_per_step: usize,
    buffer: Vec<u8>,
}

impl AreaSerializer {
    pub fn new(area: Rectangle, color: Gray4, buffer_size: usize) -> Self {
        let raw_color = color.luma();
        let data_entry = raw_color << 4 | raw_color;

        assert!(buffer_size % 2 == 0, "Buffer size must be aligned to u16");
        // calculate the buffer size
        let entries_per_row = get_entires_per_row(area) as usize * 2; // convert length from u16 to u8
        let rows_per_step = (buffer_size / entries_per_row).min(area.size.height as usize);
        assert!(rows_per_step > 0, "Buffer size to small for one row");
        let buffer = vec![data_entry; entries_per_row * rows_per_step];

        AreaSerializer {
            area,
            rows_per_step,
            buffer,
        }
    }
}

pub struct AreaSerializerIterator<'a> {
    area_serializer: &'a AreaSerializer,
    row: usize,
}
impl<'a> AreaSerializerIterator<'a> {
    pub fn new(area_serializer: &'a AreaSerializer) -> AreaSerializerIterator<'a> {
        AreaSerializerIterator {
            area_serializer,
            row: 0,
        }
    }
}

impl<'a> Iterator for AreaSerializerIterator<'a> {
    type Item = (AreaImgInfo, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let area_height = self.area_serializer.area.size.height;
        if self.row >= area_height as usize {
            return None;
        }

        let start_row = self.row;

        self.row = (start_row + self.area_serializer.rows_per_step).min(area_height as usize);

        Some((
            AreaImgInfo {
                area_x: self.area_serializer.area.top_left.x as u16,
                area_y: (self.area_serializer.area.top_left.y + start_row as i32) as u16,
                area_w: self.area_serializer.area.size.width as u16,
                area_h: (self.row - start_row) as u16,
            },
            &self.area_serializer.buffer,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics_core::prelude::*;

    const BOUNDING_BOX_DEFAULT: Rectangle = Rectangle {
        top_left: Point { x: 0, y: 0 },
        size: Size {
            width: 10,
            height: 10,
        },
    };

    #[test]
    // single pixel in bounding box at pos 0
    fn test_pixel_0() {
        let area = Rectangle {
            top_left: Point { x: 0, y: 0 },
            size: Size {
                width: 1,
                height: 1,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 0,
                    area_y: 0,
                    area_w: 1,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // single pixel in bounding box at pos 1
    fn test_pixel_1() {
        let area = Rectangle {
            top_left: Point { x: 1, y: 1 },
            size: Size {
                width: 1,
                height: 1,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 1,
                    area_y: 1,
                    area_w: 1,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }
    #[test]
    // single pixel in bounding box at pos 2
    fn test_pixel_2() {
        let area = Rectangle {
            top_left: Point { x: 2, y: 1 },
            size: Size {
                width: 1,
                height: 1,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 2,
                    area_y: 1,
                    area_w: 1,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }
    #[test]
    // single pixel in bounding box at pos 3
    fn test_pixel_3() {
        let area = Rectangle {
            top_left: Point { x: 3, y: 1 },
            size: Size {
                width: 1,
                height: 1,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 1,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // 4 pixels in a row, aligned
    fn test_pixel_single_row_packed() {
        let area = Rectangle {
            top_left: Point { x: 4, y: 1 },
            size: Size {
                width: 4,
                height: 1,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);

        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 4,
                    area_y: 1,
                    area_w: 4,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // 3 pixels in a row, not aligned
    fn test_pixel_single_row() {
        let area = Rectangle {
            top_left: Point { x: 3, y: 1 },
            size: Size {
                width: 3,
                height: 1,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 3,
                    area_h: 1
                },
                [0xAA, 0xAA, 0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, aligned
    fn test_pixel_rows_packed() {
        let area = Rectangle {
            top_left: Point { x: 4, y: 1 },
            size: Size {
                width: 4,
                height: 2,
            },
        };
        let area_s =
            AreaSerializer::new(area.intersection(&BOUNDING_BOX_DEFAULT), Gray4::new(0xA), 2);
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 4,
                    area_y: 1,
                    area_w: 4,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 4,
                    area_y: 2,
                    area_w: 4,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, not aligned
    fn test_pixel_rows() {
        let area = Rectangle {
            top_left: Point { x: 3, y: 1 },
            size: Size {
                width: 3,
                height: 2,
            },
        };
        let area_s =
            AreaSerializer::new(area.intersection(&BOUNDING_BOX_DEFAULT), Gray4::new(0xA), 4);
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 3,
                    area_h: 1
                },
                [0xAA, 0xAA, 0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 2,
                    area_w: 3,
                    area_h: 1
                },
                [0xAA, 0xAA, 0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, aligned
    fn test_pixel_rows_packed_multirow() {
        let area = Rectangle {
            top_left: Point { x: 4, y: 1 },
            size: Size {
                width: 4,
                height: 2,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 4,
                    area_y: 1,
                    area_w: 4,
                    area_h: 2
                },
                [0xAA, 0xAA, 0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, not aligned
    fn test_pixel_rows_multirow() {
        let area = Rectangle {
            top_left: Point { x: 3, y: 1 },
            size: Size {
                width: 3,
                height: 2,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 3,
                    area_h: 2
                },
                [0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, not aligned, top left pixels are out of drawable area
    fn test_pixel_non_drawable_top_left() {
        let area = Rectangle {
            top_left: Point { x: -1, y: -1 },
            size: Size {
                width: 3,
                height: 2,
            },
        };
        let area_s = AreaSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            Gray4::new(0xA),
            1024,
        );
        let mut s = AreaSerializerIterator::new(&area_s);
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 0,
                    area_y: 0,
                    area_w: 2,
                    area_h: 1
                },
                [0xAA, 0xAA].as_slice()
            ))
        );
        assert_eq!(s.next(), None);
    }
}
