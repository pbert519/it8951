use core::ops::BitXor;

use crate::{serialization_helper::get_entires_per_row, AreaImgInfo};
use alloc::vec::Vec;
use embedded_graphics_core::{
    pixelcolor::Gray4,
    prelude::*,
    primitives::{PointsIter, Rectangle},
    Pixel,
};

/// Converts a list of Pixels (pos, color) into frame buffer segements with area information.
pub struct PixelSerializer<I: Iterator<Item = Pixel<Gray4>>> {
    area: Rectangle,
    pixels: I,
    row: usize,
    max_entries: usize,
}

impl<I: Iterator<Item = Pixel<Gray4>>> PixelSerializer<I> {
    pub fn new(area: Rectangle, pixels: I, size: usize) -> Self {
        PixelSerializer {
            area,
            pixels,
            row: 0,
            // 1kByte
            max_entries: size,
        }
    }
}

impl<I: Iterator<Item = Pixel<Gray4>>> Iterator for PixelSerializer<I> {
    type Item = (AreaImgInfo, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.row >= self.area.size.height as usize {
            return None;
        }

        let start_row = self.row;

        // prepare buffer with enough capacity
        let entries_per_row = get_entires_per_row(self.area) as usize * 2; // convert length to bytes
        let max_rows = (self.max_entries / entries_per_row).min(self.area.size.height as usize);
        assert!(max_rows > 0, "Buffer size to small for one row");
        let mut bytes = vec![0x00; entries_per_row * max_rows];

        // add all pixels to buffer
        for Pixel(point, color) in self.pixels.by_ref() {
            // calculate the which u16 (pair of two bytes) the pixel is in
            let u16_pos = ((point.x - (self.area.top_left.x / 4 * 4)) / 2) as usize
                + entries_per_row * (self.row - start_row);

            // swap last pixel to map little endian behavior
            let byte_pos = u16_pos.bitxor(0x00001);

            // little endian layout
            // [P3, P2 | P1, P0]
            let bit_pos = (point.x % 2) * 4;

            bytes[byte_pos] |= (color.luma()) << bit_pos;

            //  end of row
            if point.x >= self.area.top_left.x + self.area.size.width as i32 - 1 {
                self.row += 1;
            }
            // abort if all rows are written to buffer
            if self.row >= max_rows + start_row {
                break;
            }
        }

        Some((
            AreaImgInfo {
                area_x: self.area.top_left.x as u16,
                area_y: (self.area.top_left.y + start_row as i32) as u16,
                area_w: self.area.size.width as u16,
                area_h: (self.row - start_row) as u16,
            },
            bytes,
        ))
    }
}

/// combines the color for each pixel with its position
/// the iterator filters all pixels, which are not drawable
pub fn convert_color_to_pixel_iterator<In: Iterator<Item = Gray4>>(
    area: Rectangle,
    bounding_box: Rectangle,
    colors: In,
) -> impl Iterator<Item = Pixel<Gray4>> {
    let drawable_area = area.intersection(&bounding_box);

    area.points()
        .zip(colors)
        .filter(move |(pos, _color)| drawable_area.contains(*pos))
        .map(|(pos, color)| Pixel(pos, color))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![Gray4::new(0xF)].into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 0,
                    area_y: 0,
                    area_w: 1,
                    area_h: 1
                },
                vec![0x00, 0x0F]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![Gray4::new(0x1)].into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 1,
                    area_y: 1,
                    area_w: 1,
                    area_h: 1
                },
                vec![0x00, 0x10]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![Gray4::new(0x4)].into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 2,
                    area_y: 1,
                    area_w: 1,
                    area_h: 1
                },
                vec![0x04, 0x00]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![Gray4::new(0xC)].into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 1,
                    area_h: 1
                },
                vec![0xC0, 0x00]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![
                    Gray4::new(0xA),
                    Gray4::new(0xB),
                    Gray4::new(0xC),
                    Gray4::new(0xD),
                ]
                .into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 4,
                    area_y: 1,
                    area_w: 4,
                    area_h: 1
                },
                vec![0xDC, 0xBA]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![Gray4::new(0xC), Gray4::new(0xD), Gray4::new(0xE)].into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 3,
                    area_h: 1
                },
                vec![0xC0, 0x00, 0x00, 0xED]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![
                    Gray4::new(0xA),
                    Gray4::new(0xB),
                    Gray4::new(0xC),
                    Gray4::new(0xD),
                    Gray4::new(0x1),
                    Gray4::new(0x2),
                    Gray4::new(0x3),
                    Gray4::new(0x4),
                ]
                .into_iter(),
            ),
            2,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 4,
                    area_y: 1,
                    area_w: 4,
                    area_h: 1
                },
                vec![0xDC, 0xBA]
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
                vec![0x43, 0x21]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![
                    Gray4::new(0xC),
                    Gray4::new(0xD),
                    Gray4::new(0xE),
                    Gray4::new(0x1),
                    Gray4::new(0x2),
                    Gray4::new(0x3),
                ]
                .into_iter(),
            ),
            4,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 3,
                    area_h: 1
                },
                vec![0xC0, 0x00, 0x00, 0xED]
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
                vec![0x10, 0x00, 0x00, 0x32]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![
                    Gray4::new(0xA),
                    Gray4::new(0xB),
                    Gray4::new(0xC),
                    Gray4::new(0xD),
                    Gray4::new(0x1),
                    Gray4::new(0x2),
                    Gray4::new(0x3),
                    Gray4::new(0x4),
                ]
                .into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 4,
                    area_y: 1,
                    area_w: 4,
                    area_h: 2
                },
                vec![0xDC, 0xBA, 0x43, 0x21]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![
                    Gray4::new(0xC),
                    Gray4::new(0xD),
                    Gray4::new(0xE),
                    Gray4::new(0x1),
                    Gray4::new(0x2),
                    Gray4::new(0x3),
                ]
                .into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 1,
                    area_w: 3,
                    area_h: 2
                },
                vec![0xC0, 0x00, 0x00, 0xED, 0x10, 0x00, 0x00, 0x32]
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
        let mut s = PixelSerializer::new(
            area.intersection(&BOUNDING_BOX_DEFAULT),
            convert_color_to_pixel_iterator(
                area,
                BOUNDING_BOX_DEFAULT,
                vec![
                    Gray4::new(0xC),
                    Gray4::new(0xD),
                    Gray4::new(0xE),
                    Gray4::new(0x1),
                    Gray4::new(0x2),
                    Gray4::new(0x3),
                ]
                .into_iter(),
            ),
            1024,
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 0,
                    area_y: 0,
                    area_w: 2,
                    area_h: 1
                },
                vec![0x00, 0x32]
            ))
        );
        assert_eq!(s.next(), None);
    }
}
