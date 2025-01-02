#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

//! IT8951 epaper driver for the waveshare 7.8in display
//! The implementation is based on the IT8951 I80/SPI/I2C programming guide
//! provided by waveshare: https://www.waveshare.com/wiki/7.8inch_e-Paper_HAT

#[macro_use]
extern crate alloc;
use core::{borrow::Borrow, marker::PhantomData};

use alloc::string::String;

mod area_serializer;
mod command;
pub mod interface;
pub mod memory_converter_settings;
mod pixel_serializer;
mod register;
mod serialization_helper;

use area_serializer::{AreaSerializer, AreaSerializerIterator};
use memory_converter_settings::MemoryConverterSetting;
use pixel_serializer::{convert_color_to_pixel_iterator, PixelSerializer};

/// Controller Error
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// controller interface error
    Interface(interface::Error),
    /// Timeout
    DisplayEngineTimeout,
}
impl From<interface::Error> for Error {
    fn from(e: interface::Error) -> Self {
        Error::Interface(e)
    }
}

/// Driver configuration
pub struct Config {
    /// Timeout for the internal display engine
    pub timeout_display_engine: core::time::Duration,
    /// Timeout for the busy pin
    pub timeout_interface: core::time::Duration,
    /// Max buffer size in bytes for staging buffers
    /// The buffer should be large enough to at least contain the pixels of a complete row
    /// The buffer must be aligned to u16
    /// The used IT8951 interface must support to write a complete buffer at once
    pub max_buffer_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout_display_engine: core::time::Duration::from_secs(15),
            timeout_interface: core::time::Duration::from_secs(15),
            max_buffer_size: 1024,
        }
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
#[derive(Debug, PartialEq, Eq)]
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

/// Normal Operation
pub struct Run;
/// The device is either in sleep or standby mode:
/// Sleep: All clocks, pll, osc and the panel are off, but the ram is refreshed
/// Standby: Clocks are gated off, but pll, osc, panel power and ram is active
pub struct PowerDown;
/// Not initalised driver after a power cycle
pub struct Off;

/// IT8951 e paper driver
/// The controller supports multiple interfaces
pub struct IT8951<IT8951Interface, State> {
    interface: IT8951Interface,
    dev_info: Option<DevInfo>,
    marker: core::marker::PhantomData<State>,
    config: Config,
    memory_converter_settings: MemoryConverterSetting,
}

impl<IT8951Interface: interface::IT8951Interface, TState> IT8951<IT8951Interface, TState> {
    fn into_state<TNew>(self) -> IT8951<IT8951Interface, TNew> {
        IT8951::<IT8951Interface, TNew> {
            interface: self.interface,
            dev_info: self.dev_info,
            marker: PhantomData {},
            config: self.config,
            memory_converter_settings: self.memory_converter_settings,
        }
    }
}

impl<IT8951Interface: interface::IT8951Interface> IT8951<IT8951Interface, Off> {
    /// Creates a new controller driver object
    /// Call init afterwards to initalize the controller
    pub fn new(interface: IT8951Interface, config: Config) -> Self {
        Self::new_with_mcs(interface, config, MemoryConverterSetting::default())
    }

    /// Creates a new controller driver object
    /// Call init afterwards to initalize the controller
    /// Allows to set custom MemoryConverterSetting to specify rotation
    pub fn new_with_mcs(
        mut interface: IT8951Interface,
        config: Config,
        mcs: MemoryConverterSetting,
    ) -> Self {
        interface.set_busy_timeout(config.timeout_interface);
        IT8951 {
            interface,
            dev_info: None,
            marker: PhantomData {},
            config,
            memory_converter_settings: mcs,
        }
    }

    /// Initalize the driver and resets the display
    /// VCOM should be given on your display
    /// Since version 0.4.0, this function no longer resets the display
    pub fn init(mut self, vcom: u16) -> Result<IT8951<IT8951Interface, Run>, Error> {
        self.interface.reset()?;

        let mut it8951 = self.into_state::<PowerDown>().sys_run()?;

        let dev_info = it8951.get_system_info()?;

        // Enable Pack Write
        it8951.write_register(register::I80CPCR, 0x0001)?;

        if vcom != it8951.get_vcom()? {
            it8951.set_vcom(vcom)?;
        }

        it8951.dev_info = Some(dev_info);

        Ok(it8951)
    }

