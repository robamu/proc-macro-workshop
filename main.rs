// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run

use std::mem;
//use bitfield::*;

pub enum Test {
    A = 3,
    B,
    C = 0,
    D
}

fn main() {
    println!("{:?}", mem::discriminant(&Test::A));
    //let discrim = mem::discriminant(&Test::A);
    println!("{}", Test::A as u8);
    //let discrim_raw: isize = discrim as isize;
    println!("{:?}", mem::discriminant(&Test::B));
    println!("{:?}", mem::discriminant(&Test::C));
    println!("{:?}", mem::discriminant(&Test::D));
}
