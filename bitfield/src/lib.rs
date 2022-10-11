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
pub const fn mask_from_width(width: u8) -> u8 {
    (2_usize.pow(width as u32) - 1) as u8
}

pub struct StartEndInfo {
    start_idx: usize,
    end_idx: usize,
    end_on_byte_boundary: bool,
}

pub trait Specifier {
    const BITS: usize;
    type UTYPE;

    // Has different implementions based on UTYPE
    fn write_to_bytes(val: Self::UTYPE, offset: usize, raw: &mut [u8]);
    fn read_from_bytes(offset: usize, raw: &[u8]) -> Self::UTYPE;

    #[inline]
    fn start_end_info(offset: usize) -> StartEndInfo {
        StartEndInfo {
            start_idx: offset / 8,
            end_idx: (offset + Self::BITS) / 8,
            end_on_byte_boundary: (offset + Self::BITS) % 8 == 0,
        }
    }

    #[inline]
    fn lshift_from_end(end_idx: usize, offset: usize) -> usize {
        ((end_idx + 1) * 8) - (offset + Self::BITS)
    }

    #[inline]
    fn first_seg_width(offset: usize) -> u8 {
        (8 - (offset % 8)) as u8
    }

    #[inline]
    fn last_seg_width(offset: usize) -> u8 {
        ((offset + Self::BITS) % 8) as u8
    }
}

make_bitwidth_markers!();

pub mod checks {
    pub trait TotalSizeIsMultipleOfEightsBits {}
    pub struct ZeroMod8 {}
    impl ZeroMod8 {
        pub const NUM: usize = 0;
    }
    pub struct OneMod8 {}
    impl OneMod8 {
        pub const NUM: usize = 1;
    }
    pub struct TwoMod8 {}
    impl TwoMod8 {
        pub const NUM: usize = 2;
    }
    pub struct ThreeMod8 {}
    impl ThreeMod8 {
        pub const NUM: usize = 3;
    }
    pub struct FourMod8 {}
    impl FourMod8 {
        pub const NUM: usize = 4;
    }
    pub struct FiveMod8 {}
    impl FiveMod8 {
        pub const NUM: usize = 5;
    }
    pub struct SixMod8 {}
    impl SixMod8 {
        pub const NUM: usize = 6;
    }
    pub struct SevenMod8 {}
    impl SevenMod8 {
        pub const NUM: usize = 7;
    }
    pub trait NumToGeneric {
        type GENERIC;
    }
    pub struct NumDummy<const TOTAL_WIDTH_IN_BYTES: usize> {}
    impl NumToGeneric for NumDummy<{ ZeroMod8::NUM }> {
        type GENERIC = ZeroMod8;
    }
    impl NumToGeneric for NumDummy<{ OneMod8::NUM }> {
        type GENERIC = OneMod8;
    }
    impl NumToGeneric for NumDummy<{ TwoMod8::NUM }> {
        type GENERIC = TwoMod8;
    }
    impl NumToGeneric for NumDummy<{ ThreeMod8::NUM }> {
        type GENERIC = ThreeMod8;
    }
    impl NumToGeneric for NumDummy<{ FourMod8::NUM }> {
        type GENERIC = FourMod8;
    }
    impl NumToGeneric for NumDummy<{ FiveMod8::NUM }> {
        type GENERIC = FiveMod8;
    }
    impl NumToGeneric for NumDummy<{ SixMod8::NUM }> {
        type GENERIC = SixMod8;
    }
    impl NumToGeneric for NumDummy<{ SevenMod8::NUM }> {
        type GENERIC = SevenMod8;
    }
    impl TotalSizeIsMultipleOfEightsBits for ZeroMod8 {}
    pub fn width_check<T: TotalSizeIsMultipleOfEightsBits>() {}
}
