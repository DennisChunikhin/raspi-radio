#include <stdio.h>
#include <unistd.h>
#include <time.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <sys/mman.h>


void setup();
void broadcast_image(const int *pos_array, const int *wait_array, int data_len, int repeats);

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
#define GPIO_ALT0(g) *(gpio+(g)/10) |= 1<<((g)%10*3+2)		// Set FSELn to 100 w/o changing other FSEL registers (must do GPIO_OUT_CLEAR(g) first)

#define GPIO_SET *(gpio + 0x1c/4)	// Offset 0x1c
#define GPIO_CLR *(gpio + 0x28/4)	// Offset 0x28

/* GPICLK0 */
#define CLK_PSW 0x5A000000
#define CLK_OFFSET (0x70/4)
#define CLK_BUSY (*gpclk & (1<<7))
#define CLK_ENAB *gpclk |= CLK_PSW | 1<<4
#define CLK_DISAB *gpclk = (*gpclk & ~(1<<4)) | CLK_PSW

#define CLK_DIV(divI, divF) *(gpclk+1) = CLK_PSW | ((divI) << 12) | (divF)



int main(int argc, char *argv[]) {
	FILE *fptr;
	int32_t data_len, wait_per_row;
	int *pos_array, *wait_array, repeats=1;

	if (argc < 2) {
		fprintf(stderr, "Not enough arguments\n");
		exit(EXIT_FAILURE);
	}
	
	if (argc >= 3) {
		repeats = atoi(argv[2]);
		repeats = repeats > 0 ? repeats : 1;
	}


	fptr = fopen(argv[1], "r");

	if (fptr == NULL) {
		fprintf(stderr, "Cannot open file \"%s\"\n", argv[1]);
		exit(EXIT_FAILURE);
	}

	if (fscanf(fptr, "%d", &data_len) == EOF) {
		fprintf(stderr, "Malformed data file\n");
		exit(EXIT_FAILURE);
	}
	if (fscanf(fptr, "%d", &wait_per_row) == EOF) {
		fprintf(stderr, "Malformed data file\n");
		exit(EXIT_FAILURE);
	}

	// Allocate arrays to store image data
	pos_array = (int *)malloc(sizeof(int) * data_len);
	if (pos_array == NULL) {
		fprintf(stderr, "Out of memory!\n");
		exit(EXIT_FAILURE);
	}
	wait_array = (int *)malloc(sizeof(int) * data_len);
	if (wait_array == NULL) {
		fprintf(stderr, "Out of memory!\n");
		exit(EXIT_FAILURE);
	}

	// Read data from file
	int *pos_ptr=pos_array;
	int *wait_ptr=wait_array;
	for (; pos_ptr != pos_array+data_len; pos_ptr++, wait_ptr++) {
		if (fscanf(fptr, "%d", pos_ptr) == EOF) {
			fprintf(stderr, "Malformed data file\n");
			exit(EXIT_FAILURE);
		}
		if (fscanf(fptr, "%d", wait_ptr) == EOF) {
			fprintf(stderr, "Malformed data file\n");
			exit(EXIT_FAILURE);
		}
	}

	fclose(fptr);


	// Radio!
	broadcast_image(pos_array, wait_array, data_len, repeats);


	// Free everything
	free(pos_array);
	free(wait_array);

	return 0;
}

void broadcast_image(const int *pos_array, const int *wait_array, int data_len, int repeats) {
	// mmap
	setup(&gpio, GPIO_BASE);
	setup(&gpclk, GPCLK_BASE);
	gpclk += CLK_OFFSET;

	// GPIO pin to use (must have GPCLK0 as ALT0)
	int g = 4;

	// Set GPIO pin to ALT0 (GPCLK0 for GPIO pin 4)
	GPIO_OUT_CLEAR(g);
	GPIO_ALT0(g);

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
	CLK_DIV(divI, 0);

	nanosleep(&request, &remaining);

	// Start clock
	CLK_ENAB;

	nanosleep(&request, &remaining);

	// Broadcast the image
	struct timespec sleep_req = {0,1};
	const int *pos_ptr=pos_array, *wait_ptr=wait_array, *pos_snapshot, *wait_snapshot;
	while (pos_ptr != pos_array+data_len) {
		pos_snapshot=pos_ptr;
		wait_snapshot=wait_ptr;

		// Draw row (multiple times)
		for(int i=0; i<repeats; i++) {
			pos_ptr = pos_snapshot;
			wait_ptr = wait_snapshot;

			for (; *pos_ptr != -1; pos_ptr++, wait_ptr++) {
				// Set clock frequency
				CLK_DIV(divI, *pos_ptr);
				
				for (int j=0; j<*wait_ptr; j++)
					nanosleep(&sleep_req, NULL);
			}

			CLK_DIV(divI+1, 0);
			
			for (int j=0; j<*wait_ptr; j++)
				nanosleep(&sleep_req, NULL);
		}

		pos_ptr++;
		wait_ptr++;
	}

	CLK_DISAB;
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
