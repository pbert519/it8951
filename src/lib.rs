#![cfg_attr(not(test), no_std)]
#![warn(missing_docs)]

//! IT8951 epaper driver for the waveshare 7.8in display
//! The implementation is based on the IT8951 I80/SPI/I2C programming guide
//! provided by waveshare: https://www.waveshare.com/wiki/7.8inch_e-Paper_HAT

#[macro_use]
extern crate alloc;

use core::{borrow::Borrow, marker::PhantomData};
use heapless::String;

mod chunked_buffer;
mod command;
pub mod interface;
pub mod memory_converter_settings;
pub mod origin;
mod register;

use memory_converter_settings::MemoryConverterSetting;

#[cfg(feature = "defmt")]
use defmt;

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
    /// Display rotation
    pub rotation: Rotation,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout_display_engine: core::time::Duration::from_secs(15),
            timeout_interface: core::time::Duration::from_secs(15),
            max_buffer_size: 1024,
            rotation: Rotation::Rotate0,
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
    pub firmware_version: String<16>,
    /// LUT version
    /// The lut describes the waveforms to modify the display content
    /// LUT is specific for every display
    pub lut_version: String<16>,
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

#[cfg(feature = "defmt")]
impl defmt::Format for AreaImgInfo {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "Area Img Info {}x{} @ {}x{}",
            self.area_w,
            self.area_h,
            self.area_x,
            self.area_y
        );
    }
}

/// See https://www.waveshare.com/w/upload/c/c4/E-paper-mode-declaration.pdf for full description
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u16)]
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