    /// Create a new Driver for are already active and initalized driver
    /// This can be usefull if the device was still powered on, but the uC restarts.
    pub fn attach(
        mut interface: IT8951Interface,
        config: Config,
    ) -> Result<IT8951<IT8951Interface, Run>, Error> {
        interface.set_busy_timeout(config.timeout_interface);

        let mut it8951 = IT8951 {
            interface,
            dev_info: None,
            marker: PhantomData {},
            config,
            memory_converter_settings: MemoryConverterSetting::default(),
        }
        .sys_run()?;

        it8951.dev_info = Some(it8951.get_system_info()?);

        Ok(it8951)
    }
}

impl<IT8951Interface: interface::IT8951Interface> IT8951<IT8951Interface, Run> {
    /// Get the Device information
    pub fn get_dev_info(&self) -> DevInfo {
        self.dev_info.clone().unwrap()
    }

    /// Increases the driver strength
    /// Use only if the image is not clear!
    pub fn enhance_driving_capability(&mut self) -> Result<(), Error> {
        self.write_register(0x0038, 0x0602)?;
        Ok(())
    }

    /// initalize the frame buffer and clear the display to white
    pub fn reset(&mut self) -> Result<(), Error> {
        self.clear(Gray4::WHITE)?;
        self.display(WaveformMode::Init)?;
        Ok(())
    }

    // load image functions ------------------------------------------------------------------------------------------

    /// Loads a full frame into the controller frame buffer using the pixel preprocessor
    /// Warning: For the most usecases, the underlying spi transfer ist not capable to transfer a complete frame
    /// split the frame into multiple areas and use load_image_area instead
    /// Data must be aligned to u16!
    pub fn load_image<TMemoryConverterSetting: Borrow<MemoryConverterSetting>>(
        &mut self,
        target_mem_addr: u32,
        image_settings: TMemoryConverterSetting,
        data: &[u8],
    ) -> Result<(), Error> {
        self.set_target_memory_addr(target_mem_addr)?;

        self.interface.write_command(command::IT8951_TCON_LD_IMG)?;
        self.interface
            .write_data(image_settings.borrow().into_arg())?;

        self.interface.write_multi_data(data)?;

        self.interface
            .write_command(command::IT8951_TCON_LD_IMG_END)?;
        Ok(())
    }

