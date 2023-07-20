use crate::AreaImgInfo;
use alloc::vec::Vec;
use embedded_graphics::{
    pixelcolor::Gray4,
    prelude::*,
    primitives::{PointsIter, Rectangle},
    Pixel,
};

pub struct PixelSerializer<I: Iterator<Item = Pixel<Gray4>>> {
    area: Rectangle,
    pixels: I,
    row: i32,
}

impl<I: Iterator<Item = Pixel<Gray4>>> PixelSerializer<I> {
    pub fn new(area: Rectangle, pixels: I) -> Self {
        PixelSerializer {
            area,
            pixels,
            row: 0,
        }
    }
}

impl<I: Iterator<Item = Pixel<Gray4>>> Iterator for PixelSerializer<I> {
    type Item = (AreaImgInfo, Vec<u16>);

    fn next(&mut self) -> Option<Self::Item> {
        self.row += 1;

        let mut pixel_counter = 0;

        // prepare buffer with enough capacity
        let entries_per_row = (self.area.size.width + (self.area.top_left.x % 4) as u32 + 3) / 4;
        let mut bytes = Vec::with_capacity(entries_per_row as usize);

        // add all pixels to buffer
        for Pixel(point, color) in self.pixels.by_ref() {
            let byte_pos = ((point.x - (self.area.top_left.x / 4 * 4)) / 4) as usize;
            let shift = (point.x % 4) * 4;

            if bytes.len() <= byte_pos {
                bytes.push(0x0000);
            }
            bytes[byte_pos] |= (color.luma() as u16) << shift;

            pixel_counter += 1;
            // abort condition end of row
            if point.x >= self.area.top_left.x + self.area.size.width as i32 - 1 {
                break;
            }
        }
        if bytes.is_empty() {
            None
        } else {
            Some((
                AreaImgInfo {
                    area_x: self.area.top_left.x as u16,
                    area_y: (self.area.top_left.y + self.row - 1) as u16,
                    area_w: pixel_counter,
                    area_h: 1,
                },
                bytes,
            ))
        }
    }
}

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
                vec![0x000F]
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
                vec![0x0010]
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
                vec![0x0400]
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
                vec![0xC000]
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
                vec![0xDCBA]
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
                vec![0xC000, 0x00ED]
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
                vec![0xDCBA]
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
                vec![0x4321]
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
                vec![0xC000, 0x00ED]
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
                vec![0x1000, 0x0032]
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, not aligned, color iterator has less pixel than area
    fn test_pixel_rows_early_exit() {
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
                ]
                .into_iter(),
            ),
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
                vec![0xC000, 0x00ED]
            ))
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 2,
                    area_w: 2,
                    area_h: 1
                },
                vec![0x1000, 0x0002]
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, not aligned, color iterator has less pixel than area -> a full u16 is missing
    fn test_pixel_rows_early_exit_full_byte() {
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
                ]
                .into_iter(),
            ),
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
                vec![0xC000, 0x00ED]
            ))
        );
        assert_eq!(
            s.next(),
            Some((
                AreaImgInfo {
                    area_x: 3,
                    area_y: 2,
                    area_w: 1,
                    area_h: 1
                },
                vec![0x1000]
            ))
        );
        assert_eq!(s.next(), None);
    }

    #[test]
    // two rows of pixels, not aligned, color iterator has less pixel than area -> a full row is missing
    fn test_pixel_rows_early_exit_full_row() {
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
                vec![Gray4::new(0xC), Gray4::new(0xD), Gray4::new(0xE)].into_iter(),
            ),
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
                vec![0xC000, 0x00ED]
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
                vec![0x0032]
            ))
        );
        assert_eq!(s.next(), None);
    }
}
