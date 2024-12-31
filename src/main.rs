mod wspr;
mod hardware;

use wspr::*;

fn main() {
    let wspr_msg = WSPRMessage::new("KC3ZNA", "AA00", 0);
    let wspr_msg = wspr_msg.encode_transmission();
    println!("{}", wspr_msg);
}
