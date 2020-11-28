# tms9918a_emu
Texas Instruments TMS9918A VDP emulator library for Rust

![TMS9918A](https://upload.wikimedia.org/wikipedia/commons/d/de/TMS9918A_02.jpg)

tms9918a_emu uses the [minifb](https://github.com/emoon/rust_minifb) crate to emulate a [Texas Instruments TMS9918A](https://en.wikipedia.org/wiki/Texas_Instruments_TMS9918) video display processor in a window.

High-level functions are provided as well as low-level functions, making it easy to control the VDP without needing to use the control and data ports.

This emulator is a work-in-progress and currently only supports the Graphics I and Text video modes, and sprites are unsupported in all modes. In its current state, this emulator is more of a TMS9918 (non-A variant) emulator.

## Example
This is a small [example program](examples/text/src/main.rs) which uses Text mode to display a hello world message, showing how to use the high-level functions:
![Text Mode example](examples/text/images/screenshot.png)

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
