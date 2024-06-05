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
pub const UP1SR: u16 = DISPLAY_REG_BASE + 0x138; //Update Parameter1 Setting Reg
const _LUT0ABFRV: u16 = DISPLAY_REG_BASE + 0x13C; //LUT0 Alpha blend and Fill rectangle Value
const _UPBBADDR: u16 = DISPLAY_REG_BASE + 0x17C; //Update Buffer Base Address
const _LUT0IMXY: u16 = DISPLAY_REG_BASE + 0x180; //LUT0 Image buffer X/Y offset Reg
pub const LUTAFSR: u16 = DISPLAY_REG_BASE + 0x224; //LUT Status Reg (status of All LUT Engines)
pub const BGVR: u16 = DISPLAY_REG_BASE + 0x250; //Bitmap (1bpp) image color table

//System Registers
const SYS_REG_BASE: u16 = 0x0000;

//Address of System Registers
pub const I80CPCR: u16 = SYS_REG_BASE + 0x04;

//Memory Converter Registers
const MCSR_BASE_ADDR: u16 = 0x0200;
#[allow(clippy::identity_op)]
const _MCSR: u16 = MCSR_BASE_ADDR + 0x0000;
pub const LISAR: u16 = MCSR_BASE_ADDR + 0x0008;
