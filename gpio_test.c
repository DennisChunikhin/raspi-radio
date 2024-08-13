#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <sys/mman.h>


void setup();

volatile uint32_t *gpio;


#define BLOCK_SIZE (4*1024)


/* Registers */
#define BCM2711_PERI_BASE 0xFE000000
#define GPIO_BASE (BCM2711_PERI_BASE + 0x200000)

#define GPIO_OUT_CLEAR(g) *(gpio+(g)/10) &= ~(7<<((g)%10*3))	// Set FSELn to 000 w/o changing other FSEL registers
#define GPIO_OUT(g) *(gpio+(g)/10) |= 1<<((g)%10*3)		// Set FSELn to 001 w/o changing other FSEL registers (must do GPIO_OUT_CLEAR(g) first)

#define GPIO_SET *(gpio + 7)	// Offset 0x1c
#define GPIO_CLR *(gpio + 10)	// Offset 0x28


int main(int argc, char *argv[]) {
	setup();

	GPIO_OUT_CLEAR(17);
	GPIO_OUT(17);

	for (int i=0; i<3; i++) {
		puts("On");
		GPIO_SET = 1<<17;

		sleep(1);

		puts("Off");
		GPIO_CLR = 1<<17;

		sleep(1);
	}

	return 0;
}


// Maps physical GPIO registers to virtual memory
void setup() {
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
			GPIO_BASE
	);

	close(fd);

	if (gpio_map == MAP_FAILED) {
		printf("mmap error %d\n", (int)gpio_map);
		exit(-1);
	}

	gpio = (volatile uint32_t *)gpio_map;
}
