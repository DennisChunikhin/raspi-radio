mod wspr;
mod hardware;

use wspr::*;

fn main() {
    let callsign = String::from("KC3ZNA");
    let locator = "AA00";
    let power = 0;

    //let bits: u128 = 0b1111000010101101;
    //let bits = encode_wspr_message(callsign, locator, power);
    //let bitvec = convolution_code(bits);
    //if let Some(bitvec) = interleave(bitvec) {
        //println!("{}", get_symbols(bitvec));
    //}
    
    println!("{:?}", encode_wspr_transmission(callsign, locator, power));

    //println!("{:b}", encode_callsign(callsign));
    //println!("{:b}", encode_locator_power(locator, power));
}
