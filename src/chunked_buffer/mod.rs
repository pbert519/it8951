//! Chunked Buffer strategy for processing data

mod area_serializer;
mod pixel_serializer;
mod serialization_helper;

use core::{marker::PhantomData, ops::{Deref, DerefMut}};

use embedded_graphics_core::{pixelcolor::Gray4, prelude::*, primitives::Rectangle};

use pixel_serializer::{convert_color_to_pixel_iterator, PixelSerializer};

use crate::{
    chunked_buffer::{area_serializer::{AreaSerializer, AreaSerializerIterator}, pixel_serializer::PixelSerializerIterator},
    interface,
    memory_converter_settings::{self, MemoryConverterSetting},
    origin::Origin,
    AreaImgInfo, Buffer, Error, Run, IT8951,
};

trait ChunkedBuffer: Buffer {
    type BufferType: DerefMut<Target = [u8]>;

    fn max_size(&self) -> usize;
    fn buffer(&self, initial_value: u8, size: usize) -> Self::BufferType;
}

/// A (statically) allocated fixed length buffer
// pub struct FixedBuffer<'a, const N: usize> {
//     scratch_buffer: &'a [u8; N],
// }

// impl<'a, const N: usize> FixedBuffer<'a, N> {
//     /// Iinitializes a new fixed buffer
//     pub fn new(buffer: &'a [u8; N]) -> Self {
//         Self {
//             scratch_buffer: buffer,
//         }
//     }
// }

// impl<'a, const N: usize> Buffer for FixedBuffer<'a, N> {}

// impl<'a, const N: usize> ChunkedBuffer for FixedBuffer<'a, N> {
//     type BufferType = &'a [u8];

//     fn max_size(&self) -> usize {
//         N
//     }

//     fn buffer(&self, _: u8, size: usize) -> Self::BufferType {
//         &self.scratch_buffer[0..size]
//     }
// }

/// A (dynamically) allocated buffer
pub struct AllocBuffer<'a> {
    /// Max buffer size in bytes for staging buffers
    /// The buffer should be large enough to at least contain the pixels of a complete row
    /// The buffer must be aligned to u16
    /// The used IT8951 interface must support to write a complete buffer at once
    pub max_buffer_size: usize,
    phantom_data: PhantomData<&'a usize>,
}

impl<'a> AllocBuffer<'a> {
    /// Initialize a new buffer with a given max size
    pub fn new(max_buffer_size: usize) -> Self {
        Self {
            max_buffer_size,
            phantom_data: PhantomData {},
        }
    }
}

impl<'a> Buffer for AllocBuffer<'a> {}

impl<'a> ChunkedBuffer for AllocBuffer<'a> {
    type BufferType = alloc::vec::Vec<u8>;

    fn max_size(&self) -> usize {
        self.max_buffer_size
    }

    fn buffer(&self, initial_value: u8, size: usize) -> Self::BufferType {
        vec![initial_value; size]
    }
}

impl<IT8951Interface: interface::IT8951Interface, TOrigin: Origin, BufferStrategy: Buffer>
    DrawTarget for IT8951<IT8951Interface, TOrigin, BufferStrategy, Run>
where
    BufferStrategy: ChunkedBuffer,
{
    type Color = Gray4;

    type Error = Error;

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let size = self.size();

        self.fill_solid(
            &Rectangle::new(
                Point::zero(),
                Size {
                    width: size.width,
                    height: size.height,
                },
            ),
            color,
        )
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // only update visible content
        let area = area.intersection(&self.bounding_box());
        // if the area is zero sized, skip drawing
        if area.is_zero_sized() {
            return Ok(());
        }

        let a = AreaSerializer::new(area, color, &mut self.buffer);
        let area_iter = AreaSerializerIterator::new(&a);
        let memory_address = self
            .dev_info
            .as_ref()
            .map(|d| d.memory_address)
            .expect("Dev info not initialized");

        for (mut area_img_info, buffer) in area_iter {
            self.load_image_area(
                memory_address,
                MemoryConverterSetting {
                    rotation: (&self.config.rotation).into(),
                    ..Default::default()
                },
                &mut area_img_info,
                buffer,
            )?;
        }

        #[cfg(feature = "defmt")]
        defmt::trace!("Embedded graphics: Fill solid");

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let bb = self.bounding_box();
        let iter = convert_color_to_pixel_iterator(area, &bb, colors.into_iter());
        let memory_address = self
            .dev_info
            .as_ref()
            .map(|d| d.memory_address)
            .expect("Dev info not initialized");

        let mut pixel = PixelSerializer::<TOrigin, _>::new(
            area.intersection(&bb),
            &mut self.buffer,
        );
        let pixel_iterator = PixelSerializerIterator::<_, TOrigin, _>::new(
            &mut pixel,
            iter
        );

        for mut area_img_info in pixel_iterator {
            self.load_image_area(
                memory_address,
                MemoryConverterSetting {
                    endianness: memory_converter_settings::MemoryConverterEndianness::LittleEndian,
                    rotation: (&self.config.rotation).into(),
                    ..Default::default()
                },
                &mut area_img_info,
                &pixel.buffer,
            )?;
        }

        #[cfg(feature = "defmt")]
        defmt::trace!("Embedded graphics: Fill contiguous");

        Ok(())
    }

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics_core::Pixel<Self::Color>>,
    {
        let memory_address = self
            .dev_info
            .as_ref()
            .map(|d| d.memory_address)
            .expect("Dev info not initialized");
        let size = self.size();
        let width = size.width as i32;
        let height = size.height as i32;
        for Pixel(coord, color) in pixels.into_iter() {
            if (coord.x >= 0 && coord.x < width) || (coord.y >= 0 || coord.y < height) {
                let raw_color = color.luma();
                let data = [raw_color << 4 | raw_color, raw_color << 4 | raw_color];

                self.load_image_area(
                    memory_address,
                    MemoryConverterSetting {
                        rotation: (&self.config.rotation).into(),
                        ..Default::default()
                    },
                    &mut AreaImgInfo {
                        area_x: coord.x as u16,
                        area_y: coord.y as u16,
                        area_w: 1,
                        area_h: 1,
                    },
                    &data,
                )?;
            }
        }

        #[cfg(feature = "defmt")]
        defmt::trace!("Embedded graphics: Draw iter");

        Ok(())
    }
}
