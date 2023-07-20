#![cfg_attr(not(test), no_std)]

use core::fmt::Debug;
use embedded_hal::{
    blocking::{delay::*, spi::Transfer, spi::Write},
    digital::v2::{InputPin, OutputPin},
};

// ---- IT8951 Registers defines -----------------------------------------------------------------

//Register Base Address
const DISPLAY_REG_BASE: u16 = 0x1000; //Register RW access

//Base Address of Basic LUT Registers
#[allow(clippy::identity_op)]
const _LUT0EWHR: u16 = DISPLAY_REG_BASE + 0x00; //LUT0 Engine Width Height Reg
const _LUT0XYR: u16 = DISPLAY_REG_BASE + 0x40; //LUT0 XY Reg
const _LUT0BADDR: u16 = DISPLAY_REG_BASE + 0x80; //LUT0 Base Address Reg
const _LUT0MFN: u16 = DISPLAY_REG_BASE + 0xC0; //LUT0 Mode and Frame number Reg
const _LUT01AF: u16 = DISPLAY_REG_BASE + 0x114; //LUT0 and LUT1 Active Flag Reg

//Update Parameter Setting Register
const _UP0SR: u16 = DISPLAY_REG_BASE + 0x134; //Update Parameter0 Setting Reg
const _UP1SR: u16 = DISPLAY_REG_BASE + 0x138; //Update Parameter1 Setting Reg
const _LUT0ABFRV: u16 = DISPLAY_REG_BASE + 0x13C; //LUT0 Alpha blend and Fill rectangle Value
const _UPBBADDR: u16 = DISPLAY_REG_BASE + 0x17C; //Update Buffer Base Address
const _LUT0IMXY: u16 = DISPLAY_REG_BASE + 0x180; //LUT0 Image buffer X/Y offset Reg
const LUTAFSR: u16 = DISPLAY_REG_BASE + 0x224; //LUT Status Reg (status of All LUT Engines)
const _BGVR: u16 = DISPLAY_REG_BASE + 0x250; //Bitmap (1bpp) image color table

//System Registers
const SYS_REG_BASE: u16 = 0x0000;

//Address of System Registers
const I80CPCR: u16 = SYS_REG_BASE + 0x04;

//Memory Converter Registers
const MCSR_BASE_ADDR: u16 = 0x0200;
#[allow(clippy::identity_op)]
const _MCSR: u16 = MCSR_BASE_ADDR + 0x0000;
const LISAR: u16 = MCSR_BASE_ADDR + 0x0008;

// ---- IT8951 Command defines -----------------------------------------------------------------
// Commands
const IT8951_TCON_SYS_RUN: u16 = 0x0001;
const IT8951_TCON_STANDBY: u16 = 0x0002;
const IT8951_TCON_SLEEP: u16 = 0x0003;
const _IT8951_TCON_REG_RD: u16 = 0x0010;
const IT8951_TCON_REG_WR: u16 = 0x0011;
const _IT8951_TCON_MEM_BST_RD_T: u16 = 0x0012;
const _IT8951_TCON_MEM_BST_RD_S: u16 = 0x0013;
const _IT8951_TCON_MEM_BST_WR: u16 = 0x0014;
const _IT8951_TCON_MEM_BST_END: u16 = 0x0015;
const IT8951_TCON_LD_IMG: u16 = 0x0020;
const IT8951_TCON_LD_IMG_AREA: u16 = 0x0021;
const IT8951_TCON_LD_IMG_END: u16 = 0x0022;

//I80 User defined command code
const _USDEF_I80_CMD_DPY_AREA: u16 = 0x0034;
const USDEF_I80_CMD_GET_DEV_INFO: u16 = 0x0302;
const _USDEF_I80_CMD_DPY_BUF_AREA: u16 = 0x0037;
const USDEF_I80_CMD_VCOM: u16 = 0x0039;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    SpiError,
    GPIOError,
}

#[derive(Debug, Clone)]
pub struct DevInfo {
    pub panel_width: u16,
    pub panel_height: u16,
    pub memory_address: u32,
    pub firmware_version: [u8; 16],
    pub lut_version: [u8; 16],
}

struct LoadImgInfo {
    endian_type: u16,
    pixel_format: u16,
    rotate: u16,
    source_buffer_addr: u32,
    target_memory_addr: u32,
}

struct AreaImgInfo {
    area_x: u16,
    area_y: u16,
    area_w: u16,
    area_h: u16,
}

