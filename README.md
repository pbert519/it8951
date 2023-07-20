# Driver for IT8951 E-Paper driver

This crate is mainly developed for the waveshare 7.8" epaper display using spi:
https://www.waveshare.com/wiki/7.8inch_e-Paper_HAT
The driver uses the embedded_hal traits as hardware abstraction layer.
This driver can be used with the embedded graphics trait.

## Details
- IT8951 has a image load engine which can convert pixel data before storing it in the local frame buffer
- It is possible to read and write the memory directly without using the image load engine
- **Important** Data must be always aligned to 16bit words!