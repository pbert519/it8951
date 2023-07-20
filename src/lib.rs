#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate alloc;
use alloc::string::String;

mod command;
pub mod interface;
pub mod memory_converter_settings;
mod register;

use crate::memory_converter_settings::MemoryConverterSetting;
use core::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Interface(interface::Error),
    NotInitalized,
}
impl From<interface::Error> for Error {
    fn from(e: interface::Error) -> Self {
        Error::Interface(e)
    }
}

#[derive(Debug, Clone)]
pub struct DevInfo {
    pub panel_width: u16,
    pub panel_height: u16,
    pub memory_address: u32,
    pub firmware_version: String,
    pub lut_version: String,
}

pub struct AreaImgInfo {
    pub area_x: u16,
    pub area_y: u16,
    pub area_w: u16,
    pub area_h: u16,
}

/// See https://www.waveshare.com/w/upload/c/c4/E-paper-mode-declaration.pdf for full description
pub enum WaveformMode {
    Init = 0, // used for full erase to white, flashy, should be used if framebuffer is not up to date
    DirectUpdate = 1, // any graytone to black or white, non flashy
    GrayscaleClearing16 = 2, // high image quality, all graytones
    GL16 = 3, // sparse content on white, eg. text
    GLR16 = 4, // only in combination with with propertary image preprocessing
    GLD16 = 5, // only in combination with with propertary image preprocessing
    A2 = 6,   // fast, non-flash, from black/white to black/white only
    DU4 = 7,  // fast, non flash, from any graytone to 1,6,11,16
}

pub struct IT8951<IT8951Interface> {
    interface: IT8951Interface,
    dev_info: Option<DevInfo>,
}

