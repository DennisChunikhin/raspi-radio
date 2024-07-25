#include <stdio.h>
#include <unistd.h>
#include <time.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <sys/mman.h>


void setup();
void test_blink_led();

volatile uint32_t *gpio;
volatile uint32_t *gpclk;


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
#define CLK_OFF (0x70/4)
#define CLK_BUSY (*gpclk & (1<<7))
#define CLK_ENAB *gpclk |= 1<<4
#define CLK_DISAB *gpclk &= ~(1<<4)



int main(int argc, char *argv[]) {
	setup(&gpio, GPIO_BASE);
	setup(&gpclk, GPCLK_BASE);
	gpclk += CLK_OFF;

	int g = 4;

	GPIO_OUT_CLEAR(g);

	// Select ALT0 (GPCLK0 on pin 4)
	*(gpio) |= 1<<14;


	// Clear enable bit and set clock password
	*gpclk &= 0x00FFFFEF;
	*gpclk |= 0x5A000000;

	printf("%x\n", *gpclk);

	// Stop clock
	if (CLK_BUSY) {
		CLK_DISAB;
	}
	while (CLK_BUSY);

	// Set clock source to PLLD
	*gpclk &= ~(7);
	*gpclk |= 6;

	// Start clock
	CLK_ENAB;
	
	sleep(2);
	printf("%x\n", CLK_BUSY);
	printf("%x\n", *gpclk);
	sleep(2);

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
