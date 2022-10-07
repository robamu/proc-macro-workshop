// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub use bitfield_impl::bitfield;
use bitfield_impl::make_bitwidth_markers;

pub trait Specifier {
    const BITS: usize;
    const MASK: usize;
    type UTYPE;

    #[inline]
    fn last_seg_width(offset: usize) -> u8 {
        ((offset + Self::BITS) % 8) as u8
    }

    #[inline]
    fn first_seg_width(offset: usize) -> u8 {
        (8 - (offset % 8)) as u8
    }

    #[inline]
    fn middle_segments(first_seg_width: usize, last_seg_width: usize) -> u8 {
        ((Self::BITS - first_seg_width - last_seg_width) / 8) as u8
    }
}

make_bitwidth_markers!();
