use alloy_primitives::{Bytes, U256, keccak256};
use std::marker::PhantomData;

use crate::{
    layout::{Handler, Layout, LayoutCtx, Storable, StorableType},
    slot::Slot,
    storage::StorageOps,
    InteropError,
    Result,
};

impl StorableType for Bytes {
    const LAYOUT: Layout = Layout::Slots(1);
    const IS_DYNAMIC: bool = true;
    type Handler = BytesLikeHandler<Self>;

    fn handle(slot: U256, _ctx: LayoutCtx) -> Self::Handler {
        BytesLikeHandler::new(slot)
    }
}

impl StorableType for String {
    const LAYOUT: Layout = Layout::Slots(1);
    const IS_DYNAMIC: bool = true;
    type Handler = BytesLikeHandler<Self>;

    fn handle(slot: U256, _ctx: LayoutCtx) -> Self::Handler {
        BytesLikeHandler::new(slot)
    }
}

#[derive(Debug, Clone)]
pub struct BytesLikeHandler<T> {
    base_slot: U256,
    _ty: PhantomData<T>,
}

impl<T: Storable> BytesLikeHandler<T> {
    #[inline]
    pub fn new(base_slot: U256) -> Self {
        Self {
            base_slot,
            _ty: PhantomData,
        }
    }

    #[inline]
    fn as_slot(&self) -> Slot<T> {
        Slot::new(self.base_slot)
    }

    #[inline]
    pub fn len<S: StorageOps>(&self, storage: &S) -> Result<usize> {
        let base_value = Slot::<U256>::new(self.base_slot).read(storage)?;
        let is_long = is_long_string(base_value);
        Ok(calc_string_length(base_value, is_long))
    }

    #[inline]
    pub fn is_empty<S: StorageOps>(&self, storage: &S) -> Result<bool> {
        Ok(self.len(storage)? == 0)
    }
}

impl<T: Storable> Handler<T> for BytesLikeHandler<T> {
    fn read<S: StorageOps>(&self, storage: &S) -> Result<T> {
        self.as_slot().read(storage)
    }

    fn write<S: StorageOps>(&mut self, storage: &mut S, value: T) -> Result<()> {
        self.as_slot().write(storage, value)
    }

    fn delete<S: StorageOps>(&mut self, storage: &mut S) -> Result<()> {
        self.as_slot().delete(storage)
    }
}

impl Storable for Bytes {
    fn load<S: StorageOps>(storage: &S, slot: U256, ctx: LayoutCtx) -> Result<Self> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Bytes cannot be packed");
        load_bytes_like(storage, slot, |data| Ok(Self::from(data)))
    }

    fn store<S: StorageOps>(&self, storage: &mut S, slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Bytes cannot be packed");
        store_bytes_like(self.as_ref(), storage, slot)
    }

    fn delete<S: StorageOps>(storage: &mut S, slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Bytes cannot be packed");
        delete_bytes_like(storage, slot)
    }
}

impl Storable for String {
    fn load<S: StorageOps>(storage: &S, slot: U256, ctx: LayoutCtx) -> Result<Self> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "String cannot be packed");
        load_bytes_like(storage, slot, |data| {
            String::from_utf8(data).map_err(|_| InteropError::InvalidUtf8)
        })
    }

    fn store<S: StorageOps>(&self, storage: &mut S, slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "String cannot be packed");
        store_bytes_like(self.as_bytes(), storage, slot)
    }

    fn delete<S: StorageOps>(storage: &mut S, slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "String cannot be packed");
        delete_bytes_like(storage, slot)
    }
}

fn load_bytes_like<T, S, F>(storage: &S, base_slot: U256, into: F) -> Result<T>
where
    S: StorageOps,
    F: FnOnce(Vec<u8>) -> Result<T>,
{
    let base_value = storage.load(base_slot)?;
    let is_long = is_long_string(base_value);
    let length = calc_string_length(base_value, is_long);

    if is_long {
        let slot_start = calc_data_slot(base_slot);
        let chunks = calc_chunks(length);
        let mut data = Vec::with_capacity(length);

        for i in 0..chunks {
            let slot = slot_start + U256::from(i);
            let chunk_value = storage.load(slot)?;
            let chunk_bytes = chunk_value.to_be_bytes::<32>();

            let bytes_to_take = if i == chunks - 1 {
                length - (i * 32)
            } else {
                32
            };
            data.extend_from_slice(&chunk_bytes[..bytes_to_take]);
        }

        into(data)
    } else {
        let bytes = base_value.to_be_bytes::<32>();
        into(bytes[..length].to_vec())
    }
}

fn store_bytes_like<S: StorageOps>(bytes: &[u8], storage: &mut S, base_slot: U256) -> Result<()> {
    let length = bytes.len();

    if length <= 31 {
        storage.store(base_slot, encode_short_string(bytes))
    } else {
        storage.store(base_slot, encode_long_string_length(length))?;

        let slot_start = calc_data_slot(base_slot);
        let chunks = calc_chunks(length);

        for i in 0..chunks {
            let slot = slot_start + U256::from(i);
            let chunk_start = i * 32;
            let chunk_end = (chunk_start + 32).min(length);
            let chunk = &bytes[chunk_start..chunk_end];

            let mut chunk_bytes = [0u8; 32];
            chunk_bytes[..chunk.len()].copy_from_slice(chunk);

            storage.store(slot, U256::from_be_bytes(chunk_bytes))?;
        }

        Ok(())
    }
}

fn delete_bytes_like<S: StorageOps>(storage: &mut S, base_slot: U256) -> Result<()> {
    let base_value = storage.load(base_slot)?;
    let is_long = is_long_string(base_value);

    if is_long {
        let length = calc_string_length(base_value, true);
        let slot_start = calc_data_slot(base_slot);
        let chunks = calc_chunks(length);

        for i in 0..chunks {
            let slot = slot_start + U256::from(i);
            storage.store(slot, U256::ZERO)?;
        }
    }

    storage.store(base_slot, U256::ZERO)
}

#[inline]
fn calc_data_slot(base_slot: U256) -> U256 {
    U256::from_be_bytes(keccak256(base_slot.to_be_bytes::<32>()).0)
}

#[inline]
fn calc_chunks(length: usize) -> usize {
    length.div_ceil(32)
}

#[inline]
fn is_long_string(value: U256) -> bool {
    value.bit(0)
}

#[inline]
fn calc_string_length(value: U256, is_long: bool) -> usize {
    if is_long {
        (value >> 1).to::<usize>()
    } else {
        ((value & U256::from(0xff)) >> 1).to::<usize>()
    }
}

#[inline]
fn encode_short_string(bytes: &[u8]) -> U256 {
    let mut slot_bytes = [0u8; 32];
    slot_bytes[..bytes.len()].copy_from_slice(bytes);
    slot_bytes[31] = (bytes.len() as u8) << 1;
    U256::from_be_bytes(slot_bytes)
}

#[inline]
fn encode_long_string_length(length: usize) -> U256 {
    U256::from((length as u64) << 1 | 1)
}
