// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run

extern crate core;

use bitfield::*;

#[bitfield]
pub struct RedirectionTableEntry {
    acknowledged: bool,
    trigger_mode: TriggerMode,
    delivery_mode: DeliveryMode,
    reserved: B3,
}

#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum TriggerMode {
    Edge = 0,
    Level = 1,
}

#[derive(BitfieldSpecifier, Debug, PartialEq)]
pub enum DeliveryMode {
    Fixed = 0b000,
    Lowest = 0b001,
    SMI = 0b010,
    RemoteRead = 0b011,
    NMI = 0b100,
    Init = 0b101,
    Startup = 0b110,
    External = 0b111,
}

// impl DeliveryMode {
//     fn from_u64(val: u64) -> Self {
//         match val {
//             x if x == Self::External as u64 => Self::External,
//             _ => panic!("oh no")
//         }
//     }
// }
fn main() {
    assert_eq!(std::mem::size_of::<RedirectionTableEntry>(), 1);

    // Initialized to all 0 bits.
    let mut entry = RedirectionTableEntry::new();
    assert_eq!(entry.get_acknowledged(), false);
    assert_eq!(entry.get_trigger_mode(), TriggerMode::Edge);
    assert_eq!(entry.get_delivery_mode(), DeliveryMode::Fixed);

    entry.set_acknowledged(true);
    entry.set_delivery_mode(DeliveryMode::SMI);
    assert_eq!(entry.get_acknowledged(), true);
    assert_eq!(entry.get_trigger_mode(), TriggerMode::Edge);
    assert_eq!(entry.get_delivery_mode(), DeliveryMode::SMI);
}

// // fn main() {
// //     let test = true;
// //     let test2 = test as u64;
// //
// // }
