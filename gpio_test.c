#include <stdio.h>
#include <unistd.h>
#include <time.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <sys/mman.h>


void setup();
void test_blink_led();
int pulse_clock(int gpio_pin, int divI, int divF, struct timespec *sleep_request);

volatile uint32_t *gpio;
volatile uint32_t *gpclk;

// For sleeping 10 microseconds
struct timespec remaining, request = {0,10000};


#define BLOCK_SIZE (4*1024)


/* Registers */
#define BCM2711_PERI_BASE 0xFE000000
#define GPIO_BASE (BCM2711_PERI_BASE + 0x200000)
#define GPCLK_BASE (BCM2711_PERI_BASE + 0x101000)

#define GPIO_OUT_CLEAR(g) *(gpio+(g)/10) &= ~(7<<((g)%10*3))	// Set FSELn to 000 w/o changing other FSEL registers
#define GPIO_OUT(g) *(gpio+(g)/10) |= 1<<((g)%10*3)		// Set FSELn to 001 w/o changing other FSEL registers (must do GPIO_OUT_CLEAR(g) first)

#define GPIO_SET *(gpio + 0x1c/4)	// Offset 0x1c
#define GPIO_CLR *(gpio + 0x28/4)	// Offset 0x28

/* GPICLK0 */
#define CLK_PSW 0x5A000000
#define CLK_OFF (0x70/4)
#define CLK_BUSY (*gpclk & (1<<7))
#define CLK_ENAB *gpclk |= CLK_PSW | 1<<4
#define CLK_DISAB *gpclk = (*gpclk & ~(1<<4)) | CLK_PSW



int main(int argc, char *argv[]) {
	// mmap
	setup(&gpio, GPIO_BASE);
	setup(&gpclk, GPCLK_BASE);
	gpclk += CLK_OFF;

	// GPIO pin to use (must have GPCLK0 as ALT0)
	int g = 4;

	// Clock frequency divisor
	int divI = 35;

	// Stop clock
	if (CLK_BUSY) {
		CLK_DISAB;
	}
	while (CLK_BUSY) nanosleep(&request, &remaining);

	// Set clock source to PLLD (750MHz source) and MASH to 1
	*gpclk = CLK_PSW | 6 | 1<<9;

	nanosleep(&request, &remaining);

	// Set clock frequency
	*(gpclk+1) = CLK_PSW | (divI << 12);

	nanosleep(&request, &remaining);

	// Start clock
	CLK_ENAB;

	nanosleep(&request, &remaining);

	struct timespec sleep_req = {0,1};
	for(int j=0; j<1000; j++) {
		for(int i=0; i<999; i+=10) {
			// Set clock frequency
			if (i == 10*(j/10) || i == 10*(j/10)+10) {
				*(gpclk+1) = CLK_PSW | ((divI+1) << 12) | 0;
			} else {
				*(gpclk+1) = CLK_PSW | (divI << 12) | i;
			}

			nanosleep(&sleep_req, NULL);
		}
	}

	CLK_DISAB;

	return 0;
}


int pulse_clock(int g, int divI, int divF, struct timespec *sleep_request) {
	// Clear GPIO pin function
	GPIO_OUT_CLEAR(g);

	// Stop clock
	if (CLK_BUSY) {
		CLK_DISAB;
	}
	while (CLK_BUSY) nanosleep(&request, &remaining);

	// Set clock frequency
	*(gpclk+1) = CLK_PSW | (divI << 12) | divF;

	nanosleep(&request, &remaining);

	// Set clock source to PLLD (measured as 750Mhz source)
	*gpclk = CLK_PSW | 6 | 1<<9;

	nanosleep(&request, &remaining);

	// Start clock
	CLK_ENAB;
	
	nanosleep(&request, &remaining);

	// Select ALT0 (GPCLK0 on pin 4)
	*gpio |= 1<<14;

	//puts("Clock started");

	if (sleep_request != NULL)
		nanosleep(sleep_request, NULL);

	// Stop clock
	CLK_DISAB;

	return 0;
}


// Blinks GPIO pin
void test_blink_led(int g) {
	GPIO_OUT_CLEAR(g);
	GPIO_OUT(g);

	struct timespec remaining, request = {1,0};

	for (;;) {
		GPIO_SET = 1<<g;
		nanosleep(&request, &remaining);
		GPIO_CLR = 1<<g;
		nanosleep(&request, &remaining);
	}
}


// Maps physical GPIO registers to virtual memory
void setup(volatile uint32_t **pntr, off_t offset) {
	// Open /dev/mem
	int fd;

	if ((fd = open("/dev/mem", O_RDWR|O_SYNC)) < 0) {
		printf("Could not open /dev/mem\n");
		exit(-1);
	}

	// mmap GPIO
	void *gpio_map = mmap(
			NULL,
			BLOCK_SIZE,
			PROT_READ|PROT_WRITE,
			MAP_SHARED,
			fd,
			offset
	);

	close(fd);

	if (gpio_map == MAP_FAILED) {
		printf("mmap error %d\n", (int)gpio_map);
		exit(-1);
	}

	*pntr = (volatile uint32_t *)gpio_map;
}
