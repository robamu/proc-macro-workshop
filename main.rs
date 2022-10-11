// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run

use bitfield::*;
use std::marker::PhantomData;

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

/*
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
*/

const TEST_WIDTH: usize = 13;
const MOD: usize = TEST_WIDTH % 8;
//struct Check<const WIDTH_MOD_EIGHT: usize> {}

pub enum ZeroMod8 {}
pub enum OneMod8 {}

pub trait MultipleOfEightBits {}
impl MultipleOfEightBits for ZeroMod8 {}

struct Token<T: MultipleOfEightBits> {
    phantom: PhantomData<T>,
}

struct Hello<const WIDTH_MOD_EIGHT: usize> {}

impl Hello<0> {
    pub fn new() -> Self {
        let _ = Token::<ZeroMod8> {
            phantom: Default::default(),
        };
        Self {}
    }
}

impl Hello<1> {
    pub fn new() -> Self {
        let _ = Token::<OneMod8> {
            phantom: Default::default(),
        };
        Self {}
    }
}
fn main() {
    //Hello::<MOD>::example_call();
    let test = Hello::<0>::new();
    let mut bitfield = MyFourBytes::new();

    /*
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

    let mut bitfield = OtherFourBytes::new();
    assert_eq!(0, bitfield.get_d());
    bitfield.set_d(0x30003);
    println!("{:x?}", bitfield.raw_data());
    assert_eq!(0x30003, bitfield.get_d());
     */
}