    /// Loads pixel data into the controller frame buffer using the pixel preprocessor
    /// Memory Address should be read from the dev_info struct
    /// ImageSettings define the layout of the data buffer
    /// AreaInfo describes the frame buffer area which should be updated
    pub fn load_image_area<TMemoryConverterSetting: Borrow<MemoryConverterSetting>>(
        &mut self,
        target_mem_addr: u32,
        image_settings: TMemoryConverterSetting,
        area_info: &AreaImgInfo,
        data: &[u8],
    ) -> Result<(), Error> {
        // Note that area_info does not need to be rotated here, as controller hw will do the rotation
        self.set_target_memory_addr(target_mem_addr)?;

        self.interface.write_command_with_args(
            command::IT8951_TCON_LD_IMG_AREA,
            &[
                image_settings.borrow().into_arg(),
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
    /// Buffer needs to be aligned to u16!
    pub fn memory_burst_write(&mut self, memory_address: u32, data: &[u8]) -> Result<(), Error> {
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
    /// Note that this will panic if area_info is outside of screen bounding box when rotation is enabled
    pub fn display_area(
        &mut self,
        area_info: &AreaImgInfo,
        mode: WaveformMode,
    ) -> Result<(), Error> {
        let area_info = self.rotate_area_info(area_info);

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
        let area_info = self.rotate_area_info(area_info);
        let args = [
            area_info.area_x,
            area_info.area_y,
            area_info.area_w,
            area_info.area_h,
            mode as u16,
            target_mem_addr as u16,
            (target_mem_addr >> 16) as u16,
        ];

        self.wait_for_display_ready()?;
        self.interface
            .write_command_with_args(command::USDEF_I80_CMD_DPY_BUF_AREA, &args)?;
        Ok(())
    }

    /// Refresh the full E-Ink display with the frame buffer content
    /// A usecase specific wafeform must be selected by the user
    pub fn display(&mut self, mode: WaveformMode) -> Result<(), Error> {
        let size = self.size();

        self.display_area(
            &AreaImgInfo {
                area_x: 0,
                area_y: 0,
                area_w: size.width as u16,
                area_h: size.height as u16,
            },
            mode,
        )?;
        Ok(())
    }

    // misc  ------------------------------------------------------------------------------------------------

    fn wait_for_display_ready(&mut self) -> Result<(), Error> {
        let timeout = self.config.timeout_display_engine.as_micros() as u64;
        let mut counter = 0u64;
        while 0 != self.read_register(register::LUTAFSR)? {
            if counter > timeout {
                return Err(Error::DisplayEngineTimeout);
            }
            counter += 1;
            self.interface.delay(core::time::Duration::from_micros(1))?;
        }
        Ok(())
    }

    /// Activate sleep power mode
    /// All clocks, pll, osc and the panel are off, but the ram is refreshed
    pub fn sleep(mut self) -> Result<IT8951<IT8951Interface, PowerDown>, Error> {
        self.interface.write_command(command::IT8951_TCON_SLEEP)?;
        Ok(self.into_state())
    }

    /// Activate standby power mode
    /// Clocks are gated off, but pll, osc, panel power and ram is active
    pub fn standby(mut self) -> Result<IT8951<IT8951Interface, PowerDown>, Error> {
        self.interface.write_command(command::IT8951_TCON_STANDBY)?;
        Ok(self.into_state())
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

    fn rotate_area_info(&self, area: &AreaImgInfo) -> AreaImgInfo {
        use memory_converter_settings::MemoryConverterRotation::*;
        let info = self.dev_info.as_ref().expect("Unable to load device info");
        let (pw, ph) = (info.panel_width, info.panel_height);

        let (x, y, w, h) = (area.area_x, area.area_y, area.area_w, area.area_h);

        let (x, y, w, h) = match self.memory_converter_settings.rotation {
            Rotate0 => (x, y, w, h),
            Rotate90 => (y, ph - w - x, h, w),
            Rotate180 => (pw - w - x, ph - h - y, w, h),
            Rotate270 => (pw - h - y, x, h, w),
        };

        AreaImgInfo {
            area_x: x,
            area_y: y,
            area_w: w,
            area_h: h,
        }
    }
}

impl<IT8951Interface: interface::IT8951Interface> IT8951<IT8951Interface, PowerDown> {
    /// Activate active power mode
    /// This is the normal operation power mode
    pub fn sys_run(mut self) -> Result<IT8951<IT8951Interface, Run>, Error> {
        self.interface.write_command(command::IT8951_TCON_SYS_RUN)?;
        Ok(self.into_state())
    }
}

// --------------------------- embedded graphics support --------------------------------------

use embedded_graphics_core::{pixelcolor::Gray4, prelude::*, primitives::Rectangle};

impl<IT8951Interface: interface::IT8951Interface> DrawTarget for IT8951<IT8951Interface, Run> {
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

        for (area_img_info, buffer) in area_iter {
            self.load_image_area(
                memory_address,
                self.memory_converter_settings,
                &area_img_info,
                buffer,
            )?;
        }
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

        let pixel = PixelSerializer::new(area.intersection(&bb), iter, self.config.max_buffer_size);

        for (area_img_info, buffer) in pixel {
            self.load_image_area(
                memory_address,
                self.memory_converter_settings,
                &area_img_info,
                &buffer,
            )?;
        }
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
                let mut data = [0x00, 0x00];

                let value: u8 = color.luma() << ((coord.x % 2) * 4);
                // little endian layout
                // [P3, P2 | P1, P0]
                if coord.x % 4 > 1 {
                    // pixel 2 and 3
                    data[0] = value;
                } else {
                    // pixel 0 and 1
                    data[1] = value;
                }

                self.load_image_area(
                    memory_address,
                    self.memory_converter_settings,
                    &AreaImgInfo {
                        area_x: coord.x as u16,
                        area_y: coord.y as u16,
                        area_w: 1,
                        area_h: 1,
                    },
                    &data,
                )?;
            }
        }
        Ok(())
    }
}

impl<IT8951Interface: interface::IT8951Interface> OriginDimensions
    for IT8951<IT8951Interface, Run>
{
    fn size(&self) -> Size {
        let dev_info = self.dev_info.as_ref().unwrap();
        let (w, h) = (dev_info.panel_width as u32, dev_info.panel_height as u32);
        let (w, h) = match self.memory_converter_settings.rotation {
            memory_converter_settings::MemoryConverterRotation::Rotate0
            | memory_converter_settings::MemoryConverterRotation::Rotate180 => (w, h),
            memory_converter_settings::MemoryConverterRotation::Rotate90
            | memory_converter_settings::MemoryConverterRotation::Rotate270 => (h, w),
        };
        Size::new(w, h)
    }
}
