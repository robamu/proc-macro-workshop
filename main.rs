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
    b: B4,
    c: B7,
    d: B20,
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
    bitfield.set_a(0);
    assert_eq!(0, bitfield.get_a());
    bitfield.set_b(0b111);
    assert_eq!(0b111, bitfield.get_b());
    bitfield.set_b(0b101);
    assert_eq!(0b101, bitfield.get_b());
    bitfield.set_c(0b1111111);
    assert_eq!(0b1111111, bitfield.get_c());
}
