//! Texas Instruments TMS9918A VDP emulator library

use minifb::{Scale, ScaleMode, Window, WindowOptions};
use rand::Rng;

// TMS9918A video modes
#[derive(PartialEq, Debug)]
pub enum VideoMode {
    /// Graphics I: 256x192 pixels, 32x24 tiles of 8x8 pixels each, 1 character set.
    /// 
    /// Each group of 8 tiles has the same 2-color limit.
    Gfx1,
    /// Graphics II: 256x192 pixels, 32x24 tiles of 8x8 pixels each, 3 character sets.
    /// 
    /// Each 8-pixel line of a tile has a 2-color limit.
    /// 
    /// This mode is not currently implemented.
    Gfx2,
    /// Text: 240x192 pixels, 40x24 tiles of 6x8 pixels each, 1 character set.
    /// 
    /// 2 colors for the whole screen, set by the contents of register 7.
    Text,
    /// Multicolor: 256x192 pixels, 64x48 virtual pixels
    /// 
    /// Each virtual pixel has their own color.
    /// 
    /// This mode is not currently implemented.
    Multicolor
}

pub struct TMS9918A {
    /// minifb window
    pub window: minifb::Window,
    /// Window framebuffer
    pub frame: Vec<u32>,
    // if true, clear framebuffer on next update
    frame_clear: bool,

    /// TMS9918A video memory, 16KB: contains name table, color table, and pattern table
    /// 
    /// Initialized with random values to simulate real memory behavior.
    pub vdp_ram: Vec<u8>,
    // offsets into VDP_RAM for the various tables
    vdp_name_table_offset: u16,
    vdp_color_table_offset: u16,
    vdp_pattern_table_offset: u16,
    // TMS9918A registers
    vdp_register: Vec<u8>,
    // TMS9918A video mode
    vdp_mode: VideoMode,
    // temporary data register
    vdp_temp_data: u8,
    // current VDP memory address pointer
    vdp_addr_pointer: u16,
    // true after the first command byte was sent
    vdp_first_byte_saved_flag: bool,
    // byte at current memory address pointer
    vdp_read_ahead: u8
}

