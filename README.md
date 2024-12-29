# Raspberry Pi Radio Transmitter

Uses a Raspberry Pi GPIO pin clock to transmit RF.

Contains C code that changes transmit frequency to draw an image on a waterfall and work in progress Rust code that implents the WSPR protocol.

[src/wspr.rs](src/wspr.rs) contains functions that encode a WSPR message.

[Image_Processing/prep_image.py](Image_Processing/prep_image.py) contains a simple script that converts an image to a data file that can be read by [image.c](image.c).

[image.c](image.c) and [gpio_test.c](gpio_test.c) contain simple scripts to test transmitting at set frequencies using a GPIO pin clock.
