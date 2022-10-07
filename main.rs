// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run

use bitfield::*;

#[bitfield]
pub struct MyFourBytes {
    a: B1,
    b: B3,
    c: B4,
    d: B24,
}

fn main() {
    //let mut bitfield = MyFourBytes::new();
    /*
    let mut bitfield = MyFourBytes::new();
    assert_eq!(0, bitfield.get_a());
    assert_eq!(0, bitfield.get_b());
    assert_eq!(0, bitfield.get_c());
    assert_eq!(0, bitfield.get_d());

    bitfield.set_c(14);
    assert_eq!(0, bitfield.get_a());
    assert_eq!(0, bitfield.get_b());
    assert_eq!(14, bitfield.get_c());
    assert_eq!(0, bitfield.get_d());

     */
    //let mut raw_bytes: [u8; 4] = [0; 4];
    // set second and third bit
    // clear bits first
    /*
    let offset = 1;
    let width = 2;
    let end_offset = offset + width;
    let first_byte_index = offset / 8;
    let end_byte_index = end_offset / 8;
    raw_bytes[first_byte_index] =
        (raw_bytes[first_byte_index] & !((0b11000000 as u8) >> 1)) | (0b11000000 >> 1);
    println!("{:x?}", raw_bytes);
    // get second and third bit
    let bits = (raw_bytes[0] >> 5) & 0b11;
    println!("Bits: {:#b}", bits);

    raw_bytes = [0; 4];
    println!("{:x?}", raw_bytes);
    */
}

// For the proc macro, we might be able to generate these!
/*
fn mask_for_width(width: u8) -> u8 {
    match width {
        1 => 0b1,
        2 => 0b11,
        3 => 0b111,
        4 => 0b1111,
        5 => 0b11111,
        6 => 0b111111,
        7 => 0b1111111,
        _ => panic!("shit"),
    }
}

fn shift_from_width(width: u8) -> u8 {
    7 - width
}
*/