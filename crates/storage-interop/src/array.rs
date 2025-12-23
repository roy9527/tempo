use alloy_primitives::U256;
use std::marker::PhantomData;

use crate::{
    layout::{Handler, Layout, LayoutCtx, Storable, StorableType},
    packing,
    slot::Slot,
    storage::StorageOps,
    Result,
};

pub struct ArrayHandler<T, const N: usize>
where
    T: StorableType,
{
    base_slot: U256,
    _phantom: PhantomData<T>,
}

impl<T, const N: usize> ArrayHandler<T, N>
where
    T: StorableType,
{
    #[inline]
    pub fn new(base_slot: U256) -> Self {
        Self {
            base_slot,
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn as_slot(&self) -> Slot<[T; N]> {
        Slot::new(self.base_slot)
    }

    #[inline]
    pub fn base_slot(&self) -> U256 {
        self.base_slot
    }

    #[inline]
    pub const fn len(&self) -> usize {
        N
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    #[inline]
    pub fn at(&self, index: usize) -> Option<T::Handler> {
        if index >= N {
            return None;
        }

        let (base_slot, layout_ctx) = if T::BYTES <= 16 {
            let location = packing::calc_element_loc(index, T::BYTES);
            (
                self.base_slot + U256::from(location.offset_slots),
                LayoutCtx::packed(location.offset_bytes),
            )
        } else {
            (
                self.base_slot + U256::from(index * T::SLOTS),
                LayoutCtx::FULL,
            )
        };

        Some(T::handle(base_slot, layout_ctx))
    }
}

impl<T, const N: usize> Handler<[T; N]> for ArrayHandler<T, N>
where
    T: StorableType,
    [T; N]: Storable,
{
    fn read<S: StorageOps>(&self, storage: &S) -> Result<[T; N]> {
        self.as_slot().read(storage)
    }

    fn write<S: StorageOps>(&mut self, storage: &mut S, value: [T; N]) -> Result<()> {
        self.as_slot().write(storage, value)
    }

    fn delete<S: StorageOps>(&mut self, storage: &mut S) -> Result<()> {
        self.as_slot().delete(storage)
    }
}

impl<T, const N: usize> StorableType for [T; N]
where
    T: Storable,
{
    const LAYOUT: Layout = if T::BYTES <= 16 {
        Layout::Slots(packing::calc_packed_slot_count(N, T::BYTES))
    } else {
        Layout::Slots(N * T::SLOTS)
    };

    type Handler = ArrayHandler<T, N>;

    fn handle(slot: U256, _ctx: LayoutCtx) -> Self::Handler {
        ArrayHandler::new(slot)
    }
}

impl<T, const N: usize> Storable for [T; N]
where
    T: Storable,
{
    fn load<S: StorageOps>(storage: &S, base_slot: U256, ctx: LayoutCtx) -> Result<Self> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Arrays cannot be packed");

        if T::BYTES <= 16 {
            load_packed_array(storage, base_slot)
        } else {
            load_unpacked_array(storage, base_slot)
        }
    }

    fn store<S: StorageOps>(&self, storage: &mut S, base_slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Arrays cannot be packed");

        if T::BYTES <= 16 {
            store_packed_array(self, storage, base_slot)
        } else {
            store_unpacked_array(self, storage, base_slot)
        }
    }

    fn delete<S: StorageOps>(storage: &mut S, base_slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Arrays cannot be packed");

        if T::BYTES <= 16 {
            let slot_count = packing::calc_packed_slot_count(N, T::BYTES);
            for slot_idx in 0..slot_count {
                storage.store(base_slot + U256::from(slot_idx), U256::ZERO)?;
            }
        } else {
            for index in 0..N {
                let slot = base_slot + U256::from(index * T::SLOTS);
                T::delete(storage, slot, LayoutCtx::FULL)?;
            }
        }

        Ok(())
    }
}

fn load_packed_array<T, const N: usize, S: StorageOps>(
    storage: &S,
    base_slot: U256,
) -> Result<[T; N]>
where
    T: Storable,
{
    let mut data: [std::mem::MaybeUninit<T>; N] =
        std::array::from_fn(|_| std::mem::MaybeUninit::uninit());

    for index in 0..N {
        let loc = packing::calc_element_loc(index, T::BYTES);
        let slot = base_slot + U256::from(loc.offset_slots);
        let value = T::load(storage, slot, LayoutCtx::packed(loc.offset_bytes))?;
        data[index].write(value);
    }

    Ok(unsafe { std::mem::MaybeUninit::array_assume_init(data) })
}

fn load_unpacked_array<T, const N: usize, S: StorageOps>(
    storage: &S,
    base_slot: U256,
) -> Result<[T; N]>
where
    T: Storable,
{
    let mut data: [std::mem::MaybeUninit<T>; N] =
        std::array::from_fn(|_| std::mem::MaybeUninit::uninit());

    for index in 0..N {
        let slot = base_slot + U256::from(index * T::SLOTS);
        let value = T::load(storage, slot, LayoutCtx::FULL)?;
        data[index].write(value);
    }

    Ok(unsafe { std::mem::MaybeUninit::array_assume_init(data) })
}

fn store_packed_array<T, const N: usize, S: StorageOps>(
    values: &[T; N],
    storage: &mut S,
    base_slot: U256,
) -> Result<()>
where
    T: Storable,
{
    for index in 0..N {
        let loc = packing::calc_element_loc(index, T::BYTES);
        let slot = base_slot + U256::from(loc.offset_slots);
        values[index].store(storage, slot, LayoutCtx::packed(loc.offset_bytes))?;
    }
    Ok(())
}

fn store_unpacked_array<T, const N: usize, S: StorageOps>(
    values: &[T; N],
    storage: &mut S,
    base_slot: U256,
) -> Result<()>
where
    T: Storable,
{
    for index in 0..N {
        let slot = base_slot + U256::from(index * T::SLOTS);
        values[index].store(storage, slot, LayoutCtx::FULL)?;
    }
    Ok(())
}
