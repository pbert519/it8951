use it8951::AreaImgInfo;
use linux_embedded_hal::gpio_cdev::{Chip, LineRequestFlags};
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::{CdevPin, Delay, Spidev};
use std::error::Error;

use embedded_graphics::{
    pixelcolor::Gray4,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

fn main() -> Result<(), Box<dyn Error>> {
    // Raspi SPI0.0
    // MISO: 9
    // MOSI: 10
    // SCK: 11
    // CS: 8
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let spi_options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&spi_options)?;

    let mut chip = Chip::new("/dev/gpiochip0")?;
    // RST: 17
    let rst_output = chip.get_line(17)?;
    let rst_output_handle = rst_output.request(LineRequestFlags::OUTPUT, 0, "meeting-room")?;
    let rst = CdevPin::new(rst_output_handle)?;
    // BUSY / HDRY: 24
    let busy_input = chip.get_line(24)?;
    let busy_input_handle = busy_input.request(LineRequestFlags::INPUT, 0, "meeting-room")?;
    let busy = CdevPin::new(busy_input_handle)?;

    let driver = it8951::interface::IT8951SPIInterface::new(spi, busy, rst, Delay);
    let mut epd = it8951::IT8951::new(driver);
    println!("Initalize Display");

    epd.init(1670).unwrap();

    println!("Initalized E-Ink Display: \n\r {:?}", epd.get_dev_info());

    // ToDo: Fails with not yet implemented
    // clear screen
    // epd.clear(Gray4::WHITE)?;

    // Draw a filled square
    // Rectangle::new(Point::new(50, 350), Size::new(200, 200))
    //    .into_styled(PrimitiveStyle::with_fill(Gray4::BLACK))
    //    .draw(&mut epd)?;

    epd.clear(Gray4::WHITE).expect("Error clearing display");

    let mut buf = [0x0; 100];
    let mem = epd.get_dev_info().as_ref().unwrap().memory_address;
    epd.memory_burst_read(mem, &mut buf).unwrap();
    println!("{:?}", buf);

    epd.sleep().unwrap();

    Ok(())
}