impl TMS9918A {
    /// Create a new TMS9918A state and a window with specified title
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::TMS9918A;
    /// # fn main() {
    /// let mut vdp = TMS9918A::new("Window Title");
    /// # }
    /// ```
    pub fn new(title: &str) -> Self {
        let mut window = Window::new(
            title,
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

        let frame: Vec<u32> = vec![0; 256 * 196];

        TMS9918A {
            window: window,
            frame: frame,
            frame_clear: false,
            vdp_ram: (0..16*1024).map(|_| rand::thread_rng().gen()).collect(),
            vdp_name_table_offset: 0,
            vdp_color_table_offset: 0,
            vdp_pattern_table_offset: 0,
            vdp_register: vec![0; 8],
            vdp_mode: VideoMode::Gfx1,
            vdp_temp_data: 0,
            vdp_addr_pointer: 0,
            vdp_first_byte_saved_flag: false,
            vdp_read_ahead: 0
        }
    }

    /// Update the framebuffer and window from the TMS9918A video memory contents
    ///
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::TMS9918A;
    /// # fn main() {
    /// let mut vdp = TMS9918A::new("Window Title");
    /// 
    /// while vdp.window.is_open() {
    ///     vdp.update();
    /// }
    /// # }
    /// ```
    pub fn update(&mut self) {
        let colors: [u32; 16] = [0x000000, 0x000000, 0x21C942, 0x5EDC78,
                                0x5455ED, 0x7D75FC, 0xD3524D, 0x43EBF6,
                                0xFD5554, 0xFF7978, 0xD3C153, 0xE5CE80,
                                0x21B03C, 0xC95BBA, 0xCCCCCC, 0xFFFFFF];

        if self.frame_clear {
            for i in self.frame.iter_mut() {
                *i = 0;
            }
            self.frame_clear = false;
        }

        // check blanking bit
        if self.vdp_register[1] & (1 << 6) != 0 {
            // blanking bit is set, screen is enabled
            // Graphics I
            if self.vdp_mode == VideoMode::Gfx1 {
                let frame_width = 256;
                let frame_height = 196;
                for tile_y in 0..24 {
                    for tile_x in 0..32 {
                        let name_entry = self.vdp_ram[self.vdp_name_table_offset as usize + (tile_y * 32) + tile_x];
                        let color_entry = name_entry / 8;
                        let color_byte = self.vdp_ram[self.vdp_color_table_offset as usize + color_entry as usize];
                        let foreground_color = colors[color_byte as usize >> 4 & 0x0F];
                        let background_color = colors[color_byte as usize & 0x0F];
                        for pattern_byte in 0..8 {
                            let offset = self.vdp_pattern_table_offset as usize + (name_entry as usize * 8) + (pattern_byte);
                            let pattern = self.vdp_ram[offset];
                            let pattern_bit_indexes = 0..8;
                            let frame_bit_indexes = pattern_bit_indexes.clone().rev();
                            for (pattern_bit, frame_bit) in pattern_bit_indexes.zip(frame_bit_indexes) {
                                let pixel = if pattern & (1 << pattern_bit) != 0 { foreground_color } else { background_color };
                                let frame_offset = (tile_x * 8) + (tile_y * 8 * frame_width) + (pattern_byte * frame_width) + frame_bit;
                                self.frame[frame_offset] = pixel;
                            }
                        }
                    }
                }

                // update window
                self.window.update_with_buffer(&self.frame, frame_width, frame_height).unwrap();
            }

            // Text
            if self.vdp_mode == VideoMode::Text {
                let frame_width = 240;
                let frame_height = 196;
                for tile_y in 0..24 {
                    for tile_x in 0..40 {
                        let name_entry = self.vdp_ram[self.vdp_name_table_offset as usize + (tile_y * 40) + tile_x];
                        let color_byte = self.vdp_register[7];
                        let foreground_color = colors[color_byte as usize >> 4 & 0x0F];
                        let background_color = colors[color_byte as usize & 0x0F];
                        for pattern_byte in 0..8 {
                            let offset = self.vdp_pattern_table_offset as usize + (name_entry as usize * 8) + (pattern_byte);
                            let pattern = self.vdp_ram[offset];
                            let pattern_bit_indexes = 2..8;
                            let frame_bit_indexes = (0..6).rev();
                            for (pattern_bit, frame_bit) in pattern_bit_indexes.zip(frame_bit_indexes) {
                                let pixel = if pattern & (1 << pattern_bit) != 0 { foreground_color } else { background_color };
                                let frame_offset = (tile_x * 6) + (tile_y * 8 * frame_width) + (pattern_byte * frame_width) + frame_bit;
                                self.frame[frame_offset] = pixel;
                            }
                        }
                    }
                }

                // update window
                self.window.update_with_buffer(&self.frame, frame_width, frame_height).unwrap();
            }
        } else {
            // blanking bit is clear, screen is disabled
            for i in self.frame.iter_mut() {
                *i = 0;
            }
            // update window
            self.window.update_with_buffer(&self.frame, 256, 196).unwrap();
        }
    }

    /// Enable or disable the video display by setting or clearing the blanking bit in register 1
    /// 
    /// The video display is disabled by default due to registers 0 and 1 being cleared on reset,
    /// resulting in a black screen similar to the behavior of a real TMS9918A.
    #[inline]
    pub fn enable_video(&mut self, enable: bool) {
        if enable {
            self.vdp_register[1] |= 1 << 6;
        } else {
            self.vdp_register[1] &= !(1 << 6);
        }
    }

    /// Reset VDP to initial state without modifying video memory
    pub fn warm_reset(&mut self) {
        self.write_register(0, 0);
        self.write_register(1, 0);
        self.vdp_temp_data = 0;
        self.vdp_addr_pointer = 0;
        self.vdp_read_ahead = 0;
        self.vdp_first_byte_saved_flag = false;
    }

    /// Reset VDP to initial state and randomize video memory contents
    pub fn cold_reset(&mut self) {
        self.warm_reset();
        for i in self.vdp_ram.iter_mut() {
            *i = rand::thread_rng().gen();
        }
    }

    /// Set TMS9918A video mode
    /// 
    /// Valid video modes are Text, Graphics I, Graphics II, and Multicolor.
    /// 
    /// Graphics II and Multicolor modes are not currently implemented, and sprites are not currently implemented in any mode.
    /// 
    /// Undocumented modes (combining video modes by setting the bitmap enable bit in register 0) are not supported.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::{TMS9918A, VideoMode};
    /// # fn main() {
    /// # let mut vdp = TMS9918A::new("Window Title");
    /// // use text mode
    /// vdp.set_video_mode(VideoMode::Text);
    /// # }
    /// ```
    pub fn set_video_mode(&mut self, mode: VideoMode) {
        match mode {
            VideoMode::Gfx1 => {
                let r0 = self.vdp_register[0] & !(0b01000000);
                let r1 = self.vdp_register[1] & !(0b00011000);
                self.write_register(0, r0);
                self.write_register(1, r1);
            }
            VideoMode::Gfx2 => {
                let r0 = self.vdp_register[0] | (0b01000000);
                let r1 = self.vdp_register[1] & !(0b00011000);
                self.write_register(0, r0);
                self.write_register(1, r1);
            }
            VideoMode::Multicolor => {
                let r0 = self.vdp_register[0] & !(0b01000000);
                let r1 = (self.vdp_register[1] & !(0b00010000)) | 0b00001000;
                self.write_register(0, r0);
                self.write_register(1, r1);
            }
            VideoMode::Text => {
                let r0 = self.vdp_register[0] & !(0b01000000);
                let r1 = (self.vdp_register[1] & !(0b00001000)) | 0b00010000;
                self.write_register(0, r0);
                self.write_register(1, r1);
            }
        }
    }

    /// Write register value
    pub fn write_register(&mut self, register: u8, data: u8) {
        // write register value
        self.vdp_register[register as usize] = data;

        // write offset values
        self.vdp_name_table_offset = self.vdp_register[2] as u16 * 0x0400;
        self.vdp_color_table_offset = self.vdp_register[3] as u16 * 0x0040;
        self.vdp_pattern_table_offset = self.vdp_register[4] as u16 * 0x0800;

        // write video mode
        if register == 0 || register == 1 {
            // register 0 bit 6: enable a bitmap graphics mode
            let m3 = if self.vdp_register[0] & (1 << 6) != 0 { true } else { false };
            // register 1 bit 3: enable text mode
            let m1 = if self.vdp_register[1] & (1 << 4) != 0 { true } else { false };
            // register 0 bit 6: enable multicolor mode
            let m2 = if self.vdp_register[1] & (1 << 3) != 0 { true } else { false };

            match (m1, m2, m3) {
                (false, false, false) => {
                    self.vdp_mode = VideoMode::Gfx1;
                    // clear framebuffer on next update
                    self.frame_clear = true;
                }
                (false, false, true) => {
                    self.vdp_mode = VideoMode::Gfx2;
                    // clear framebuffer on next update
                    self.frame_clear = true;
                }
                (false, true, false) => {
                    self.vdp_mode = VideoMode::Multicolor;
                    // clear framebuffer on next update
                    self.frame_clear = true;
                }
                (true, false, false) => {
                    self.vdp_mode = VideoMode::Text;
                    // clear framebuffer on next update
                    self.frame_clear = true;
                }
                _ => panic!("unimplemented video mode combination: M1: {}, M2: {}, M3: {}", m1, m2, m3)
            }

            //println!("set graphics mode: {:?}", self.vdp_mode);
        }
    }

    /// Write memory contents
    #[inline]
    pub fn write_ram(&mut self, address: usize, data: u8) {
        self.vdp_ram[address] = data;
    }

    /// Read memory contents
    #[inline]
    pub fn read_ram(&mut self, address: usize) -> u8 {
        let data = self.vdp_ram[address];
        data
    }

    /// Set the name table address multiplier in register 2
    /// 
    /// Name table base address is equal to multiplier * 0x0400.
    /// 
    /// This function is equivalent to setting register 2 directly.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::{TMS9918A, VideoMode};
    /// # fn main() {
    /// # let mut vdp = TMS9918A::new("Window Title");
    /// // set name table base address to 0x0400
    /// vdp.set_name_table_multiplier(1);
    /// # }
    /// ```
    #[inline]
    pub fn set_name_table_multiplier(&mut self, mut multiplier: u8) {
        if multiplier > 15 {
            multiplier = 15;
        }
        self.write_register(2, multiplier);
    }

    /// Fill name table contents from an array
    /// 
    /// Name table offset register must be set first.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::{TMS9918A, VideoMode};
    /// # fn main() {
    /// # let mut vdp = TMS9918A::new("Window Title");
    /// // fill the first 5 name table entries
    /// let name_table: [u8; 5] = [1, 2, 3, 4, 5];
    /// vdp.fill_name_table(&name_table, 0, name_table.len());
    /// # }
    /// ```
    #[inline]
    pub fn fill_name_table(&mut self, array: &[u8], offset: usize, length: usize) {
        for i in offset..offset+length {
            self.write_name_table(i, array[i]);
        }
    }

    /// Clear the screen by zeroing the name table
    /// 
    /// Name table offset register must be set first.
    #[inline]
    pub fn clear_name_table(&mut self) {
        if self.vdp_mode == VideoMode::Text {
            // text mode's name table is 960 bytes
            for i in 0..960 {
                self.write_name_table(i, 0);
            }
        } else {
            // all other modes' name tables are 768 bytes
            for i in 0..768 {
                self.write_name_table(i, 0);
            }
        }
    }

    /// Write name table contents
    /// 
    /// Name table offset register must be set first.
    #[inline]
    pub fn write_name_table(&mut self, offset: usize, data: u8) {
        self.vdp_ram[self.vdp_name_table_offset as usize + offset] = data;
    }

    /// Read name table contents
    /// 
    /// Name table offset register must be set first.
    #[inline]
    pub fn read_name_table(&self, offset: usize) -> u8 {
        self.vdp_ram[self.vdp_name_table_offset as usize + offset]
    }

    /// Set the color table address multiplier in register 3
    /// 
    /// Color table base address is equal to multiplier * 0x0040.
    /// 
    /// This function is equivalent to setting register 3 directly.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::{TMS9918A, VideoMode};
    /// # fn main() {
    /// # let mut vdp = TMS9918A::new("Window Title");
    /// // set color table base address to 0x0040
    /// vdp.set_color_table_multiplier(1);
    /// # }
    /// ```
    #[inline]
    pub fn set_color_table_multiplier(&mut self, multiplier: u8) {
        self.write_register(3, multiplier);
    }

    /// Fill color table contents from an array
    /// 
    /// Color table offset register must be set first.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::{TMS9918A, VideoMode};
    /// # fn main() {
    /// # let mut vdp = TMS9918A::new("Window Title");
    /// // fill the first 5 color table entries
    /// // black on white, white on black, light blue on dark blue, light red on cyan, black on gray
    /// let color_table: [u8; 5] = [0x1F, 0xF1, 0x54, 0x97, 0x1E];
    /// vdp.fill_color_table(&color_table, 0, color_table.len());
    /// # }
    /// ```
    #[inline]
    pub fn fill_color_table(&mut self, array: &[u8], offset: usize, length: usize) {
        for i in offset..offset+length {
            self.write_color_table(i, array[i]);
        }
    }

    /// Write color table contents
    /// 
    /// Color table offset register must be set first.
    #[inline]
    pub fn write_color_table(&mut self, offset: usize, data: u8) {
        self.vdp_ram[self.vdp_color_table_offset as usize + offset] = data;
    }

    /// Read color table contents
    /// 
    /// Color table offset register must be set first.
    #[inline]
    pub fn read_color_table(&self, offset: usize) -> u8 {
        self.vdp_ram[self.vdp_color_table_offset as usize + offset]
    }

    /// Set the pattern table address multiplier in register 4
    /// 
    /// Pattern table base address is equal to multiplier * 0x0800.
    /// 
    /// This function is equivalent to setting register 4 directly.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::{TMS9918A, VideoMode};
    /// # fn main() {
    /// # let mut vdp = TMS9918A::new("Window Title");
    /// // set pattern table base address to 0x0800
    /// vdp.set_pattern_table_multiplier(1);
    /// # }
    /// ```
    #[inline]
    pub fn set_pattern_table_multiplier(&mut self, mut multiplier: u8) {
        if multiplier > 7 {
            multiplier = 7;
        }
        self.write_register(4, multiplier);
    }

    /// Fill pattern table contents from an array
    /// 
    /// Pattern table offset register must be set first.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use tms9918a_emu::{TMS9918A, VideoMode};
    /// # fn main() {
    /// # let mut vdp = TMS9918A::new("Window Title");
    /// // fill 8 pattern table entries starting at offset 8
    /// // 8 pattern table entries make one tile
    /// // this makes tile 1 a nice smiley face :)
    /// let pattern_table: [u8; 8] = [0b00000000,
    ///                               0b00100100,
    ///                               0b00100100,
    ///                               0b00100100,
    ///                               0b00000000,
    ///                               0b01000010,
    ///                               0b01111110,
    ///                               0b00000000];
    /// vdp.fill_pattern_table(&pattern_table, 8, pattern_table.len());
    /// # }
    /// ```
    #[inline]
    pub fn fill_pattern_table(&mut self, array: &[u8], offset: usize, length: usize) {
        for i in offset..offset+length {
            self.write_pattern_table(i, array[i]);
        }
    }

    /// Write pattern table contents
    /// 
    /// Pattern table offset register must be set first.
    #[inline]
    pub fn write_pattern_table(&mut self, offset: usize, data: u8) {
        self.vdp_ram[self.vdp_pattern_table_offset as usize + offset] = data;
    }

    /// Read pattern table contents
    /// 
    /// Pattern table offset register must be set first.
    #[inline]
    pub fn read_pattern_table(&self, offset: usize) -> u8 {
        self.vdp_ram[self.vdp_pattern_table_offset as usize + offset]
    }

    /// Write to the TMS9918A control port
    /// 
    /// This expects standard TMS9918A commands,
    /// see the [TMS9918A Data Manual](http://www.bitsavers.org/components/ti/TMS9900/TMS9918A_TMS9928A_TMS9929A_Video_Display_Processors_Data_Manual_Nov82.pdf) for details.
    pub fn write_control_port(&mut self, data: u8) {
        if self.vdp_first_byte_saved_flag == false {
            // this is the first byte of the command, save it
            self.vdp_temp_data = data;
            self.vdp_first_byte_saved_flag = true;
        } else {
            // this is the second byte of the command, execute the command
            if (data & (1 << 7) != 0) && (data & (1 << 6) == 0) {
                // bit 7 is set and bit 6 is clear, this is a write a register
                let register = data & 0b00000111;
                let register_value = self.vdp_temp_data;
                self.write_register(register, register_value);
                self.vdp_first_byte_saved_flag = false;
                return;
            }
            if (data & (1 << 7) == 0) && (data & (1 << 6) != 0) {
                // bit 7 is clear and bit 6 is set, this is a write to memory
                let address = ((data as u16 & 0b00111111) << 8) | (self.vdp_temp_data as u16 & 0x00FF);
                self.vdp_addr_pointer = address;
                self.vdp_first_byte_saved_flag = false;
                return;
            }
            if (data & (1 << 7) == 0) && (data & (1 << 6) == 0) {
                // bit 7 is clear and bit 6 is clear, this is a read from memory
                let address = ((data as u16 & 0b00111111) << 8) | (self.vdp_temp_data as u16 & 0x00FF);
                self.vdp_addr_pointer = address;
                self.vdp_read_ahead = self.read_ram(address as usize);
                self.vdp_first_byte_saved_flag = false;
                return;
            }
        }
    }

    /// Write to the TMS9918A data port
    /// 
    /// This follows the standard TMS9918A behavior of incrementing the addr. pointer after each write,
    /// see the [TMS9918A Data Manual](http://www.bitsavers.org/components/ti/TMS9900/TMS9918A_TMS9928A_TMS9929A_Video_Display_Processors_Data_Manual_Nov82.pdf) for details.
    pub fn write_data_port(&mut self, data: u8) {
        self.vdp_first_byte_saved_flag = false;
        let address = self.vdp_addr_pointer;
        self.write_ram(address as usize, data);
        self.vdp_addr_pointer += 1;
    }

    /// Read from the TMS9918A data port
    /// 
    /// This follows the standard TMS9918A behavior of incrementing the addr. pointer after each read,
    /// see the [TMS9918A Data Manual](http://www.bitsavers.org/components/ti/TMS9900/TMS9918A_TMS9928A_TMS9929A_Video_Display_Processors_Data_Manual_Nov82.pdf) for details.
    pub fn read_data_port(&mut self) -> u8 {
        self.vdp_first_byte_saved_flag = false;
        let data = self.vdp_read_ahead;
        self.vdp_addr_pointer += 1;
        self.vdp_read_ahead = self.read_ram(self.vdp_addr_pointer as usize);
        data
    }
}