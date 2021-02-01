// TMS9918A Text Mode example using high-level functions

use minifb::{Scale, ScaleMode, Window, WindowOptions};
use tms9918a_emu::{TMS9918A, VideoMode};

fn main() {
    // create a new TMS9918A VDP instance
    let mut vdp = TMS9918A::new();

    // create a new minifb window
    let mut window = Window::new(
        "TMS9918A Text Mode Example (high-level)",
        256,
        196,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::AspectRatioStretch,
            scale: Scale::X4,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    // limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // set the name table base address to 0x0000 (base address = multiplier * 0x0400)
    vdp.set_name_table_multiplier(0);

    // set the pattern table base address to 0x0800 (base address = multiplier * 0x0800)
    vdp.set_pattern_table_multiplier(1);

    // use Text Mode, 40x24 tiles at 6x8 pixels each
    vdp.set_video_mode(VideoMode::Text);

    // set foreground color to light red (0x9) and background color to black (0x1)
    vdp.write_register(7, 0x91);

    // fill pattern table with font data
    let font = include_bytes!("font.bin");
    vdp.fill_pattern_table(font, 0, font.len());

    // clear the screen
    // the video memory contains random data on startup, similar to how real memory works
    vdp.clear_name_table();

    // write text by iterating over a string
    let text_string = "Hello, world!";
    for (i, c) in text_string.chars().enumerate() {
        vdp.write_name_table(i+40, c as u8);
    }

    // enable video output (sets the blanking bit in register 1)
    vdp.enable_video(true);

    // update VDP framebuffer and window contents
    while window.is_open() {
        vdp.update();

        window.update_with_buffer(
            &vdp.frame,
            vdp.frame_width,
            vdp.frame_height,
        )
        .unwrap();
    }
}