impl<IT8951Interface> IT8951<IT8951Interface>
where
    IT8951Interface: interface::IT8951Interface,
{
    pub fn new(interface: IT8951Interface) -> IT8951<IT8951Interface> {
        IT8951 {
            interface,
            dev_info: None,
        }
    }

    pub fn init(&mut self, vcom: u16) -> Result<(), Error> {
        self.interface.reset()?;
        self.sys_run()?;
 
        let dev_info = self.get_system_info()?;

        // Enable Pack Write
        self.write_register(register::I80CPCR, 0x0001)?;

        if vcom != self.get_vcom()? {
            self.set_vcom(vcom)?;
        }

        self.dev_info = Some(dev_info);

        Ok(())
    }

    pub fn get_dev_info(&self) -> Result<DevInfo, Error> {
        match &self.dev_info {
            Some(dev_info) => Ok(dev_info.clone()),
            None => Err(Error::NotInitalized),
        }
    }

    pub fn enhance_driving_capability(&mut self) -> Result<(), Error> {
        self.write_register(0x0038, 0x0602)?;
        Ok(())
    }

    // load image functions ------------------------------------------------------------------------------------------

    pub fn load_image(
        &mut self,
        target_mem_addr: u32,
        image_settings: MemoryConverterSetting,
        data: &[u16],
    ) -> Result<(), Error> {
        self.set_target_memory_addr(target_mem_addr)?;

        self.interface.write_command(command::IT8951_TCON_LD_IMG)?;
        self.interface.write_data(image_settings.into_arg())?;

        self.interface.write_multi_data(data)?;

        self.interface
            .write_command(command::IT8951_TCON_LD_IMG_END)?;
        Ok(())
    }

    pub fn load_image_area(
        &mut self,
        target_mem_addr: u32,
        image_settings: MemoryConverterSetting,
        area_info: &AreaImgInfo,
        data: &[u16],
    ) -> Result<(), Error> {
        self.set_target_memory_addr(target_mem_addr)?;

        self.interface.write_command_with_args(
            command::IT8951_TCON_LD_IMG_AREA,
            &[
                image_settings.into_arg(),
                area_info.area_x,
                area_info.area_y,
                area_info.area_w,
                area_info.area_h,
            ],
        )?;

        self.interface.write_multi_data(data)?;

        self.interface
            .write_command(command::IT8951_TCON_LD_IMG_END)?;

        Ok(())
    }

    fn set_target_memory_addr(&mut self, target_mem_addr: u32) -> Result<(), Error> {
        self.write_register(register::LISAR + 2, (target_mem_addr >> 16) as u16)?;
        self.write_register(register::LISAR, target_mem_addr as u16)?;
        Ok(())
    }

    // buffer functions -------------------------------------------------------------------------------------------------

    pub fn memory_burst_read(
        &mut self,
        memory_address: u32,
        data: &mut [u16],
    ) -> Result<(), Error> {
        let args = [
            memory_address as u16,
            (memory_address >> 16) as u16,
            data.len() as u16,
            (data.len() >> 16) as u16,
        ];
        self.interface
            .write_command_with_args(command::IT8951_TCON_MEM_BST_RD_T, &args)?;
        self.interface
            .write_command(command::IT8951_TCON_MEM_BST_RD_S)?;

        self.interface.read_multi_data(data)?;

        self.interface
            .write_command(command::IT8951_TCON_MEM_BST_END)?;

        Ok(())
    }

    pub fn memory_burst_write(&mut self, memory_address: u32, data: &[u16]) -> Result<(), Error> {
        let args = [
            memory_address as u16,
            (memory_address >> 16) as u16,
            data.len() as u16,
            (data.len() >> 16) as u16,
        ];
        self.interface
            .write_command_with_args(command::IT8951_TCON_MEM_BST_WR, &args)?;

        self.interface.write_multi_data(data)?;

        self.interface
            .write_command(command::IT8951_TCON_MEM_BST_END)?;
        Ok(())
    }

    // display functions ------------------------------------------------------------------------------------------------
    pub fn display_area(
        &mut self,
        area_info: &AreaImgInfo,
        mode: WaveformMode,
    ) -> Result<(), Error> {
        self.wait_for_display_ready()?;

        let args = [
            area_info.area_x,
            area_info.area_y,
            area_info.area_w,
            area_info.area_h,
            mode as u16,
        ];

        self.interface
            .write_command_with_args(command::USDEF_I80_CMD_DPY_AREA, &args)?;
        Ok(())
    }

    pub fn display_area_buf(
        &mut self,
        area_info: &AreaImgInfo,
        mode: WaveformMode,
        target_mem_addr: u32,
    ) -> Result<(), Error> {
        self.wait_for_display_ready()?;

        let args = [
            area_info.area_x,
            area_info.area_y,
            area_info.area_w,
            area_info.area_h,
            mode as u16,
            target_mem_addr as u16,
            (target_mem_addr >> 16) as u16,
        ];

        self.interface
            .write_command_with_args(command::USDEF_I80_CMD_DPY_BUF_AREA, &args)?;
        Ok(())
    }

    // misc  ------------------------------------------------------------------------------------------------

    fn wait_for_display_ready(&mut self) -> Result<(), Error> {
        while Ok(0) != self.read_register(register::LUTAFSR) {}
        Ok(())
    }

    fn sys_run(&mut self) -> Result<(), Error> {
        self.interface.write_command(command::IT8951_TCON_SYS_RUN)?;
        Ok(())
    }

    pub fn sleep(&mut self) -> Result<(), Error> {
        self.interface.write_command(command::IT8951_TCON_SLEEP)?;
        Ok(())
    }

    pub fn standby(&mut self) -> Result<(), Error> {
        self.interface.write_command(command::IT8951_TCON_STANDBY)?;
        Ok(())
    }

    fn get_system_info(&mut self) -> Result<DevInfo, Error> {
        self.interface
            .write_command(command::USDEF_I80_CMD_GET_DEV_INFO)?;

        self.interface.wait_while_busy()?;

        // 40 bytes payload
        let mut buf = [0x0000; 20];
        self.interface.read_multi_data(&mut buf)?;

        Ok(DevInfo {
            panel_width: buf[0],
            panel_height: buf[1],
            memory_address: ((buf[3] as u32) << 16) | (buf[2] as u32),
            firmware_version: self.buf_to_string(&buf[4..12]),
            lut_version: self.buf_to_string(&buf[12..20]),
        })
    }

    fn buf_to_string(&self, buf: &[u16]) -> String {
        buf.iter()
            .filter(|&&raw| raw != 0x0000)
            .fold(String::new(), |mut res, &raw| {
                if let Some(c) = char::from_u32((raw & 0xFF) as u32) {
                    res.push(c);
                }
                if let Some(c) = char::from_u32((raw >> 8) as u32) {
                    res.push(c);
                }
                res
            })
    }

    fn get_vcom(&mut self) -> Result<u16, Error> {
        self.interface.write_command(command::USDEF_I80_CMD_VCOM)?;
        self.interface.write_data(0x0000)?;
        let vcom = self.interface.read_data()?;
        Ok(vcom)
    }

    fn set_vcom(&mut self, vcom: u16) -> Result<(), Error> {
        self.interface.write_command(command::USDEF_I80_CMD_VCOM)?;
        self.interface.write_data(0x0001)?;
        self.interface.write_data(vcom)?;
        Ok(())
    }

    fn read_register(&mut self, reg: u16) -> Result<u16, Error> {
        self.interface.write_command(command::IT8951_TCON_REG_RD)?;
        self.interface.write_data(reg)?;
        let data = self.interface.read_data()?;
        Ok(data)
    }

    fn write_register(&mut self, reg: u16, data: u16) -> Result<(), Error> {
        self.interface.write_command(command::IT8951_TCON_REG_WR)?;
        self.interface.write_data(reg)?;
        self.interface.write_data(data)?;
        Ok(())
    }
}

