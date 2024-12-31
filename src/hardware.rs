use std::{thread,time};
use std::error::Error;
use std::fs::OpenOptions;
use memmap::*;

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

// TODO:
// ClockTransmitter structure
//  Configures GPIO pin to clock (ALT0)
//  Functionality to transmit symbols from symbol bitarray
//  Extra: turn on arbitrary frequency
// Other settings:
//  Transmit band (base frequency)
//  Use n-th harmonic

pub struct GPIOController {
    gpio: *mut u32,
    gpclk: *mut u32,
}

impl GPIOController {
    pub fn new() -> GPIOController {
        // TODO: you may need to preserve the MmapMut struct,
        // otherwise the pointer might become dangling
        let gpio = get_mmap(GPIO_BASE).expect("mmap failed").as_mut_ptr() as *mut u32;
        let gpclk = get_mmap(GPCLK_BASE).expect("mmap failed").as_mut_ptr() as *mut u32;

        unsafe{ GPIOController { gpio, gpclk: gpclk.offset(CLK_OFFSET) } }
    }

    pub unsafe fn pulse_clock(&self, g: isize, divI: u32, divF: u32, ms: u64) {
        let sleep_dur = time::Duration::from_millis(ms);
        let sleep_ms = time::Duration::from_millis(1);

        gpio_out_clear!(self, g);

        if clk_busy!(self) {
            clk_disab!(self);
        }
        while clk_busy!(self) { thread::sleep(sleep_ms) };

        // Set clock frequency
        clk_div!(self, divI, divF);

        thread::sleep(sleep_ms);

        // Start clock
        clk_enab!(self);

        thread::sleep(sleep_ms);

        // Select ALT0 (GPCLK0 on pin 4)
        gpio_alt0!(self, g);

        thread::sleep(sleep_dur);

        // Stop clock
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
