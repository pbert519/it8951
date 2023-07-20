// ---- IT8951 Command defines -----------------------------------------------------------------
// Commands
pub const IT8951_TCON_SYS_RUN: u16 = 0x0001;
pub const IT8951_TCON_STANDBY: u16 = 0x0002;
pub const IT8951_TCON_SLEEP: u16 = 0x0003;
pub const IT8951_TCON_REG_RD: u16 = 0x0010;
pub const IT8951_TCON_REG_WR: u16 = 0x0011;
pub const IT8951_TCON_MEM_BST_RD_T: u16 = 0x0012; // trigger fifo to read from memeory
pub const IT8951_TCON_MEM_BST_RD_S: u16 = 0x0013; // read from fifo
pub const IT8951_TCON_MEM_BST_WR: u16 = 0x0014; // write to memory
pub const IT8951_TCON_MEM_BST_END: u16 = 0x0015;
pub const IT8951_TCON_LD_IMG: u16 = 0x0020;
pub const IT8951_TCON_LD_IMG_AREA: u16 = 0x0021;
pub const IT8951_TCON_LD_IMG_END: u16 = 0x0022;

//I80 User defined command code
pub const USDEF_I80_CMD_DPY_AREA: u16 = 0x0034;
pub const USDEF_I80_CMD_GET_DEV_INFO: u16 = 0x0302;
pub const USDEF_I80_CMD_DPY_BUF_AREA: u16 = 0x0037;
pub const USDEF_I80_CMD_VCOM: u16 = 0x0039;
