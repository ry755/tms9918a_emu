// TMS9918A Text Mode example using low-level functions
// these low-level functions are equivalent to how a real TMS9918A would be programmed
// see the TMS9918A Data Manual for more details:
// http://www.bitsavers.org/components/ti/TMS9900/TMS9918A_TMS9928A_TMS9929A_Video_Display_Processors_Data_Manual_Nov82.pdf
// keep in mind that TI's line of TMS99xx processors uses reversed bit numbers
// in the Data Manual, bit 0 is actually bit 7, bit 1 is actually bit 6, etc.

// to write to a register:
// 1. write the data byte to the control port
//    the VDP stores this byte in the internal temporary data register
// 2. write the register number as a byte with bit 7 set (register number | 0x80) to the control port
//    setting bit 7 tells the VDP to write the stored data byte to a register

// to write to VRAM:
// 1. write the low address byte to the control port
//    the VDP stores this byte in the internal temporary data register
// 2. write the high address as a byte with bit 6 set (high address | 0x40) to the control port
//    setting bit 6 tells the VDP this is a memory write operation
//    the address is saved to the internal address pointer
// 3. write the VRAM data byte to the data port
//    the internal address pointer is automatically incremented after each write
//    additional VRAM data bytes can be sent to the data port without needing to send the address again

// to read from VRAM:
// 1. write the low address byte to the control port
//    the VDP stores this byte in the internal temporary data register
// 2. write the high address byte to the control port
//    clearing bit 6 tells the VDP this is a memory read operation
//    the address is saved to the internal address pointer
//    the data byte pointed to by the internal address pointer is immediately read into the read-ahead register
// 3. read a VRAM data byte from the data port
//    the internal address pointer is automatically incremented after each read
//    the data byte pointed to by the (now incremented) internal address pointer is immediately read into the read-ahead register
//    additional VRAM data bytes can be read from the data port without needing to send the address again

use tms9918a_emu::TMS9918A;

fn main() {
    // create a new TMS9918A VDP instance
    let mut vdp = TMS9918A::new("TMS9918A Text Mode Example (low-level)");

    // register 0: disable bitmap mode, disable external video input
    vdp.write_control_port(0b00000000);
    vdp.write_control_port(0x80);
    // register 1: enable video output, use Text mode
    vdp.write_control_port(0b11010000);
    vdp.write_control_port(0x81);
    // register 2: set the name table base address to 0x0000 (base address = multiplier * 0x0400)
    vdp.write_control_port(0x00);
    vdp.write_control_port(0x82);
    // register 4: set the pattern table base address to 0x0800 (base address = multiplier * 0x0800)
    vdp.write_control_port(0x01);
    vdp.write_control_port(0x84);
    // register 7: set foreground color to light red (0x9) and background color to black (0x1)
    vdp.write_control_port(0x91);
    vdp.write_control_port(0x87);

    // set VDP internal address pointer to the pattern table location
    vdp.write_control_port(0x00);
    vdp.write_control_port(0x48); // 0x08 | 0x40
    // fill pattern table with font data
    let font = include_bytes!("font.bin");
    for i in font.iter() {
        vdp.write_data_port(*i);
    }

    // set VDP internal address pointer to the name table location
    vdp.write_control_port(0x00);
    vdp.write_control_port(0x40); // 0x00 | 0x40
    // clear the screen
    // the video memory contains random data on startup, similar to how real memory works
    for _ in 0..960 { // 40x24 tiles = 960
        vdp.write_data_port(0);
    }

    // set VDP internal address pointer to the name table location + 40 to start on the second line of tiles
    vdp.write_control_port(0x28);
    vdp.write_control_port(0x40); // 0x00 | 0x40
    // write text by iterating over a string
    let text_string = "Hello, world!";
    for c in text_string.chars() {
        vdp.write_data_port(c as u8);
    }

    // update window contents
    while vdp.window.is_open() {
        vdp.update();
    }
}
