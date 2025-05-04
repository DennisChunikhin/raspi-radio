use bitvec::prelude::*;

// WSPR protocol description from: https://swharden.com/software/FSKview/wspr/

const REG0_TAP: u32 = 0xF2D05351;
const REG1_TAP: u32 = 0xE4613C47;
const NUM_BITS_WSPR: usize = 81;

type WSPRBits = BitArr!(for NUM_BITS_WSPR*2);

pub enum WSPRSymbol {
    A,
    B,
    C,
    D
}

pub struct Callsign(String);

impl Callsign {
    // Constructor
    pub fn new(callsign: &str) -> Callsign {
        let mut callsign = callsign.to_uppercase();

        // Pads callsign with spaces
        callsign.chars().nth(1).inspect(|chr|
            if chr.is_digit(10) {
                callsign.insert_str(0, " ");
            }
        );

        Callsign(format!("{: <6}", callsign))
    }

    // Character to numerical code according to WSPR protocol
    pub fn callsign_char_lookup(chr: char) -> u32 {
        chr.to_digit(10).unwrap_or_else(||
            match chr {
                ' ' => 36,
                _ => (chr as u32) - 55
            }
        )
    }

    // Encodes callsign according to WSPR protocol (28 bit output)
    pub fn encode(&self) -> u32 {
        self.0
            .chars()
            .zip( [1,36,10,27,27,27].iter() )
            .fold(0, |acc, (chr, m)| acc*m + Callsign::callsign_char_lookup(chr))
        - 7570
    }
}

pub struct Locator(u32, u32, u32, u32);

impl Locator {
    pub fn from_str(locator: &str) -> Option<Self> {
        match locator.as_bytes() {
            [field_1, field_2, sq_1, sq_2] => Some(Self {
                0: *field_1 as u32 - 65,
                1: *field_2 as u32 - 65,
                2: *sq_1 as u32,
                3: *sq_2 as u32
            }),
            _ => None
        }
    }
}


pub struct WSPRMessage {
    callsign: Callsign,
    locator: Locator,
    power: u32,
}

impl WSPRMessage {
    // Constructor
    pub fn new(callsign: &str, locator: &str, power: u32) -> WSPRMessage {
        if power > 60 {
            panic!("Power must be a u32 between 0 and 60");
        }

        let callsign = Callsign::new(callsign);
        WSPRMessage { callsign, locator: Locator::from_str(locator).expect("Invalid maidenhead locator :3"), power }
    }
    
    // Encodes 4-character Maidenhead Locator and power according to WSPR protocol (22 bit output)
    pub fn encode_locator_power(&self) -> u32 {
        ((179 - 10*self.locator.0 - self.locator.2)*180 + 10*self.locator.1 + self.locator.3)*128 + self.power + 64
    }

    // Encodes the callsign, Maidenhead Locator, and power to an 81 bit WSPR message
    // 11 byte output in the fromat callsign-locator-power-38 0 bits
    pub fn encode(&self) -> u128 {
        let callsign_bits = self.callsign.encode() as u128;
        let locator_power_bits = self.encode_locator_power() as u128;

        (callsign_bits<<(22+38)) + (locator_power_bits<<38)
    }


    // Constraint length K=32
    // Rate r=1/2
    pub fn convolution_code(mut bits: u128) -> WSPRBits {
        let mut out = bitarr![0; NUM_BITS_WSPR*2];

        // Memory registers
        let mut reg0: u32 = 0;
        let mut reg1: u32 = 0;

        for i in 0..NUM_BITS_WSPR {
            // Shift next bit into register
            reg0 <<= 1;
            reg0 |= 1 & (bits as u32);

            reg1 <<= 1;
            reg1 |= 1 & (bits as u32);

            bits >>= 1;

            // Single bit parity of register sums
            let parity1 = (reg0 & REG0_TAP).count_ones() % 2 == 1;
            let parity2 = (reg1 & REG1_TAP).count_ones() % 2 == 1;

            out.set(2*i, parity1);
            out.set(2*i+1, parity2);
        }

        out
    }

    // Interleaves bits according to WSPR protocol specification
    pub fn interleave(bits: WSPRBits) -> WSPRBits {
        let mut out: WSPRBits = bitarr![0; NUM_BITS_WSPR*2];

        let mut p = 0;
        for i in 0u8..=255u8 {
            let j = i.reverse_bits() as usize;
            if j < NUM_BITS_WSPR*2 {
                out.set(j, *bits.get(p).unwrap());
                p += 1;

                if p == NUM_BITS_WSPR*2 {
                    return out;
                }
            }
        }

        out
    }

    // Merges the 162 bit encoded and interleaved WSPR message with a set pseudo-random synchronization
    // bit vector to generate 2-bit symbol values for use in transmission modulation
    pub fn get_symbols(bits: WSPRBits) -> Vec<WSPRSymbol> {
        let sync_vec: WSPRBits = bitarr![1,1,0,0,0,0,0,0,1,0,0,0,1,1,1,0,0,0,1,0,0,1,0,1,1,1,1,0,0,0,0,0,0,0,1,0,0,1,0,1,0,0,0,0,0,0,1,0,1,1,0,0,1,1,0,1,0,0,0,1,1,0,1,0,0,0,0,1,1,0,1,0,1,0,1,0,1,0,0,1,0,0,1,0,1,1,0,0,0,1,1,0,1,0,1,0,0,0,1,0,0,0,0,0,1,0,0,1,0,0,1,1,1,0,1,1,0,0,1,1,0,1,0,0,0,1,1,1,0,0,0,0,0,1,0,1,0,0,1,1,0,0,0,0,0,0,0,1,1,0,1,0,1,1,0,0,0,1,1,0,0,0];
        
        //let mut sym_bits: SymbolBits = bitarr![0; NUM_BITS_WSPR*4];
        let mut sym_bits = Vec::new();

        for (i, (b, s)) in bits.iter().by_vals().zip(sync_vec.iter().by_vals()).enumerate() {
            if i == NUM_BITS_WSPR*2 {
                break;
            }
            sym_bits.push(
                match (b, s) {
                    (false, false) => WSPRSymbol::A,
                    (false, true) => WSPRSymbol::B,
                    (true, false) => WSPRSymbol::C,
                    (true, true) => WSPRSymbol::D,
                }
            );
        }

        sym_bits
    }


    // Converts a callsign, locator, and power an encodes it to a WSPR symbol bit vector ready for
    // transmission
    // Consumes self
    pub fn encode_transmission(self) -> Vec<WSPRSymbol> {
        let bits = self.encode();
        let bits = WSPRMessage::convolution_code(bits);
        let bits = WSPRMessage::interleave(bits);
        WSPRMessage::get_symbols(bits)
    }
}
