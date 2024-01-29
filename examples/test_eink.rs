use linux_embedded_hal::gpio_cdev::{Chip, LineRequestFlags};
use linux_embedded_hal::spidev::{SpiModeFlags, SpidevOptions};
use linux_embedded_hal::{CdevPin, Delay, SpidevDevice};
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
    let mut spi = SpidevDevice::open("/dev/spidev0.0")?;
    let spi_options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(12_000_000)
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
    let mut epd = it8951::IT8951::new(driver).init(1670).unwrap();

    println!(
        "Reset and initalized E-Ink Display: \n\r {:?}",
        epd.get_dev_info()
    );

    // Draw a filled square
    Rectangle::new(Point::new(50, 350), Size::new(20, 20))
        .into_styled(PrimitiveStyle::with_fill(Gray4::BLACK))
        .draw(&mut epd)
        .unwrap();

    Rectangle::new(Point::new(0, 1000), Size::new(200, 200))
        .into_styled(PrimitiveStyle::with_fill(Gray4::new(8)))
        .draw(&mut epd)
        .unwrap();

    // Draw centered text.
    let text = "IT8951 Driver Example";
    embedded_graphics::text::Text::with_alignment(
        text,
        epd.bounding_box().center() + Point::new(0, 15),
        embedded_graphics::mono_font::MonoTextStyle::new(
            &embedded_graphics::mono_font::iso_8859_1::FONT_9X18_BOLD,
            Gray4::new(11),
        ),
        embedded_graphics::text::Alignment::Center,
    )
    .draw(&mut epd)
    .unwrap();

    epd.display(it8951::WaveformMode::GL16).unwrap();

    epd.sleep().unwrap();

    Ok(())
}
