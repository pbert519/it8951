#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

//! IT8951 epaper driver for the waveshare 7.8in display
//! The implementation is based on the IT8951 I80/SPI/I2C programming guide
//! provided by waveshare: https://www.waveshare.com/wiki/7.8inch_e-Paper_HAT

#[macro_use]
extern crate alloc;
use alloc::string::String;

mod command;
pub mod interface;
pub mod memory_converter_settings;
mod register;

use crate::memory_converter_settings::MemoryConverterSetting;
use core::fmt::Debug;

/// Controller Error
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// controller interface error
    Interface(interface::Error),
    /// driver not initalized error
    NotInitalized,
}
impl From<interface::Error> for Error {
    fn from(e: interface::Error) -> Self {
        Error::Interface(e)
    }
}

/// Device Info Struct
/// Describes the connected display
#[derive(Debug, Clone)]
pub struct DevInfo {
    /// width in pixel of the connected panel
    pub panel_width: u16,
    /// height in pixel of the connected panel
    pub panel_height: u16,
    /// start address of the frame buffer in the controller ram
    pub memory_address: u32,
    /// Controller firmware version
    pub firmware_version: String,
    /// LUT version
    /// The lut describes the waveforms to modify the display content
    /// LUT is specific for every display
    pub lut_version: String,
}

/// Describes a area on the display
pub struct AreaImgInfo {
    /// x position (left to right, 0 is top left corner)
    pub area_x: u16,
    /// y position (top to bottom, 0 is top left corner)
    pub area_y: u16,
    /// width (x-axis)
    pub area_w: u16,
    /// height (y-axis)
    pub area_h: u16,
}

/// See https://www.waveshare.com/w/upload/c/c4/E-paper-mode-declaration.pdf for full description
pub enum WaveformMode {
    /// used for full erase to white, flashy, should be used if framebuffer is not up to date
    Init = 0,
    /// any graytone to black or white, non flashy
    DirectUpdate = 1,
    /// high image quality, all graytones
    GrayscaleClearing16 = 2,
    ///  sparse content on white, eg. text
    GL16 = 3,
    ///  only in combination with with propertary image preprocessing
    GLR16 = 4,
    /// only in combination with with propertary image preprocessing
    GLD16 = 5,
    /// fast, non-flash, from black/white to black/white only
    A2 = 6,
    /// fast, non flash, from any graytone to 1,6,11,16
    DU4 = 7,
}

/// IT8951 e paper driver
/// The controller supports multiple interfaces
pub struct IT8951<IT8951Interface> {
    interface: IT8951Interface,
    dev_info: Option<DevInfo>,
}

impl<IT8951Interface> IT8951<IT8951Interface>
where
    IT8951Interface: interface::IT8951Interface,
{
    /// Creates a new controller driver object
    /// Call init afterwards to initalize the controller
    pub fn new(interface: IT8951Interface) -> IT8951<IT8951Interface> {
        IT8951 {
            interface,
            dev_info: None,
        }
    }

    /// Initalize the driver and resets the display
    /// VCOM should be given on your display
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

        self.reset()?;

        Ok(())
    }

    /// Get the Device information
    pub fn get_dev_info(&self) -> Result<DevInfo, Error> {
        match &self.dev_info {
            Some(dev_info) => Ok(dev_info.clone()),
            None => Err(Error::NotInitalized),
        }
    }

    /// Increases the driver strength
    /// Use only if the image is not clear!
    pub fn enhance_driving_capability(&mut self) -> Result<(), Error> {
        self.write_register(0x0038, 0x0602)?;
        Ok(())
    }

    /// initalize the frame buffer and clear the display to white
    pub fn reset(&mut self) -> Result<(), Error> {
        let dev_info = self.get_dev_info()?;
        let width = dev_info.panel_width;
        let height = dev_info.panel_height;
        let mem_addr = dev_info.memory_address;

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
                &vec![0xFFFF; width as usize / 4],
            )?;
        }

        self.display(WaveformMode::Init)?;
        Ok(())
    }

    // load image functions ------------------------------------------------------------------------------------------

    /// Loads a full frame into the controller frame buffer using the pixel preprocessor
    /// Warning: For the most usecases, the underlying spi transfer ist not capable to transfer a complete frame
    /// split the frame into multiple areas and use load_image_area instead
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

    /// Loads pixel data into the controller frame buffer using the pixel preprocessor
    /// Memory Address should be read from the dev_info struct
    /// ImageSettings define the layout of the data buffer
    /// AreaInfo describes the frame buffer area which should be updated
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

    /// Reads the given memory address from the controller ram into data
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

    /// Writes a buffer of u16 values to the given memory address in the controller ram
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

    /// Refresh a specific area of the display with the frame buffer content
    /// A usecase specific wafeform must be selected by the user
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

    /// Refresh a specific area of the display from a dedicated frame buffer
    /// A usecase specific wafeform must be selected by the user
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

    /// Refresh the full E-Ink display with the frame buffer content
    /// A usecase specific wafeform must be selected by the user
    pub fn display(&mut self, mode: WaveformMode) -> Result<(), Error> {
        let dev_info = self.get_dev_info()?;
        let width = dev_info.panel_width;
        let height = dev_info.panel_height;

        self.display_area(
            &AreaImgInfo {
                area_x: 0,
                area_y: 0,
                area_w: width,
                area_h: height,
            },
            mode,
        )?;
        Ok(())
    }

    // misc  ------------------------------------------------------------------------------------------------

    fn wait_for_display_ready(&mut self) -> Result<(), Error> {
        while Ok(0) != self.read_register(register::LUTAFSR) {}
        Ok(())
    }

    /// Activate active power mode
    /// This is the normal operation power mode
    pub fn sys_run(&mut self) -> Result<(), Error> {
        self.interface.write_command(command::IT8951_TCON_SYS_RUN)?;
        Ok(())
    }

    /// Activate sleep power mode
    /// All clocks, pll, osc and the panel are off, but the ram is refreshed
    pub fn sleep(&mut self) -> Result<(), Error> {
        self.interface.write_command(command::IT8951_TCON_SLEEP)?;
        Ok(())
    }

    /// Activate standby power mode
    /// Clocks are gated off, but pll, osc, panel power and ram is active
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

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let dev_info = self.get_dev_info()?;
        let width = dev_info.panel_width as i32;
        let height = dev_info.panel_height as i32;
        for Pixel(coord, color) in pixels.into_iter() {
            if (coord.x >= 0 && coord.x < width) || (coord.y >= 0 || coord.y < height) {
                let data: u16 = (color.luma() as u16) << ((coord.x % 4) * 4);

                self.load_image_area(
                    dev_info.memory_address,
                    MemoryConverterSetting {
                        endianness:
                            memory_converter_settings::MemoryConverterEndianness::LittleEndian,
                        bit_per_pixel:
                            memory_converter_settings::MemoryConverterBitPerPixel::BitsPerPixel4,
                        rotation: memory_converter_settings::MemoryConverterRotation::Rotate0,
                    },
                    &AreaImgInfo {
                        area_x: coord.x as u16,
                        area_y: coord.y as u16,
                        area_w: 1,
                        area_h: 1,
                    },
                    &[data],
                )?;
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
