# Driver for IT8951 E-Paper display

This crate is mainly developed for the waveshare 7.8" epaper display using spi:
https://www.waveshare.com/wiki/7.8inch_e-Paper_HAT
The driver uses the embedded_hal traits as hardware abstraction layer.
This driver can be used with the embedded graphics trait, currently only supporing Gray4 (16bit grayscale).

## Details
- IT8951 has a image load engine which can convert pixel data before storing it in the local frame  buffer.
- It is possible to read and write the memory directly without using the image load engine
- **Important** Data must be always aligned to 16bit words!
- The crates uses the alloc feature to allocate memory on the heap:
    - Firmware and LUT version string read from the controller
    - Staging buffers to write pixel to the controller. The buffers are allocated as needed, but only one buffer at a time and with up to 1kByte of size.
    - When reading controller memory a staging buffer with the size of of the requested data is created.


## TODOs
- Support Gray2 and Gray8 with embedded-graphics
- Support display engine fill area
- Support display engine 1 bit per pixel mode
- Support static buffer allocations

