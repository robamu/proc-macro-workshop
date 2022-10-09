// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run

use bitfield::*;

// 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
// A B - - B C - -   - - - C D - - -   - - - - - - - -   D E - - - - - E

// 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 |
// A - - A B - - -   - - - - - - - B

// 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
// A - - - - - A B   - - - - - - - -   - - - - - - - B   C - - - - - - C
// start index 0 -> end index 3: two full segments including last one -> 1, 2 (end - 1)

// 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
// A B - - B C C D   - - - - - - - -   - - - - - - - -   D E - - - - - E
// start index 0 -> end index 3: two full segments -> 1, 2
#[bitfield]
pub struct MyFourBytes {
    a: B1,
    b: B4,
    c: B7,
    d: B13,
    e: B7,
}

#[bitfield]
pub struct MyTwoBytes {
    a: B4,
    b: B12,
}

#[bitfield]
pub struct OtherFourBytes {
    a: B1,
    b: B4,
    c: B2,
    d: B18,
    e: B7,
}

fn main() {
    let mut bitfield = MyFourBytes::new();

    assert_eq!(0, bitfield.get_a());
    assert_eq!(0, bitfield.get_b());
    assert_eq!(0, bitfield.get_c());
    assert_eq!(0, bitfield.get_d());

    //bitfield.set_c(14);
    assert_eq!(0, bitfield.get_a());
    assert_eq!(0, bitfield.get_b());
    // assert_eq!(14, bitfield.get_c());
    assert_eq!(0, bitfield.get_d());
    bitfield.set_a(1);
    assert_eq!(1, bitfield.get_a());
    println!("{:x?}", bitfield.raw_data());
    bitfield.set_a(0);
    assert_eq!(0, bitfield.get_a());
    bitfield.set_b(0b1001);
    println!("{:x?}", bitfield.raw_data());
    assert_eq!(0b1001, bitfield.get_b());
    bitfield.set_b(0b101);
    assert_eq!(0b101, bitfield.get_b());
    bitfield.set_b(0);
    println!("{:x?}", bitfield.raw_data());
    bitfield.set_c(0b1011101);
    assert_eq!(0b1011101, bitfield.get_c());
    println!("{:x?}", bitfield.raw_data());
    bitfield.set_c(0);
    bitfield.set_d(0b1000100000011);
    println!("{:x?}", bitfield.raw_data());
    assert_eq!(0b1000100000011, bitfield.get_d());

    let mut raw_bytes: [u8; 4] = [0; 4];
    raw_bytes[1] = 0b1;
    raw_bytes[2] = 0b10101010;
    raw_bytes[3] = 0b10101010;
    let start_idx = 1;
    let mut val = (raw_bytes[start_idx] & 0b1) as u32;
    val = (val << 8) | raw_bytes[start_idx + 1] as u32;
    val = (val << 8) | raw_bytes[start_idx + 2] as u32;
    println!("Value: {:x?}", val);

    let mut bitfield = OtherFourBytes::new();
    assert_eq!(0, bitfield.get_d());
    bitfield.set_d(0x1800180);
    println!("{:x?}", bitfield.raw_data());
    assert_eq!(0x1800180, bitfield.get_d());
}
