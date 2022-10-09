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
#[bitfield]
pub struct MyFourBytes {
    a: B1,
    b: B4,
    c: B7,
    d: B13,
    e: B7,
}
// OFFSET_A = 0
// OFFSET_B = OFFSET_A + B1::BITS
// OFFSET_C = OFFSET_B + B4::BITS

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
    bitfield.set_a(0);
    assert_eq!(0, bitfield.get_a());
    bitfield.set_b(0b111);
    assert_eq!(0b111, bitfield.get_b());
    bitfield.set_b(0b101);
    assert_eq!(0b101, bitfield.get_b());
    bitfield.set_c(0b1111111);
    assert_eq!(0b1111111, bitfield.get_c());
}
