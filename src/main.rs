mod wspr;
mod hardware;

use wspr::*;
use hardware::*;

use bitvec::prelude::*;

fn main() {
    // WSPR encoding test
    let wspr_msg = WSPRMessage::new("KC3ZNA", "AA00", 0);
    let wspr_msg = wspr_msg.encode_transmission();

    //println!("{}", wspr_msg);

    let (data_len, wait_per_row, pos_array, wait_array) = read_image_file("Image_Processing/Images/mauzy.txt");

    // Hardware test
    let cntrl = GPIOController::new();
    unsafe {
        //cntrl.pulse_clock(4, 35, 1, 5000);
        //cntrl.test_clock(4, 35);
        //println!("{}", cntrl.clock_busy());
        cntrl.broadcast_image(pos_array.as_ptr(), wait_array.as_ptr(), data_len, 3);
    }

    //let base_freq = 14.0956;
}
