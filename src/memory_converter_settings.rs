#[repr(u16)]
pub enum MemoryConverterEndianness {
    LittleEndian = 0,
    BigEndian = 1,
}

#[repr(u16)]
pub enum MemoryConverterBitPerPixel {
    BitsPerPixel2 = 0b00,
    BitsPerPixel3 = 0b01,
    BitsPerPixel4 = 0b10,
    BitsPerPixel8 = 0b11,
}

#[repr(u16)]
pub enum MemoryConverterRotation {
    Rotate0 = 0b00,
    Rotate90 = 0b01,
    Rotate180 = 0b10,
    Rotate270 = 0b11,
}

pub struct MemoryConverterSetting {
    pub endianness: MemoryConverterEndianness,
    pub bit_per_pixel: MemoryConverterBitPerPixel,
    pub rotation: MemoryConverterRotation,
}

impl MemoryConverterSetting {
    pub fn into_arg(self) -> u16 {
        let endianness = self.endianness as u16;
        let bit_per_pixel = self.bit_per_pixel as u16;
        let rotation = self.rotation as u16;
        (endianness << 8) | (bit_per_pixel << 4) | rotation
    }
}
