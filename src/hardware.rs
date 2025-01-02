use std::{thread,time};
use std::error::Error;
use std::fs::{OpenOptions,File};
use std::io::{BufReader,BufRead};
use memmap::*;

use crate::wspr::SymbolBits;
use bitvec::prelude::bits;

// This file implements a simplified GPIO controller for a raspberry pi 4
// See the datasheet: https://datasheets.raspberrypi.com/bcm2711/bcm2711-peripherals.pdf
// In theory this should work for other models of the raspberry pi as long as the bus base address
// (BCM2711_PERI_BASE) is changed accordingly

const BLOCK_SIZE: usize = 4*1024;

// Peripheral Registers
const BCM2711_PERI_BASE: u64 = 0xFE000000;
const GPIO_BASE: u64 = BCM2711_PERI_BASE + 0x200000;
const GPCLK_BASE: u64 = BCM2711_PERI_BASE + 0x101000;

macro_rules! get_gpio {
    ($self:expr, $g:expr) => { $self.gpio.offset(($g)/10) }
}

// Set FSELn to 000 w/o changing other FSEL registers
macro_rules! gpio_out_clear {
    ($self:expr, $g:expr) => {
        get_gpio!($self, $g).write_volatile(
            get_gpio!($self, $g).read_volatile() & !(7<<(($g)%10*3))
        )
    }
}
// Set FSELn to 001 w/o changing other FSEL registers (must do gpio_out_clear!(g) first)
macro_rules! gpio_out {
    ($self:expr, $g:expr) => {
        get_gpio!($self, $g).write_volatile(
            get_gpio!($self, $g).read_volatile() | 1<<(($g)%10*3)
        )
    }
}
// Set FSELn to 100 w/o changing other FSEL registers (must do gpio_out_clear!(g) first)
macro_rules! gpio_alt0 {
    ($self:expr, $g:expr) => {
        get_gpio!($self, $g).write_volatile(
            get_gpio!($self, $g).read_volatile() | 1<<(($g)%10*3+2)
        )
    }
}

//#define GPIO_SET *(gpio + 0x1c/4)	// Offset 0x1c
//#define GPIO_CLR *(gpio + 0x28/4)	// Offset 0x28

/* GPICLK0 */
const CLK_PSW: u32 = 0x5A000000;
const CLK_OFFSET: isize = 0x70/4;

// Check if clock is busy
macro_rules! clk_busy {
    ($self:expr) => {
        ($self.gpclk.read_volatile() & (1<<7)) == (1<<7)
    }
}

// Enable clock
macro_rules! clk_enab {
    ($self:expr) => {
        $self.gpclk.write_volatile(
            $self.gpclk.read_volatile() | CLK_PSW | 1<<4
        )
    }
}

// Disable clock
macro_rules! clk_disab {
    ($self:expr) => {
        $self.gpclk.write_volatile(
            ($self.gpclk.read_volatile() & !(1<<4)) | CLK_PSW
        )
    }
}

// Set clock divisors
macro_rules! clk_div {
    ($self:expr, $divI:expr, $divF:expr) => {
        $self.gpclk.offset(1).write_volatile( CLK_PSW | (($divI)<<12) | ($divF) )
    }
}

macro_rules! div_from_freq {
    ($freq:expr, $base:expr) => {{
        let div: f64 = $base/$freq;
        let divI = div.trunc();
        (divI as u32, ((div-divI)*1024.0).round() as u32)
    }}
}


const sleep_ms: time::Duration = time::Duration::from_millis(1);

// WSPR FSK encoding parameters
const FREQ_SHIFT: f64 = 12000f64/8192f64; // Hz

// TODO:
// ClockTransmitter structure
//  Configures GPIO pin to clock (ALT0)
//  Functionality to transmit symbols from symbol bitarray
//  Extra: turn on arbitrary frequency
// Other settings:
//  Transmit band (base frequency)
//  Use n-th harmonic

pub struct GPIOController {
    gpio_mmap: MmapMut,
    gpclk_mmap: MmapMut,
    gpio: *mut u32,
    gpclk: *mut u32,
}

impl GPIOController {
    pub fn new() -> GPIOController {
        // Must preserve Mmap object so that pointer doesn't dangle
        let mut gpio_mmap = get_mmap(GPIO_BASE).expect("mmap failed");
        let mut gpclk_mmap = get_mmap(GPCLK_BASE).expect("mmap failed");

        let mut gpio = gpio_mmap.as_mut_ptr() as *mut u32;
        let mut gpclk = gpclk_mmap.as_mut_ptr() as *mut u32;

        unsafe{ GPIOController { gpio, gpclk: gpclk.offset(CLK_OFFSET), gpio_mmap, gpclk_mmap } }
    }

    pub unsafe fn clock_busy(&self) -> bool {
        clk_busy!(self)
    }

    // Not marked public since I want all public functions to turn off clock after use
    // g must be a pin that has CLK0 as its ALT0 function
    unsafe fn turn_on_clock(&self, g: isize, divI: u32, divF: u32) {
        // Set pin output to ALT0 (which is CLK0 on pin 4)
        gpio_out_clear!(self, g);
        gpio_alt0!(self, g);

        // Turn of clock (before modifying clock settings)
        if clk_busy!(self) {
            clk_disab!(self);
        }
        while clk_busy!(self) { thread::sleep(sleep_ms) };

        // Set clock source to PLLD (750Mhz source) and MASH to 1
        self.gpclk.write_volatile( CLK_PSW | 6 | 1<<9 );

        thread::sleep(sleep_ms);

        // Set clock frequency
        clk_div!(self, divI, divF);

        thread::sleep(sleep_ms);

        // Turn on clock
        clk_enab!(self);
    }

