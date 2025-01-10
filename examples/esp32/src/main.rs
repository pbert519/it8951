use std::time::Duration;

use embedded_graphics::{
    pixelcolor::Gray4,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use esp_idf_hal::{delay::Ets, gpio::PinDriver, prelude::*, spi::*};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use it8951::Config;
use it8951::{interface::*, *};

fn main() -> ! {
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    setup_watchdog().unwrap();

    // Setup display
    // Enable display power supply
    let mut display_en = PinDriver::output(peripherals.pins.gpio18).unwrap();
    display_en.set_high().unwrap();

    let mut reset = PinDriver::output(peripherals.pins.gpio1).unwrap();
    reset.set_high().unwrap();
    let display_interface = IT8951SPIInterface::new(
        SpiDeviceDriver::new_single(
            peripherals.spi2,
            peripherals.pins.gpio7,
            peripherals.pins.gpio6,
            Some(peripherals.pins.gpio2),
            Some(peripherals.pins.gpio0),
            &SpiDriverConfig::new(),
            &config::Config::new().baudrate(10.MHz().into()),
        )
        .unwrap(),
        PinDriver::input(peripherals.pins.gpio5).unwrap(),
        reset,
        Ets,
    );
    let mut epd = IT8951::new(display_interface, Config::default())
        .init(1605)
        .unwrap();
    epd.reset().unwrap();

    log::info!("Initialized display: {:?}", epd.get_dev_info());

    // Draw a filled square
    Rectangle::new(Point::new(50, 350), Size::new(20, 20))
        .into_styled(PrimitiveStyle::with_fill(Gray4::BLACK))
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
    let mut sleppy_epd = epd.sleep().unwrap();

    let mut xpos = -200;

    loop {
        std::thread::sleep(Duration::from_millis(1000));

        // wakeup display
        let mut epd = sleppy_epd.sys_run().unwrap();

        // clear old rectangle
        Rectangle::new(Point::new(xpos, 1000), Size::new(200, 200))
        .into_styled(PrimitiveStyle::with_fill(Gray4::WHITE))
        .draw(&mut epd)
        .unwrap();

        // update rectangle pos
        if xpos >= epd.size().width as i32 {
            xpos = -200;
        } else {
            xpos +=10;
        }

        // draw new rectangle
        Rectangle::new(Point::new(xpos, 1000), Size::new(200, 200))
        .into_styled(PrimitiveStyle::with_fill(Gray4::BLACK))
        .draw(&mut epd)
        .unwrap();

        // Update screen
        epd.display(it8951::WaveformMode::A2).unwrap();

        // sleep display
        sleppy_epd = epd.sleep().unwrap();
    }
}

fn setup_watchdog() -> Result<(), esp_idf_sys::EspError> {
    // Setup watchdog to 45s
    unsafe {
        esp_idf_sys::esp!(esp_idf_sys::esp_task_wdt_reconfigure(
            &esp_idf_sys::esp_task_wdt_config_t {
                idle_core_mask: 1,
                timeout_ms: 45000,
                trigger_panic: true,
            }
        ))
        .unwrap()
    };
    Ok(())
}
