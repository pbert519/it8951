//! Settings for the pixel preprocessing / memory converter unit on the controller

/// Endianness of the pixel data send to the controller
#[repr(u16)]
pub enum MemoryConverterEndianness {
    /// pixel data is little endian
    LittleEndian = 0,
    /// pixel data is big endian
    BigEndian = 1,
}

/// Bits per pixel
/// the pixel data send to the controller can encode the pixels with a different number of bits
#[repr(u16)]
pub enum MemoryConverterBitPerPixel {
    /// each pixel value is given by 2 bits
    BitsPerPixel2 = 0b00,
    /// each pixel value is given by 2 bits
    BitsPerPixel3 = 0b01,
    /// each pixel value is given by 4 bits
    BitsPerPixel4 = 0b10,
    /// each pixel value is given by 8 bits
    BitsPerPixel8 = 0b11,
}

/// The memory converter supports rotating the written pixel data
#[repr(u16)]
pub enum MemoryConverterRotation {
    /// dont rotate image
    Rotate0 = 0b00,
    /// rotate image by 90 degree
    Rotate90 = 0b01,
    /// rotate image by 180 degree
    Rotate180 = 0b10,
    /// rotate image by 270 degree
    Rotate270 = 0b11,
}

/// Memory converter settings
/// pixel data send by the load_image commands can be converted by the controller
pub struct MemoryConverterSetting {
    /// pixel data endianess
    pub endianness: MemoryConverterEndianness,
    /// enocding of each pixel
    pub bit_per_pixel: MemoryConverterBitPerPixel,
    /// image rotation settinsg
    pub rotation: MemoryConverterRotation,
}

impl Default for MemoryConverterSetting {
    fn default() -> Self {
        Self {
            endianness: MemoryConverterEndianness::LittleEndian,
            bit_per_pixel: MemoryConverterBitPerPixel::BitsPerPixel4,
            rotation: MemoryConverterRotation::Rotate0,
        }
    }
}

impl MemoryConverterSetting {
    pub(crate) fn into_arg(self) -> u16 {
        let endianness = self.endianness as u16;
        let bit_per_pixel = self.bit_per_pixel as u16;
        let rotation = self.rotation as u16;
        (endianness << 8) | (bit_per_pixel << 4) | rotation
    }
}
