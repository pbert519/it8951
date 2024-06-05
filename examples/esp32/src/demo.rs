// based on https://github.com/waveshareteam/IT8951-ePaper/blob/master/Raspberry
use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::{Gray4, GrayColor},
};
use it8951::{
    interface::*,
    memory_converter_settings::{MemoryConverterBitPerPixel, MemoryConverterSetting},
    *,
};
use std::time::Instant;

pub fn display_color_palette_example(
    epd: &mut it8951::IT8951<impl IT8951Interface, Run>,
    bpp: MemoryConverterBitPerPixel,
) {
    // create stripes for each color and display it
    let width = epd.get_dev_info().panel_width as usize;
    let height = 85;
    let pixels = width * height / bpp.pixel_per_byte() / 2;
    let mut buf = vec![0u16; pixels];

    let pattern = [
        0x0000, 0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777, 0x8888, 0x9999, 0xAAAA,
        0xBBBB, 0xCCCC, 0xDDDD, 0xEEEE, 0xFFFF,
    ];

    let before = Instant::now();

    for i in 0..pattern.len() {
        buf.fill(pattern[i]);
        // write buff & display

        epd.load_image_area(
            epd.get_dev_info().memory_address,
            MemoryConverterSetting {
                bit_per_pixel: bpp,
                ..MemoryConverterSetting::default()
            },
            &AreaImgInfo {
                area_x: 0,
                area_y: (i * height) as u16,
                area_w: width as u16,
                area_h: height as u16,
            },
            &buf,
        )
        .unwrap();
        println!(
            "all Elapsed time: {:.2?}, written chunk {}",
            before.elapsed(),
            i
        );
    }

    epd.display(it8951::WaveformMode::GrayscaleClearing16)
        .unwrap();

    println!("Elapsed time: {:.2?}", before.elapsed());
}

pub fn display_area_1bpp(epd: &mut it8951::IT8951<impl IT8951Interface, Run>) {
    let pattern = [
        0x0000, 0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777, 0x8888, 0x9999, 0xAAAA,
        0xBBBB, 0xCCCC, 0xDDDD, 0xEEEE, 0xFFFF,
    ];
    let height = 85;

    let before = Instant::now();

    for i in 0..pattern.len() { 
        let area = AreaImgInfo {
            area_x: 0,
            area_y: (i * height) as u16,
            area_w: 1872 as u16,
            area_h: height as u16,
        };

        epd.display_area_with_color(&area, WaveformMode::GrayscaleClearing16, pattern[i])
        .unwrap();
        println!("Elapsed time: {:.2?}", before.elapsed());
    }
    println!("Elapsed time: {:.2?}", before.elapsed());

}

pub fn clear_refresh(epd: &mut it8951::IT8951<impl IT8951Interface, Run>) {
    epd.clear(Gray4::WHITE).unwrap();
}

pub fn run(epd: &mut it8951::IT8951<impl IT8951Interface, Run>) {
    clear_refresh(epd);

    // 16 color grayscale
     display_color_palette_example(epd, MemoryConverterBitPerPixel::BitsPerPixel4);
     std::thread::sleep(std::time::Duration::from_secs(10));
     clear_refresh(epd);
    // 4 color grayscale
     display_color_palette_example(epd, MemoryConverterBitPerPixel::BitsPerPixel2);
     std::thread::sleep(std::time::Duration::from_secs(10));
     clear_refresh(epd);
    // 2 color grayscale
    display_area_1bpp(epd);
    std::thread::sleep(std::time::Duration::from_secs(10));
    clear_refresh(epd);
}
