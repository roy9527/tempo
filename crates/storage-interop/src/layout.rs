use alloy_primitives::U256;

use crate::{
    packing,
    storage::StorageOps,
    types::sealed,
    Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Single slot, N bytes (1-32). Can be packed with other fields if N < 32.
    Bytes(usize),
    /// Occupies N full slots (each 32 bytes). Cannot be packed.
    Slots(usize),
}

impl Layout {
    pub const fn is_packable(&self) -> bool {
        match self {
            Self::Bytes(_) => true,
            Self::Slots(_) => false,
        }
    }

    pub const fn slots(&self) -> usize {
        match self {
            Self::Bytes(_) => 1,
            Self::Slots(n) => *n,
        }
    }

    pub const fn bytes(&self) -> usize {
        match self {
            Self::Bytes(n) => *n,
            Self::Slots(n) => {
                let (mut i, mut result) = (0, 0);
                while i < *n {
                    result += 32;
                    i += 1;
                }
                result
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct LayoutCtx(usize);

impl LayoutCtx {
    pub const FULL: Self = Self(usize::MAX);

    pub const fn packed(offset: usize) -> Self {
        debug_assert!(offset < 32);
        Self(offset)
    }

    #[inline]
    pub const fn packed_offset(&self) -> Option<usize> {
        if self.0 == usize::MAX {
            None
        } else {
            Some(self.0)
        }
    }
}

pub trait StorableType {
    const LAYOUT: Layout;
    const SLOTS: usize = Self::LAYOUT.slots();
    const BYTES: usize = Self::LAYOUT.bytes();
    const IS_PACKABLE: bool = Self::LAYOUT.is_packable();
    const IS_DYNAMIC: bool = false;

    type Handler;

    fn handle(slot: U256, ctx: LayoutCtx) -> Self::Handler;
}

pub trait Handler<T: Storable> {
    fn read<S: StorageOps>(&self, storage: &S) -> Result<T>;
    fn write<S: StorageOps>(&mut self, storage: &mut S, value: T) -> Result<()>;
    fn delete<S: StorageOps>(&mut self, storage: &mut S) -> Result<()>;
}

pub trait Storable: StorableType + Sized {
    fn load<S: StorageOps>(storage: &S, slot: U256, ctx: LayoutCtx) -> Result<Self>;

    fn store<S: StorageOps>(&self, storage: &mut S, slot: U256, ctx: LayoutCtx) -> Result<()>;

    fn delete<S: StorageOps>(storage: &mut S, slot: U256, ctx: LayoutCtx) -> Result<()> {
        match ctx.packed_offset() {
            None => {
                for offset in 0..Self::SLOTS {
                    storage.store(slot + U256::from(offset), U256::ZERO)?;
                }
                Ok(())
            }
            Some(offset) => {
                let bytes = Self::BYTES;
                let current = storage.load(slot)?;
                let cleared = packing::zero_packed_value(current, offset, bytes)?;
                storage.store(slot, cleared)
            }
        }
    }
}

pub trait Packable: sealed::OnlyPrimitives + StorableType {
    fn to_word(&self) -> U256;
    fn from_word(word: U256) -> Result<Self>
    where
        Self: Sized;
}

impl<T: Packable> Storable for T {
    #[inline]
    fn load<S: StorageOps>(storage: &S, slot: U256, ctx: LayoutCtx) -> Result<Self> {
        const { assert!(T::IS_PACKABLE, "Packable requires IS_PACKABLE to be true") };

        match ctx.packed_offset() {
            None => storage.load(slot).and_then(Self::from_word),
            Some(offset) => {
                let slot_value = storage.load(slot)?;
                packing::extract_packed_value(slot_value, offset, Self::BYTES)
            }
        }
    }

    #[inline]
    fn store<S: StorageOps>(&self, storage: &mut S, slot: U256, ctx: LayoutCtx) -> Result<()> {
        const { assert!(T::IS_PACKABLE, "Packable requires IS_PACKABLE to be true") };

        match ctx.packed_offset() {
            None => storage.store(slot, self.to_word()),
            Some(offset) => {
                let current = storage.load(slot)?;
                let updated = packing::insert_packed_value(current, self, offset, Self::BYTES)?;
                storage.store(slot, updated)
            }
        }
    }
}
