// Peripheral Registers
const BCM2711_PERI_BASE: u32 = 0xFE000000;
const GPIO_BASE: u32 = BCM2711_PERI_BASE + 0x200000;
const GPCLK_BASE: u32 = BCM2711_PERI_BASE + 0x101000;

//#define GPIO_OUT_CLEAR(g) *(gpio+(g)/10) &= ~(7<<((g)%10*3))	// Set FSELn to 000 w/o changing other FSEL registers
//#define GPIO_OUT(g) *(gpio+(g)/10) |= 1<<((g)%10*3)		// Set FSELn to 001 w/o changing other FSEL registers (must do GPIO_OUT_CLEAR(g) first)
//#define GPIO_ALT0(g) *(gpio+(g)/10) |= 1<<((g)%10*3+2)		// Set FSELn to 100 w/o changing other FSEL registers (must do GPIO_OUT_CLEAR(g) first)

//#define GPIO_SET *(gpio + 0x1c/4)	// Offset 0x1c
//#define GPIO_CLR *(gpio + 0x28/4)	// Offset 0x28

/* GPICLK0 */
const CLK_PSW: u32 = 0x5A000000;
const CLK_OFFSET: u32 = 0x70/4;
//#define CLK_BUSY (*gpclk & (1<<7))
//#define CLK_ENAB *gpclk |= CLK_PSW | 1<<4
//#define CLK_DISAB *gpclk = (*gpclk & ~(1<<4)) | CLK_PSW

//#define CLK_DIV(divI, divF) *(gpclk+1) = CLK_PSW | ((divI) << 12) | (divF)


// TODO:
// ClockTransmitter structure
//  Configures GPIO pin to clock (ALT0)
//  Functionality to transmit symbols from symbol bitarray
//  Extra: turn on arbitrary frequency
// Other settings:
//  Transmit band (base frequency)
//  Use n-th harmonic
