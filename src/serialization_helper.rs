use embedded_graphics_core::primitives::Rectangle;

/// Calculates how many u16 values are necessary per line on the display.
/// This includes the correct alignment
pub fn get_u16_per_row(area: Rectangle, bits_per_pixel: u32) -> u32 {
    let pixel_per_word: u32 = 16 / bits_per_pixel as u32;

    let alignment_pixels = area.top_left.x as u32 % pixel_per_word;

    (area.size.width + alignment_pixels).div_ceil(pixel_per_word)
}

#[cfg(test)]
mod tests {
    use embedded_graphics_core::geometry::{Point, Size};

    use super::*;

    macro_rules! get_u16_per_row_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (offset, width, expected, bpp) = $value;
                assert_eq!(expected, get_u16_per_row(Rectangle::new(Point::new(offset, 0), Size::new(width, 1)), bpp));
            }
        )*
        }
    }

    get_u16_per_row_tests! {
        aligned_2bpp_0: (0, 0, 0, 2),
        aligned_2bpp_1: (0, 1, 1, 2),
        aligned_2bpp_2: (0, 2, 1, 2),
        aligned_2bpp_3: (0, 3, 1, 2),
        aligned_2bpp_4: (0, 4, 1, 2),
        aligned_2bpp_5: (0, 5, 1, 2),
        aligned_2bpp_6: (0, 6, 1, 2),
        aligned_2bpp_7: (0, 7, 1, 2),
        aligned_2bpp_8: (0, 8, 1, 2),
        aligned_2bpp_9: (0, 9, 2, 2),
        off_by_one_2bpp_1: (1, 1, 1, 2),
        off_by_one_2bpp_2: (1, 2, 1, 2),
        off_by_one_2bpp_3: (1, 3, 1, 2),
        off_by_one_2bpp_4: (1, 4, 1, 2),
        off_by_one_2bpp_8: (1, 8, 2, 2),
        off_by_one_2bpp_9: (1, 9, 2, 2),
        aligned_0: (0, 0, 0, 4),
        aligned_1: (0, 1, 1, 4),
        aligned_2: (0, 2, 1, 4),
        aligned_3: (0, 3, 1, 4),
        aligned_4: (0, 4, 1, 4),
        aligned_5: (0, 5, 2, 4),
        aligned_7: (0, 7, 2, 4),
        aligned_8: (0, 8, 2, 4),
        aligned_9: (0, 9, 3, 4),
        off_by_one_1: (1, 1, 1, 4),
        off_by_one_2: (1, 2, 1, 4),
        off_by_one_3: (1, 3, 1, 4),
        off_by_one_4: (1, 4, 2, 4),
        off_by_one_5: (1, 5, 2, 4),
        off_by_two_1: (2, 1, 1, 4),
        off_by_two_2: (2, 2, 1, 4),
        off_by_two_3: (2, 3, 2, 4),
        off_by_two_4: (2, 4, 2, 4),
        off_by_two_5: (2, 5, 2, 4),
        off_by_three_1: (3, 1, 1, 4),
        off_by_three_2: (3, 2, 2, 4),
        off_by_three_3: (3, 3, 2, 4),
        off_by_three_4: (3, 4, 2, 4),
        off_by_three_5: (3, 5, 2, 4),
        off_by_three_6: (3, 6, 3, 4),
        off_by_four_6: (4, 1, 1, 4),
    }
}