pub struct IT8951<SPI, BUSY, RST, DELAY> {
    spi: SPI,
    busy: BUSY,
    rst: RST,
    delay: DELAY,
    dev_info: Option<DevInfo>,
}

impl<SPI, BUSY, RST, DELAY> IT8951<SPI, BUSY, RST, DELAY>
where
    SPI: Write<u8> + Transfer<u8>,
    BUSY: InputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    pub fn new(spi: SPI, busy: BUSY, rst: RST, delay: DELAY) -> IT8951<SPI, BUSY, RST, DELAY> {
        IT8951 {
            spi,
            busy,
            rst,
            delay,
            dev_info: None,
        }
    }

    pub fn init(&mut self, vcom: u16) -> Result<(), Error> {
        self.reset()?;
        self.write_command(IT8951_TCON_SYS_RUN)?;

        let dev_info = self.get_system_info()?;

        // Enable Pack Write
        self.write_register(I80CPCR, 0x0001)?;

        if vcom != self.get_vcom()? {
            self.set_vcom(vcom)?;
        }

        self.dev_info = Some(dev_info);

        Ok(())
    }

    pub fn get_dev_info(&self) -> &Option<DevInfo> {
        &self.dev_info
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        if self.rst.set_high().is_err() {
            return Err(Error::GPIOError);
        }
        self.delay.delay_ms(200);
        if self.rst.set_low().is_err() {
            return Err(Error::GPIOError);
        }
        self.delay.delay_ms(20);
        if self.rst.set_high().is_err() {
            return Err(Error::GPIOError);
        }
        self.delay.delay_ms(200);
        Ok(())
    }

    pub fn clear_refresh(&mut self) {
        //let dev_info = self.dev_info.as_ref().unwrap();
        todo!();
    }

    pub fn sleep(&mut self) -> Result<(), Error> {
        self.write_command(IT8951_TCON_SLEEP)?;
        Ok(())
    }

    #[deprecated]
    pub fn standby(&mut self) -> Result<(), Error> {
        self.write_command(IT8951_TCON_STANDBY)?;
        Ok(())
    }

    // set display functions ------------------------------------------------------------------------------------------

    fn set_target_memory_addr(&mut self, target_mem_addr: u32) -> Result<(), Error> {
        self.write_register(LISAR + 2, (target_mem_addr >> 16) as u16)?;
        self.write_register(LISAR, target_mem_addr as u16)?;
        Ok(())
    }

    fn load_image_start(&mut self, image_info: &LoadImgInfo) -> Result<(), Error> {
        let arg0: u16 =
        (image_info.endian_type << 8) | (image_info.pixel_format << 4) | image_info.rotate;

        self.write_command(IT8951_TCON_LD_IMG)?;
        self.write_data(arg0)?;

        Ok(())
    }

    fn load_img_area_start(&mut self, image_info: &LoadImgInfo, area_info: &AreaImgInfo) -> Result<(), Error>{
        let arg0: u16 =
            (image_info.endian_type << 8) | (image_info.pixel_format << 4) | image_info.rotate;

        let args = [
            arg0,
            area_info.area_x,
            area_info.area_y,
            area_info.area_w,
            area_info.area_h,
        ];

        self.write_multi_args(IT8951_TCON_LD_IMG_AREA, &args)?;
        Ok(())
    }

    fn load_img_end(&mut self) -> Result<(), Error> {
        self.write_command(IT8951_TCON_LD_IMG_END)?;
        Ok(())
    }

    fn host_area_packed_pixel_write_4bp(&mut self,  image_info: &LoadImgInfo, area_info: &AreaImgInfo) -> Result<(), Error>{
        
        self.set_target_memory_addr(image_info.target_memory_addr)?;
        self.load_img_area_start(image_info, area_info)?;

        // write data

        self.load_img_end()?;

        Ok(())
    }

    pub fn refresh_4bp(&mut self) -> Result<(), Error> {
        self.wait_for_display_ready()?;

        todo!();

        Ok(())
    }

    // private functions ------------------------------------------------------------------------------------------------

    fn wait_for_display_ready(&mut self) -> Result<(), Error> {
        while Ok(0) != self.read_register(LUTAFSR) {}
        Ok(())
    }

    fn get_system_info(&mut self) -> Result<DevInfo, Error> {
        self.write_command(USDEF_I80_CMD_GET_DEV_INFO)?;

        self.wait_while_busy();

        // 40 bytes payload + 2 dummby response bytes + 2 bytes write preamble
        let mut buf = [0x00; 44];
        buf[0] = 0x10;
        buf[1] = 0x00;
        if self.spi.transfer(&mut buf).is_err() {
            return Err(Error::SpiError);
        }

        Ok(DevInfo {
            panel_width: u16::from_be_bytes([buf[4], buf[5]]),
            panel_height: u16::from_be_bytes([buf[6], buf[7]]),
            memory_address: u32::from_be_bytes([buf[10], buf[11], buf[8], buf[9]]),
            firmware_version: buf[12..28].try_into().unwrap(),
            lut_version: buf[28..44].try_into().unwrap(),
        })
    }

    fn get_vcom(&mut self) -> Result<u16, Error> {
        self.write_command(USDEF_I80_CMD_VCOM)?;
        self.write_data(0x0000)?;
        self.read_data()
    }

    fn set_vcom(&mut self, vcom: u16) -> Result<(), Error> {
        self.write_command(USDEF_I80_CMD_VCOM)?;
        self.write_data(0x0001)?;
        self.write_data(vcom)?;
        Ok(())
    }

    fn read_register(&mut self, reg: u16) -> Result<u16, Error> {
        self.write_command(_IT8951_TCON_REG_RD)?;
        self.write_data(reg)?;
        self.read_data()
    }

    fn write_register(&mut self, reg: u16, data: u16) -> Result<(), Error> {
        self.write_command(IT8951_TCON_REG_WR)?;
        self.write_data(reg)?;
        self.write_data(data)?;
        Ok(())
    }

    fn write_data(&mut self, data: u16) -> Result<(), Error> {
        self.wait_while_busy();

        // Write Data:
        // 0x0000 -> Prefix for a Command
        // data; u16 -> 16bit data to write
        let buf = [0x00, 0x00, (data >> 8) as u8, data as u8];

        if self.spi.write(&buf).is_err() {
            return Err(Error::SpiError);
        }

        Ok(())
    }

    fn write_command(&mut self, cmd: u16) -> Result<(), Error> {
        self.wait_while_busy();

        // Write Command:
        // 0x6000 -> Prefix for a Command
        // cmd; u16 -> 16bit Command code
        let buf = [0x60, 0x00, (cmd >> 8) as u8, cmd as u8];

        if self.spi.write(&buf).is_err() {
            return Err(Error::SpiError);
        }

        Ok(())
    }

    fn write_multi_args(&mut self, cmd: u16, args: &[u16]) -> Result<(), Error> {
        self.write_command(cmd)?;
        for arg in args {
            self.write_data(*arg)?;
        }
        Ok(())
    }

    fn read_data(&mut self) -> Result<u16, Error> {
        self.wait_while_busy();

        // Read Data
        // 0x1000 -> Prefix for Read Data
        let mut buf = [0x10, 0x00, 0x00, 0x00, 0x00, 0x00];
        if self.spi.transfer(&mut buf).is_err() {
            return Err(Error::SpiError);
        }
        // we skip the first 2 bytes -> shifted out while transfer the prefix
        // the next two bytes are only dummies and are skipped to
        // only the last two bytes are the expected data and are stored
        Ok(u16::from_be_bytes([buf[4], buf[5]]))
    }

    fn wait_while_busy(&mut self) {
        while self.busy.is_low().unwrap_or(true) {}
    }
}

// --------------------------- embedded graphics support --------------------------------------

use embedded_graphics::{pixelcolor::Gray4, prelude::*};

impl<SPI, BUSY, RST, DELAY> DrawTarget for IT8951<SPI, BUSY, RST, DELAY>
where
    SPI: Write<u8> + Transfer<u8>,
    BUSY: InputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    type Color = Gray4;

    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let dev_info = self.dev_info.as_ref().unwrap();
        let width = dev_info.panel_width as i32;
        let height = dev_info.panel_height as i32;
        for Pixel(coord, color) in pixels.into_iter() {
            if (coord.x >= 0 && coord.x < width) || (coord.y >= 0 || coord.y < height) {
                todo!("write pixel")
            }
        }
        Ok(())
    }
}

impl<SPI, BUSY, RST, DELAY> OriginDimensions for IT8951<SPI, BUSY, RST, DELAY>
where
    SPI: Write<u8> + Transfer<u8>,
    BUSY: InputPin,
    RST: OutputPin,
    DELAY: DelayMs<u8>,
{
    fn size(&self) -> Size {
        let dev_info = self.dev_info.as_ref().unwrap();
        Size::new(dev_info.panel_width as u32, dev_info.panel_height as u32)
    }
}