/// Sets hardware rotation used by controller
/// This will perform approriate rotation for all public interfaces exposed by the driver
/// Including bounding boxes, pixel, and image drawing
pub enum Rotation {
    /// No rotation
    Rotate0,
    /// Rotate 90 degree
    Rotate90,
    /// Rotate 180 degree
    Rotate180,
    /// Rotate 270 degree
    Rotate270,
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
pub struct IT8951<IT8951Interface, TOrigin: Origin, State> {
    interface: IT8951Interface,
    dev_info: Option<DevInfo>,
    marker: core::marker::PhantomData<State>,
    origin: core::marker::PhantomData<TOrigin>,
    config: Config,
}

impl<IT8951Interface: interface::IT8951Interface, TOrigin: Origin, TState>
    IT8951<IT8951Interface, TOrigin, TState>
{
    fn into_state<TNew>(self) -> IT8951<IT8951Interface, TOrigin, TNew> {
        IT8951::<IT8951Interface, TOrigin, TNew> {
            interface: self.interface,
            dev_info: self.dev_info,
            marker: PhantomData {},
            origin: PhantomData {},
            config: self.config,
        }
    }
}

impl<IT8951Interface: interface::IT8951Interface> IT8951<IT8951Interface, OriginTopLeft, Off> {
    /// Creates a new controller driver object
    /// Call init afterwards to initalize the controller
    pub fn new(
        interface: IT8951Interface,
        config: Config,
    ) -> IT8951<IT8951Interface, OriginTopLeft, Off> {
        Self::new_with_origin(interface, config, OriginTopLeft {})
    }
}

impl<IT8951Interface: interface::IT8951Interface, TOrigin: Origin>
    IT8951<IT8951Interface, TOrigin, Off>
{
    /// Creates a new controller driver object with a customized origin type
    /// Call init afterwards to initalize the controller
    pub fn new_with_origin(
        mut interface: IT8951Interface,
        config: Config,
        _: TOrigin,
    ) -> IT8951<IT8951Interface, TOrigin, Off> {
        interface.set_busy_timeout(config.timeout_interface);
        IT8951 {
            interface,
            dev_info: None,
            marker: PhantomData {},
            origin: PhantomData {},
            config,
        }
    }

    /// Initalize the driver and resets the display
    /// VCOM should be given on your display
    /// Since version 0.4.0, this function no longer resets the display
    pub fn init(mut self, vcom: u16) -> Result<IT8951<IT8951Interface, TOrigin, Run>, Error> {
        self.interface.reset()?;

        let mut it8951 = self.into_state::<PowerDown>().sys_run()?;

        let dev_info = it8951.get_system_info()?;

        // Enable Pack Write
        it8951.write_register(register::I80CPCR, 0x0001)?;

        let current_vcom = it8951.get_vcom()?;
        if vcom != current_vcom {
            #[cfg(feature = "defmt")]
            defmt::trace!("Overriding vcom, wanted {}, current {}", vcom, current_vcom);

            it8951.set_vcom(vcom)?;
        }

        #[cfg(feature = "defmt")]
        defmt::info!(
            "Initialized screen Resolution {}x{}, LUT {=str}, FWV {=str} MA = {:x}",
            dev_info.panel_width,
            dev_info.panel_height,
            dev_info.lut_version,
            dev_info.firmware_version,
            dev_info.memory_address,
        );

        it8951.dev_info = Some(dev_info);

        Ok(it8951)
    }

    /// Create a new Driver for are already active and initalized driver
    /// This can be usefull if the device was still powered on, but the uC restarts.
    pub fn attach(
        mut interface: IT8951Interface,
        config: Config,
    ) -> Result<IT8951<IT8951Interface, OriginTopLeft, Run>, Error> {
        interface.set_busy_timeout(config.timeout_interface);

        let mut it8951: IT8951<IT8951Interface, OriginTopLeft, Run> = IT8951 {
            interface,
            dev_info: None,
            marker: PhantomData {},
            origin: PhantomData {},
            config,
        }
        .sys_run()?;

        it8951.dev_info = Some(it8951.get_system_info()?);

        #[cfg(feature = "defmt")]
        {
            let dev_info = it8951.dev_info.as_ref().unwrap();
            defmt::info!(
                "Attached screen Resolution {}x{}, LUT {=str}, FWV {=str}",
                dev_info.panel_width,
                dev_info.panel_height,
                dev_info.lut_version,
                dev_info.firmware_version,
            );
        }

        Ok(it8951)
    }
}

impl<IT8951Interface: interface::IT8951Interface, TOrigin: Origin>
    IT8951<IT8951Interface, TOrigin, Run>
{
    /// Get the Device information
    pub fn get_dev_info(&self) -> DevInfo {
        self.dev_info.clone().unwrap()
    }

    /// Increases the driver strength
    /// Use only if the image is not clear!
    pub fn enhance_driving_capability(&mut self) -> Result<(), Error> {
        self.write_register(0x0038, 0x0602)?;

        #[cfg(feature = "defmt")]
        defmt::warn!("Increased driver strength!");

        Ok(())
    }

    /// initalize the frame buffer and clear the display to white
    pub fn reset(&mut self) -> Result<(), Error> {
        self.clear(Gray4::WHITE)?;
        self.display(WaveformMode::Init)?;

        #[cfg(feature = "defmt")]
        defmt::trace!("Cleared display");

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

        #[cfg(feature = "defmt")]
        defmt::trace!("Loaded full image");
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
        area_info: &mut AreaImgInfo,
        data: &[u8],
    ) -> Result<(), Error> {
        // Note that area_info does not need to be rotated here, as controller hw will do the rotation
        self.set_target_memory_addr(target_mem_addr)?;

        TOrigin::transform(area_info, self.dev_info.as_ref().unwrap());
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

        #[cfg(feature = "defmt")]
        defmt::trace!("Loaded image area {}", area_info);

        Ok(())
    }

    fn set_target_memory_addr(&mut self, target_mem_addr: u32) -> Result<(), Error> {
        self.write_register(register::LISAR + 2, (target_mem_addr >> 16) as u16)?;
        self.write_register(register::LISAR, target_mem_addr as u16)?;

        #[cfg(feature = "defmt")]
        defmt::trace!("Target memory addr set {:x}", target_mem_addr);

        Ok(())
    }

    // buffer functions -------------------------------------------------------------------------------------------------

    /// Reads the given memory address from the controller ram into data
    /// Buffer needs to be aligned to u16!
    pub fn memory_burst_read(&mut self, memory_address: u32, data: &mut [u8]) -> Result<(), Error> {
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

        #[cfg(feature = "defmt")]
        defmt::trace!(
            "Read {} bytes of data from {:x}",
            data.len(),
            memory_address
        );

        Self::convert_endianness(data);

        Ok(())
    }

    /// Writes a buffer of u16 values to the given memory address in the controller ram
    /// Buffer needs to be aligned to u16!
    pub fn memory_burst_write(
        &mut self,
        memory_address: u32,
        data: &mut [u8],
    ) -> Result<(), Error> {
        let args = [
            memory_address as u16,
            (memory_address >> 16) as u16,
            data.len() as u16,
            (data.len() >> 16) as u16,
        ];
        self.interface
            .write_command_with_args(command::IT8951_TCON_MEM_BST_WR, &args)?;

        Self::convert_endianness(data);

        self.interface.write_multi_data(data)?;

        self.interface
            .write_command(command::IT8951_TCON_MEM_BST_END)?;

        #[cfg(feature = "defmt")]
        defmt::trace!("Wrote {} bytes of data to {:x}", data.len(), memory_address);

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

        #[cfg(feature = "defmt")]
        defmt::trace!(
            "Refreshed display area {} with mode {}",
            area_info,
            mode.clone()
        );

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

        #[cfg(feature = "defmt")]
        defmt::trace!(
            "Refreshed display area {} with mode {} from addr {}",
            area_info,
            mode,
            target_mem_addr
        );

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
    pub fn sleep(mut self) -> Result<IT8951<IT8951Interface, TOrigin, PowerDown>, Error> {
        self.interface.write_command(command::IT8951_TCON_SLEEP)?;

        #[cfg(feature = "defmt")]
        defmt::trace!("Sleep mode");

        Ok(self.into_state())
    }

    /// Activate standby power mode
    /// Clocks are gated off, but pll, osc, panel power and ram is active
    pub fn standby(mut self) -> Result<IT8951<IT8951Interface, TOrigin, PowerDown>, Error> {
        self.interface.write_command(command::IT8951_TCON_STANDBY)?;

        #[cfg(feature = "defmt")]
        defmt::trace!("Standby mode");

        Ok(self.into_state())
    }

    fn get_system_info(&mut self) -> Result<DevInfo, Error> {
        self.interface
            .write_command(command::USDEF_I80_CMD_GET_DEV_INFO)?;

        self.interface.wait_while_busy()?;

        // 40 bytes payload
        let mut buf = [0x0000; 40];
        self.interface.read_multi_data(&mut buf)?;

        Self::convert_endianness(&mut buf);

        Ok(DevInfo {
            panel_width: u16::from_be_bytes([buf[1], buf[0]]),
            panel_height: u16::from_be_bytes([buf[3], buf[2]]),
            memory_address: u32::from_be_bytes([buf[7], buf[6], buf[5], buf[4]]),
            firmware_version: Self::buf_to_str(&buf[8..24]),
            lut_version: Self::buf_to_str(&buf[25..40]),
        })
    }

    fn convert_endianness(buffer: &mut [u8]) {
        if !buffer.len().is_multiple_of(2) {
            panic!("Buffer needs to be align on u16");
        }

        for i in (0..buffer.len() - 1).step_by(2) {
            buffer.swap(i, i + 1)
        }
    }

    fn buf_to_str<const N: usize>(buffer: &[u8]) -> String<N> {
        String::from_iter(
            buffer
                .iter()
                .filter(|&&raw| raw != 0x0000)
                .map(|c| char::from(*c)),
        )
    }

    fn get_vcom(&mut self) -> Result<u16, Error> {
        self.interface.write_command(command::USDEF_I80_CMD_VCOM)?;
        self.interface.write_data(0x0000)?;
        let vcom = self.interface.read_data()?;

        #[cfg(feature = "defmt")]
        defmt::trace!("CURRENT VCOM = {}", vcom);

        Ok(vcom)
    }

    fn set_vcom(&mut self, vcom: u16) -> Result<(), Error> {
        self.interface.write_command(command::USDEF_I80_CMD_VCOM)?;
        self.interface.write_data(0x0001)?;
        self.interface.write_data(vcom)?;

        #[cfg(feature = "defmt")]
        defmt::trace!("VCOM Set {}", vcom);

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
        use Rotation::*;
        let info = self.dev_info.as_ref().expect("Unable to load device info");
        let (pw, ph) = (info.panel_width, info.panel_height);

        let (x, y, w, h) = (area.area_x, area.area_y, area.area_w, area.area_h);

        let (x, y, w, h) = match self.config.rotation {
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

impl<IT8951Interface: interface::IT8951Interface, TOrigin: Origin>
    IT8951<IT8951Interface, TOrigin, PowerDown>
{
    /// Activate active power mode
    /// This is the normal operation power mode
    pub fn sys_run(mut self) -> Result<IT8951<IT8951Interface, TOrigin, Run>, Error> {
        self.interface.write_command(command::IT8951_TCON_SYS_RUN)?;

        #[cfg(feature = "defmt")]
        defmt::trace!("Sys run");

        Ok(self.into_state())
    }
}

use crate::origin::{Origin, OriginTopLeft};
use embedded_graphics_core::{pixelcolor::Gray4, prelude::*};

impl<IT8951Interface: interface::IT8951Interface, TOrigin: Origin> OriginDimensions
    for IT8951<IT8951Interface, TOrigin, Run>
{
    fn size(&self) -> Size {
        let dev_info = self.dev_info.as_ref().unwrap();
        let (w, h) = (dev_info.panel_width as u32, dev_info.panel_height as u32);
        let (w, h) = match self.config.rotation {
            Rotation::Rotate0 | Rotation::Rotate180 => (w, h),
            Rotation::Rotate90 | Rotation::Rotate270 => (h, w),
        };
        Size::new(w, h)
    }
}