// --------------------------- embedded graphics support --------------------------------------

use embedded_graphics::{pixelcolor::Gray4, prelude::*};

impl<IT8951Interface> DrawTarget for IT8951<IT8951Interface>
where
    IT8951Interface: interface::IT8951Interface,
{
    type Color = Gray4;

    type Error = Error;

    // fetch area from it8951 frame buffer,
    // overwrites given pixels
    // transmit area
    // How to areas to big for the internal memory? Split into areas? How to handle "wrong" order of pixels?
    // full refresh
    fn fill_contiguous<I>(
        &mut self,
        _area: &embedded_graphics::primitives::Rectangle,
        _colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        todo!()
    }

    // create area locally and send it to the devices
    // split into multiple buffers if to big for ram
    // raspi spi buffer size is 4096kb
    // refresh full frame
    fn fill_solid(
        &mut self,
        _area: &embedded_graphics::primitives::Rectangle,
        _color: Self::Color,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let dev_info = self.get_dev_info()?;
        let width = dev_info.panel_width;
        let height = dev_info.panel_height;
        let mem_addr = dev_info.memory_address;
        let pixel_data_u8: u8 = color.luma() | color.luma() << 4;
        let pixel_data_u16 = (pixel_data_u8 as u16) << 8 | pixel_data_u8 as u16;

        // we need to split the data in multiple transfers to keep the buffer size small
        for w in 0..height {
            self.load_image_area(
                mem_addr,
                MemoryConverterSetting {
                    endianness: memory_converter_settings::MemoryConverterEndianness::LittleEndian,
                    bit_per_pixel:
                        memory_converter_settings::MemoryConverterBitPerPixel::BitsPerPixel4,
                    rotation: memory_converter_settings::MemoryConverterRotation::Rotate0,
                },
                &AreaImgInfo {
                    area_x: 0,
                    area_y: w,
                    area_w: width,
                    area_h: 1,
                },
                &vec![pixel_data_u16; width as usize / 4],
            )?;
        }

        self.display_area(
            &AreaImgInfo {
                area_x: 0,
                area_y: 0,
                area_w: width,
                area_h: height,
            },
            WaveformMode::Init,
        )?;
        Ok(())
    }

    // it is possible to set only on pixel by using load image area with a area size of 1
    // however we still have to send 2 bytes, the other 3 pixels are ignored
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let dev_info = self.get_dev_info()?;
        let width = dev_info.panel_width as i32;
        let height = dev_info.panel_height as i32;
        for Pixel(coord, _color) in pixels.into_iter() {
            if (coord.x >= 0 && coord.x < width) || (coord.y >= 0 || coord.y < height) {
                todo!("write pixel")
            }
        }
        Ok(())
    }
}

impl<IT8951Interface> OriginDimensions for IT8951<IT8951Interface>
where
    IT8951Interface: interface::IT8951Interface,
{
    fn size(&self) -> Size {
        let dev_info = self.dev_info.as_ref().unwrap();
        Size::new(dev_info.panel_width as u32, dev_info.panel_height as u32)
    }
}
