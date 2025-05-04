mod wspr;
mod hardware;

use wspr::*;
use hardware::*;

use bitvec::prelude::*;

fn main() {
    // CW
    /*let base_freq = 2.;
    let pll_freq = 750.;
    let (divI, divF) = div_from_freq!(base_freq, pll_freq);

    let g_in = 12;
    let g_clk = 4;

    let cntrl = GPIOController::new();
    println!("Broadcasting CW at {} MHz", base_freq*100.);
    unsafe {
        cntrl.cw(g_in, g_clk, divI, divF);
    }*/


    // WSPR encoding test
    /*let wspr_msg = WSPRMessage::new("KC3ZNA", "AA00", 1);
    let wspr_msg = wspr_msg.encode_transmission();
    println!("{}", wspr_msg.len());*/

    let (data_len, wait_per_row, pos_array, wait_array) = read_image_file("Image_Processing/Images/mauzy.txt");

    // Hardware test
    let cntrl = GPIOController::new();
    //let freq_shift = 0.012/8192.;

    //let base_freq = 1.8366;
    //let base_freq = 0.4742;
    //let pll_freq = 750.;
    unsafe {
        //let (divI, divF) = div_from_freq!(base_freq, pll_freq);
        //println!("{}, {}", divI, divF);
        //cntrl.pulse_clock(4, divI, divF, 5000);

        //cntrl.test_clock(4, 35);
        //println!("{}", cntrl.clock_busy());
        cntrl.broadcast_image(pos_array.as_ptr(), wait_array.as_ptr(), data_len, 32);
        //cntrl.transmit_wspr(wspr_msg, base_freq);
    }
}
