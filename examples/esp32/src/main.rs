use embedded_graphics::{
    pixelcolor::Gray4,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use esp_idf_hal::{delay::Ets, gpio::PinDriver, prelude::*, spi::*};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use it8951::{interface::*, *};
use embedded_graphics::{prelude::*, primitives::{Rectangle, PrimitiveStyle}, pixelcolor::Gray4};
use it8951::Config;

mod demo;

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
    let mut epd = IT8951::new(display_interface, Config::default()).init(1605).unwrap();

    log::info!("Initialized display: {:?}", epd.get_dev_info());

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

    demo::run(&mut epd);

    let _epd = epd.standby().unwrap();

    loop {
        println!("Reached main loop, sleep!");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn setup_watchdog() -> Result<(), esp_idf_sys::EspError> {
    // Setup watchdog to 120s
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
