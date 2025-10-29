//! Trait to manage origin of the coordinate system

/// Origin for TopLeft corner
pub struct OriginTopLeft;
/// Origin for TopRight corner
pub struct OriginTopRight;

mod private {
    use core::ops::BitXor;
    use embedded_graphics_core::{prelude::Point, primitives::Rectangle};

    use crate::{AreaImgInfo, DevInfo};

    pub trait Sealed {
        /// Transforms the AreaImgInfo
        fn transform(area_img_info: &mut AreaImgInfo, dev_info: &DevInfo);

        /// Determines the right position in a area image buffer for a given pixel
        fn bit_and_byte_pos(
            area: &Rectangle,
            point: Point,
            u16_per_row: usize,
            row: usize,
            start_row: usize,
        ) -> (usize, i32);
    }

    impl Sealed for super::OriginTopLeft {
        #[inline(always)]
        fn transform(_: &mut AreaImgInfo, _: &DevInfo) {
            // NoOp
        }

        #[inline(always)]
        fn bit_and_byte_pos(
            area: &Rectangle,
            point: Point,
            u16_per_row: usize,
            row: usize,
            start_row: usize,
        ) -> (usize, i32) {
            let u16_pos = ((point.x - (area.top_left.x / 4 * 4)) / 2) as usize
                + u16_per_row * (row - start_row);

            // swap last pixel to map little endian behavior
            let byte_pos = u16_pos.bitxor(0x00001);

            // little endian layout
            // [P3, P2 | P1, P0]
            let bit_pos = (point.x % 2) * 4;

            (byte_pos, bit_pos)
        }
    }

    impl Sealed for super::OriginTopRight {
        #[inline(always)]
        fn transform(area_img_info: &mut AreaImgInfo, dev_info: &DevInfo) {
            area_img_info.area_x =
                dev_info.panel_width - area_img_info.area_x - area_img_info.area_w;
        }

        #[inline(always)]
        fn bit_and_byte_pos(
            area: &Rectangle,
            point: Point,
            u16_per_row: usize,
            row: usize,
            start_row: usize,
        ) -> (usize, i32) {
            let u16_pos =
                u16_per_row - 1 - ((point.x - (area.top_left.x / 4 * 4)) / 2) as usize
                    + u16_per_row * (row - start_row);

            // swap last pixel to map little endian behavior
            let byte_pos = u16_pos.bitxor(0x00001);

            // little endian layout
            // [P3, P2 | P1, P0]
            let bit_pos = ((point.x + 1) % 2) * 4;

            (byte_pos, bit_pos)
        }
    }
}

/// Origin trait to transforrm AreaImgInfo before sending to controller
pub trait Origin: private::Sealed {}

impl Origin for OriginTopLeft {}
impl Origin for OriginTopRight {}
