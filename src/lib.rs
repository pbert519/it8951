#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate alloc;

pub mod comm;

use core::fmt::Debug;

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
const USDEF_I80_CMD_DPY_AREA: u16 = 0x0034;
const USDEF_I80_CMD_GET_DEV_INFO: u16 = 0x0302;
const USDEF_I80_CMD_DPY_BUF_AREA: u16 = 0x0037;
const USDEF_I80_CMD_VCOM: u16 = 0x0039;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Interface(comm::Error),
}
impl From<comm::Error> for Error {
    fn from(e: comm::Error) -> Self {
        Error::Interface(e)
    }
}

#[derive(Debug, Clone)]
pub struct DevInfo {
    pub panel_width: u16,
    pub panel_height: u16,
    pub memory_address: u32,
    pub firmware_version: [u16; 8],
    pub lut_version: [u16; 8],
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

pub struct IT8951<IT8951Interface> {
    interface: IT8951Interface,
    dev_info: Option<DevInfo>,
}

impl<IT8951Interface> IT8951<IT8951Interface>
where
    IT8951Interface: comm::IT8951Interface,
{
    pub fn new(interface: IT8951Interface) -> IT8951<IT8951Interface> {
        IT8951 {
            interface,
            dev_info: None,
        }
    }

    pub fn init(&mut self, vcom: u16) -> Result<(), Error> {
        self.interface.reset()?;
        self.interface.write_command(IT8951_TCON_SYS_RUN)?;

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

    pub fn clear_refresh(&mut self) {
        //let dev_info = self.dev_info.as_ref().unwrap();
        todo!();
    }

    pub fn sleep(&mut self) -> Result<(), Error> {
        self.interface.write_command(IT8951_TCON_SLEEP)?;
        Ok(())
    }

    pub fn standby(&mut self) -> Result<(), Error> {
        self.interface.write_command(IT8951_TCON_STANDBY)?;
        Ok(())
    }

    pub fn enhance_driving_capability(&mut self) -> Result<(), Error>{
        self.write_register(0x0038, 0x0602)?;
        Ok(())
    }

    // load image functions ------------------------------------------------------------------------------------------

    fn set_target_memory_addr(&mut self, target_mem_addr: u32) -> Result<(), Error> {
        self.write_register(LISAR + 2, (target_mem_addr >> 16) as u16)?;
        self.write_register(LISAR, target_mem_addr as u16)?;
        Ok(())
    }

    fn load_image_start(&mut self, image_info: &LoadImgInfo) -> Result<(), Error> {
        let arg0: u16 =
            (image_info.endian_type << 8) | (image_info.pixel_format << 4) | image_info.rotate;

        self.interface.write_command(IT8951_TCON_LD_IMG)?;
        self.interface.write_data(arg0)?;

        Ok(())
    }

    fn load_img_area_start(
        &mut self,
        image_info: &LoadImgInfo,
        area_info: &AreaImgInfo,
    ) -> Result<(), Error> {
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
        self.interface.write_command(IT8951_TCON_LD_IMG_END)?;
        Ok(())
    }

    fn host_area_packed_pixel_write_4bp(
        &mut self,
        image_info: &LoadImgInfo,
        area_info: &AreaImgInfo,
    ) -> Result<(), Error> {
        self.set_target_memory_addr(image_info.target_memory_addr)?;
        self.load_img_area_start(image_info, area_info)?;

        // write data

        self.load_img_end()?;

        Ok(())
    }

    pub fn refresh_4bp(&mut self) -> Result<(), Error> {
        self.wait_for_display_ready()?;

        let image_info = LoadImgInfo {
            endian_type: 0,
            pixel_format: 0,
            rotate: 0,
            source_buffer_addr: 0,
            target_memory_addr: 0,
        };
        let area_info = AreaImgInfo {
            area_x: 0,
            area_y: 0,
            area_w: 10,
            area_h: 10,
        };

        self.host_area_packed_pixel_write_4bp(&image_info, &area_info)?;

        Ok(())
    }

    // display functions ------------------------------------------------------------------------------------------------
    pub fn display_area(&mut self, x: u16, y: u16, w: u16, h: u16, mode: u16) -> Result<(), Error> {
        let args = [x, y, w, h, mode];

        self.write_multi_args(USDEF_I80_CMD_DPY_AREA, &args)?;
        Ok(())
    }

    pub fn display_area_buf(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        mode: u16,
        target_mem_addr: u32,
    ) -> Result<(), Error> {
        let args = [
            x,
            y,
            w,
            h,
            mode,
            target_mem_addr as u16,
            (target_mem_addr >> 16) as u16,
        ];

        self.write_multi_args(USDEF_I80_CMD_DPY_BUF_AREA, &args)?;
        Ok(())
    }

    // private functions ------------------------------------------------------------------------------------------------

    fn wait_for_display_ready(&mut self) -> Result<(), Error> {
        while Ok(0) != self.read_register(LUTAFSR) {}
        Ok(())
    }

    fn get_system_info(&mut self) -> Result<DevInfo, Error> {
        self.interface.write_command(USDEF_I80_CMD_GET_DEV_INFO)?;

        self.interface.wait_while_busy()?;

        // 40 bytes payload
        let mut buf = [0x0000; 20];
        self.interface.read_multi_data(&mut buf)?;

        Ok(DevInfo {
            panel_width: buf[0],
            panel_height: buf[1],
            memory_address: ((buf[2] as u32) << 16) | (buf[3] as u32),
            firmware_version: buf[4..12].try_into().unwrap(),
            lut_version: buf[12..20].try_into().unwrap(),
        })
    }

    fn get_vcom(&mut self) -> Result<u16, Error> {
        self.interface.write_command(USDEF_I80_CMD_VCOM)?;
        self.interface.write_data(0x0000)?;
        let vcom = self.interface.read_data()?;
        Ok(vcom)
    }

    fn set_vcom(&mut self, vcom: u16) -> Result<(), Error> {
        self.interface.write_command(USDEF_I80_CMD_VCOM)?;
        self.interface.write_data(0x0001)?;
        self.interface.write_data(vcom)?;
        Ok(())
    }

    fn read_register(&mut self, reg: u16) -> Result<u16, Error> {
        self.interface.write_command(_IT8951_TCON_REG_RD)?;
        self.interface.write_data(reg)?;
        let data = self.interface.read_data()?;
        Ok(data)
    }

    fn write_register(&mut self, reg: u16, data: u16) -> Result<(), Error> {
        self.interface.write_command(IT8951_TCON_REG_WR)?;
        self.interface.write_data(reg)?;
        self.interface.write_data(data)?;
        Ok(())
    }

    fn write_multi_args(&mut self, cmd: u16, args: &[u16]) -> Result<(), Error> {
        self.interface.write_command(cmd)?;
        for arg in args {
            self.interface.write_data(*arg)?;
        }
        Ok(())
    }
}

// --------------------------- embedded graphics support --------------------------------------

use embedded_graphics::{pixelcolor::Gray4, prelude::*};

impl<IT8951Interface> DrawTarget for IT8951<IT8951Interface>
where
    IT8951Interface: comm::IT8951Interface,
{
    type Color = Gray4;

    type Error = core::convert::Infallible;

    // fetch area from it8951 frame buffer,
    // overwrites given pixels
    // transmit area
    // How to areas to big for the internal memory? Split into areas? How to handle "wrong" order of pixels?
    // full refresh
    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        todo!()
    }

    // create area locally and send it to the devices
    // split into multiple buffers if to big for ram
    // refresh full frame
    fn fill_solid(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        color: Self::Color,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        todo!()
    }

    // Fetch frame buffer for every pixel,
    // modify it
    // and upload it again
    // after all pixels are processed, refresh full frame
    // do we even have a accessible frame buffer?!
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        let dev_info = self.dev_info.as_ref().unwrap();
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
    IT8951Interface: comm::IT8951Interface,
{
    fn size(&self) -> Size {
        let dev_info = self.dev_info.as_ref().unwrap();
        Size::new(dev_info.panel_width as u32, dev_info.panel_height as u32)
    }
}
