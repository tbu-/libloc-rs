use zerocopy::byteorder::big_endian as be;

#[cfg(not(feature = "verified"))]
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned};

#[cfg(not(feature = "verified"))]
macro_rules! manual_impl {
    ($ty:path: $($trait:path),*) => {
        $(
            unsafe impl $trait for $ty {
                fn only_derive_is_allowed_to_implement_this_trait() {}
            }
        )*
    };
}

#[cfg(feature = "verified")]
use zerocopy_derive::{AsBytes, FromBytes, FromZeroes, Unaligned};

#[cfg(feature = "verified")]
macro_rules! manual_impl {
    ($ty:ident: $($trait:ident),*) => {};
}

pub const MAGIC: [u8; 7] = *b"LOCDBXX";
pub const VERSION: u8 = 1;

manual_impl!(StrRef: AsBytes, FromBytes, FromZeroes, Unaligned);
#[cfg_attr(
    feature = "verified",
    derive(AsBytes, FromBytes, FromZeroes, Unaligned)
)]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StrRef {
    pub offset: be::U32,
}

manual_impl!(FileRange: AsBytes, FromBytes, FromZeroes, Unaligned);
#[cfg_attr(
    feature = "verified",
    derive(AsBytes, FromBytes, FromZeroes, Unaligned)
)]
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FileRange {
    pub offset: be::U32,
    pub length: be::U32,
}

manual_impl!(Header: AsBytes, FromBytes, FromZeroes, Unaligned);
#[cfg_attr(
    feature = "verified",
    derive(AsBytes, FromBytes, FromZeroes, Unaligned)
)]
#[derive(Debug)]
#[repr(C)]
pub struct Header {
    pub magic: [u8; 7],
    pub version: u8,
    pub created_at: be::U64,
    pub vendor: StrRef,
    pub description: StrRef,
    pub license: StrRef,
    pub as_: FileRange,
    pub networks: FileRange,
    pub network_nodes: FileRange,
    pub countries: FileRange,
    pub string_pool: FileRange,
    pub signature1_length: be::U16,
    pub signature2_length: be::U16,
    pub signature1_buf: [u8; 2048],
    pub signature2_buf: [u8; 2048],
    pub padding: [u8; 32],
}

manual_impl!(As: AsBytes, FromBytes, FromZeroes, Unaligned);
#[cfg_attr(
    feature = "verified",
    derive(AsBytes, FromBytes, FromZeroes, Unaligned)
)]
#[derive(Debug)]
#[repr(C)]
pub struct As {
    pub id: be::U32,
    pub name: StrRef,
}

pub const NETWORK_FLAG_ANONYMOUS_PROXY: u16 = 1 << 0;
pub const NETWORK_FLAG_SATTELITE_PROVIDER: u16 = 1 << 1;
pub const NETWORK_FLAG_ANYCAST: u16 = 1 << 2;
pub const NETWORK_FLAG_DROP: u16 = 1 << 3;

manual_impl!(Network: AsBytes, FromBytes, FromZeroes, Unaligned);
#[cfg_attr(
    feature = "verified",
    derive(AsBytes, FromBytes, FromZeroes, Unaligned)
)]
#[derive(Debug)]
#[repr(C)]
pub struct Network {
    pub country_code: [u8; 2],
    pub _padding1: [u8; 2],
    pub asn: be::U32,
    pub flags: be::U16,
    pub _padding2: [u8; 2],
}

manual_impl!(NetworkNode: AsBytes, FromBytes, FromZeroes, Unaligned);
#[cfg_attr(
    feature = "verified",
    derive(AsBytes, FromBytes, FromZeroes, Unaligned)
)]
#[derive(Debug)]
#[repr(C)]
pub struct NetworkNode {
    pub children: [be::U32; 2],
    pub network: be::U32,
}

manual_impl!(Country: AsBytes, FromBytes, FromZeroes, Unaligned);
#[cfg_attr(
    feature = "verified",
    derive(AsBytes, FromBytes, FromZeroes, Unaligned)
)]
#[derive(Debug)]
#[repr(C)]
pub struct Country {
    pub code: [u8; 2],
    pub continent_code: [u8; 2],
    pub name: StrRef,
}

impl NetworkNode {
    pub fn network(&self) -> Option<u32> {
        let network = self.network.get();
        if network != u32::MAX {
            Some(network)
        } else {
            None
        }
    }
}