    pub unsafe fn test_clock(&self, g: isize, divI: u32) {
        let sleep_dur = time::Duration::from_nanos(1);

        self.turn_on_clock(g, divI, 0);

        thread::sleep(sleep_ms);

        for j in 0..1000 {
            for i in (0..999).step_by(10) {
                // Set clock frequency
                if i==10*(j/10) || i==10*(j/10)+10 {
                    clk_div!(self, divI+1, 0);
                } else {
                    clk_div!(self, divI, i);
                }

                thread::sleep(sleep_dur);
            }
        }

        clk_disab!(self);
    }

    // Turns clock on for a given time
    pub unsafe fn pulse_clock(&self, g: isize, divI: u32, divF: u32, ms: u64) {
        let sleep_dur = time::Duration::from_millis(ms);

        self.turn_on_clock(g, divI, divF);

        thread::sleep(sleep_dur);

        // Stop clock
        clk_disab!(self);
    }

    // TODO: Write script to read in file image array, and test this function
    pub unsafe fn broadcast_image(&self, pos_array: *const i32, wait_array: *const i32, data_len: isize, repeats: u32) {
        let sleep_dur = time::Duration::from_nanos(1);

        let g = 4;
        let divI = 35;

        self.turn_on_clock(g, divI, 0);

        thread::sleep(sleep_ms);

        // Broadcast the image
        // Pointer arithmetic implementation copied from C
        let mut pos_pntr = pos_array;
        let mut wait_pntr = wait_array;
        while pos_pntr.offset_from(pos_array) != data_len {
            let pos_snapshot = pos_pntr;
            let wait_snapshot = wait_pntr;

            for i in 0..repeats {
                pos_pntr = pos_snapshot;
                wait_pntr = wait_snapshot;

                while *pos_pntr != -1 {
                    // Set clock frequency
                    clk_div!(self, divI, *pos_pntr as u32);

                    for j in 0..*wait_pntr {
                        thread::sleep(sleep_ms);
                    }

                    pos_pntr = pos_pntr.offset(1);
                    wait_pntr = wait_pntr.offset(1);
                }

                clk_div!(self, divI+1, 0);
                for j in 0..*wait_pntr {
                    thread::sleep(sleep_dur);
                }
            }
            
            pos_pntr = pos_pntr.offset(1);
            wait_pntr = wait_pntr.offset(1);
        }
        
        // Turn clock off
        clk_disab!(self);
    }

    pub unsafe fn transmit_wspr(&self, bits: SymbolBits, base_freq: f64) {
        // TODO: add checks for proper (safe) frequency ranges
        // Good idea to add these to turn_on_clock method
        let tone_len = time::Duration::from_secs_f64(FREQ_SHIFT.recip());

        // Symbol frequencies
        let base_freq = 750.0;
        let (divI_0, divF_0) = div_from_freq!(base_freq, base_freq);
        let (divI_1, divF_1) = div_from_freq!(base_freq+FREQ_SHIFT, base_freq);
        let (divI_2, divF_2) = div_from_freq!(base_freq+FREQ_SHIFT*2., base_freq);
        let (divI_3, divF_3) = div_from_freq!(base_freq+FREQ_SHIFT*3., base_freq);

        // TODO: Timing (synchronization)
        self.turn_on_clock(4, divI_0, divF_0);

        // Perhaps not the best way to iterate thru the symbol bits, but fast enough
        for symbol in bits.chunks(2) {
            match symbol.iter().by_vals().enumerate()
                    .fold(0, |acc, (i, b)| acc + if b {1+i} else {0}) {
                0 => clk_div!(self, divI_0, divF_0),
                1 => clk_div!(self, divI_1, divF_1),
                2 => clk_div!(self, divI_2, divF_2),
                3 => clk_div!(self, divI_3, divF_3),
                _ => (),
            };

            thread::sleep(tone_len);
        }

        clk_disab!(self);
    }
}

pub fn get_mmap(offset: u64) -> Result<MmapMut, Box<dyn Error>> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("/dev/mem")?;

    unsafe {
        Ok(MmapOptions::new()
            .offset(offset)
            .len(BLOCK_SIZE)
            .map_mut(&file)?)
    }
}

pub fn read_image_file(path: &str) -> (isize, isize, Vec<i32>, Vec<i32>) {
    let file = File::open(path).expect("Could not open file.");
    let mut reader = BufReader::new(file).lines();

    let data_len: isize = reader.next().unwrap().expect("Malformed file")
        .parse().expect("Malformed file");
    let wait_per_row: isize = reader.next().unwrap().expect("Malformed file")
        .parse().expect("Malformed file");

    let vec: Vec<i32> = reader
        .map(|line| line.unwrap().parse::<i32>().unwrap())
        .collect();

    let data_vec = vec.iter()
        .step_by(2)
        .copied()
        .collect::<Vec<i32>>();
    let wait_vec = vec.iter()
        .skip(1)
        .step_by(2)
        .copied()
        .collect::<Vec<i32>>();

    (data_len, wait_per_row, data_vec, wait_vec)
}
