mod area_serializer;
mod pixel_serializer;
mod serialization_helper;

use embedded_graphics_core::{pixelcolor::Gray4, prelude::*, primitives::Rectangle};

use pixel_serializer::{convert_color_to_pixel_iterator, PixelSerializer};

use crate::{
    chunked_buffer::area_serializer::{AreaSerializer, AreaSerializerIterator},
    interface,
    memory_converter_settings::{self, MemoryConverterSetting},
    origin::Origin,
    AreaImgInfo, Error, Run, IT8951,
};

impl<IT8951Interface: interface::IT8951Interface, TOrigin: Origin> DrawTarget
    for IT8951<IT8951Interface, TOrigin, Run>
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

        let a = AreaSerializer::new(area, color, self.config.max_buffer_size);
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

        let pixel = PixelSerializer::<_, TOrigin>::new(
            area.intersection(&bb),
            iter,
            self.config.max_buffer_size,
        );

        for (mut area_img_info, buffer) in pixel {
            self.load_image_area(
                memory_address,
                MemoryConverterSetting {
                    endianness: memory_converter_settings::MemoryConverterEndianness::LittleEndian,
                    rotation: (&self.config.rotation).into(),
                    ..Default::default()
                },
                &mut area_img_info,
                &buffer,
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
