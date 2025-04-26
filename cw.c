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

	// GPIO clock pin
	int g_clk = 4;

	// GPIO input pin to use
	int g_in = 12;

	GPIO_CLR;
	GPIO_OUT_CLEAR(g);

	

	while(1) {
		if (*(gpio + 0x34/4) & 1<<g) {
			printf("On");
		}
		//printf("%d\n", *(gpio + 0x34/4) & 1<<g);
		nanosleep(&request, &remaining);
	}


	//nanosleep(&request, &remaining);

	return 0;
}

int turn_on_clock(int g, int divI, int divF) {
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

int turn_off_clock(int g) {

	return 0;
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
