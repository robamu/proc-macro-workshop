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

#[inline]
pub fn mask_from_width(width: u8) -> u8 {
    (2_usize.pow(width as u32) - 1) as u8
}

pub trait Specifier {
    const BITS: usize;
    const MASK: usize;
    type UTYPE;

    // Has different implementions based on UTYPE
    fn write_to_bytes(val: Self::UTYPE, offset: usize, raw: &mut [u8]);
    fn read_from_bytes(offset: usize, raw: &[u8]) -> Self::UTYPE;

    #[inline]
    fn last_seg_width(offset: usize) -> u8 {
        ((offset + Self::BITS) % 8) as u8
    }

    #[inline]
    fn first_seg_width(offset: usize) -> u8 {
        (8 - (offset % 8)) as u8
    }

    fn middle_segments(&self, first_seg_width: u8, last_seg_width: u8) -> u8;
}

make_bitwidth_markers!();
