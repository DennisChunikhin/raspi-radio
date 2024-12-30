mod wspr;
mod hardware;

use wspr::*;
use hardware::*;

fn main() {
    // WSPR encoding test
    let wspr_msg = WSPRMessage::new("KC3ZNA", "AA00", 0);
    let wspr_msg = wspr_msg.encode_transmission();
    //println!("{}", wspr_msg);

    // Hardware test
    let cntrl = GPIOController::new();
    unsafe {
        //cntrl.pulse_clock(4, 35, 1, 5000);
        cntrl.test_clock(4, 35);
        //println!("{}", cntrl.clock_busy());
    }
}
