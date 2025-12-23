use alloy_primitives::{Address, U256};

use crate::{
    layout::{Layout, Packable, StorableType},
    storage::Slot,
    InteropError,
    Result,
};

pub(crate) mod sealed {
    pub trait OnlyPrimitives {}
}

macro_rules! impl_unsigned_packable {
    ($ty:ty, $bytes:expr) => {
        impl sealed::OnlyPrimitives for $ty {}

        impl StorableType for $ty {
            const LAYOUT: Layout = Layout::Bytes($bytes);
            type Handler = Slot<Self>;

            fn handle(slot: U256, ctx: crate::LayoutCtx) -> Self::Handler {
                Slot::new_with_ctx(slot, ctx)
            }
        }

        impl Packable for $ty {
            fn to_word(&self) -> U256 {
                U256::from(*self)
            }

            fn from_word(word: U256) -> Result<Self> {
                let bytes = word.to_be_bytes::<32>();
                let start = 32 - $bytes;
                let mut value_bytes = [0u8; $bytes];
                value_bytes.copy_from_slice(&bytes[start..]);
                Ok(<$ty>::from_be_bytes(value_bytes))
            }
        }
    };
}

macro_rules! impl_signed_packable {
    ($ty:ty, $bytes:expr) => {
        impl sealed::OnlyPrimitives for $ty {}

        impl StorableType for $ty {
            const LAYOUT: Layout = Layout::Bytes($bytes);
            type Handler = Slot<Self>;

            fn handle(slot: U256, ctx: crate::LayoutCtx) -> Self::Handler {
                Slot::new_with_ctx(slot, ctx)
            }
        }

        impl Packable for $ty {
            fn to_word(&self) -> U256 {
                let bytes = self.to_be_bytes();
                let mut out = [0u8; 32];
                let sign_fill = if bytes[0] & 0x80 != 0 { 0xff } else { 0x00 };
                out[..32 - bytes.len()].fill(sign_fill);
                out[32 - bytes.len()..].copy_from_slice(&bytes);
                U256::from_be_bytes(out)
            }

            fn from_word(word: U256) -> Result<Self> {
                let bytes = word.to_be_bytes::<32>();
                let start = 32 - $bytes;
                let mut value_bytes = [0u8; $bytes];
                value_bytes.copy_from_slice(&bytes[start..]);
                Ok(<$ty>::from_be_bytes(value_bytes))
            }
        }
    };
}

impl sealed::OnlyPrimitives for bool {}

impl StorableType for bool {
    const LAYOUT: Layout = Layout::Bytes(1);
    type Handler = Slot<Self>;

    fn handle(slot: U256, ctx: crate::LayoutCtx) -> Self::Handler {
        Slot::new_with_ctx(slot, ctx)
    }
}

impl Packable for bool {
    fn to_word(&self) -> U256 {
        if *self {
            U256::from(1u8)
        } else {
            U256::ZERO
        }
    }

    fn from_word(word: U256) -> Result<Self> {
        let value = word.to_be_bytes::<32>()[31];
        match value {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(InteropError::InvalidBool(other.into())),
        }
    }
}

impl sealed::OnlyPrimitives for Address {}

impl StorableType for Address {
    const LAYOUT: Layout = Layout::Bytes(20);
    type Handler = Slot<Self>;

    fn handle(slot: U256, ctx: crate::LayoutCtx) -> Self::Handler {
        Slot::new_with_ctx(slot, ctx)
    }
}

impl Packable for Address {
    fn to_word(&self) -> U256 {
        U256::from_be_slice(self.as_slice())
    }

    fn from_word(word: U256) -> Result<Self> {
        let bytes = word.to_be_bytes::<32>();
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes[12..]);
        Ok(Address::from(addr))
    }
}

impl sealed::OnlyPrimitives for U256 {}

impl StorableType for U256 {
    const LAYOUT: Layout = Layout::Bytes(32);
    type Handler = Slot<Self>;

    fn handle(slot: U256, ctx: crate::LayoutCtx) -> Self::Handler {
        Slot::new_with_ctx(slot, ctx)
    }
}

impl Packable for U256 {
    fn to_word(&self) -> U256 {
        *self
    }

    fn from_word(word: U256) -> Result<Self> {
        Ok(word)
    }
}

impl_unsigned_packable!(u8, 1);
impl_unsigned_packable!(u16, 2);
impl_unsigned_packable!(u32, 4);
impl_unsigned_packable!(u64, 8);
impl_unsigned_packable!(u128, 16);

impl_signed_packable!(i8, 1);
impl_signed_packable!(i16, 2);
impl_signed_packable!(i32, 4);
impl_signed_packable!(i64, 8);
impl_signed_packable!(i128, 16);
