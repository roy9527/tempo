use alloy_primitives::{U256, keccak256};
use std::marker::PhantomData;

use crate::{
    layout::{Handler, Layout, LayoutCtx, Storable, StorableType},
    packing::{PackedSlot, calc_element_loc, calc_packed_slot_count},
    slot::Slot,
    storage::StorageOps,
    Result,
};

impl<T> StorableType for Vec<T>
where
    T: Storable,
{
    const LAYOUT: Layout = Layout::Slots(1);
    const IS_DYNAMIC: bool = true;
    type Handler = VecHandler<T>;

    fn handle(slot: U256, _ctx: LayoutCtx) -> Self::Handler {
        VecHandler::new(slot)
    }
}

impl<T> Storable for Vec<T>
where
    T: Storable,
{
    fn load<S: StorageOps>(storage: &S, len_slot: U256, ctx: LayoutCtx) -> Result<Self> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Dynamic arrays cannot be packed");

        let length_value = storage.load(len_slot)?;
        let length = length_value.to::<usize>();

        if length == 0 {
            return Ok(Self::new());
        }

        let data_start = calc_data_slot(len_slot);
        if T::BYTES <= 16 {
            load_packed_elements(storage, data_start, length, T::BYTES)
        } else {
            load_unpacked_elements(storage, data_start, length)
        }
    }

    fn store<S: StorageOps>(&self, storage: &mut S, len_slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Dynamic arrays cannot be packed");

        storage.store(len_slot, U256::from(self.len()))?;

        if self.is_empty() {
            return Ok(());
        }

        let data_start = calc_data_slot(len_slot);
        if T::BYTES <= 16 {
            store_packed_elements(self, storage, data_start, T::BYTES)
        } else {
            store_unpacked_elements(self, storage, data_start)
        }
    }

    fn delete<S: StorageOps>(storage: &mut S, len_slot: U256, ctx: LayoutCtx) -> Result<()> {
        debug_assert_eq!(ctx, LayoutCtx::FULL, "Dynamic arrays cannot be packed");

        let length_value = storage.load(len_slot)?;
        let length = length_value.to::<usize>();

        storage.store(len_slot, U256::ZERO)?;

        if length == 0 {
            return Ok(());
        }

        let data_start = calc_data_slot(len_slot);
        if T::BYTES <= 16 {
            let slot_count = calc_packed_slot_count(length, T::BYTES);
            for slot_idx in 0..slot_count {
                storage.store(data_start + U256::from(slot_idx), U256::ZERO)?;
            }
        } else {
            for elem_idx in 0..length {
                let elem_slot = data_start + U256::from(elem_idx * T::SLOTS);
                T::delete(storage, elem_slot, LayoutCtx::FULL)?;
            }
        }

        Ok(())
    }
}

pub struct VecHandler<T>
where
    T: Storable,
{
    len_slot: U256,
    _ty: PhantomData<T>,
}

impl<T> Handler<Vec<T>> for VecHandler<T>
where
    T: Storable,
{
    fn read<S: StorageOps>(&self, storage: &S) -> Result<Vec<T>> {
        self.as_slot().read(storage)
    }

    fn write<S: StorageOps>(&mut self, storage: &mut S, value: Vec<T>) -> Result<()> {
        self.as_slot().write(storage, value)
    }

    fn delete<S: StorageOps>(&mut self, storage: &mut S) -> Result<()> {
        self.as_slot().delete(storage)
    }
}

impl<T> VecHandler<T>
where
    T: Storable,
{
    #[inline]
    pub fn new(len_slot: U256) -> Self {
        Self {
            len_slot,
            _ty: PhantomData,
        }
    }

    #[inline]
    pub fn len_slot(&self) -> U256 {
        self.len_slot
    }

    #[inline]
    pub fn data_slot(&self) -> U256 {
        calc_data_slot(self.len_slot)
    }

    #[inline]
    fn as_slot(&self) -> Slot<Vec<T>> {
        Slot::new(self.len_slot)
    }

    #[inline]
    pub fn len<S: StorageOps>(&self, storage: &S) -> Result<usize> {
        let slot = Slot::<U256>::new(self.len_slot);
        Ok(slot.read(storage)?.to::<usize>())
    }

    #[inline]
    pub fn is_empty<S: StorageOps>(&self, storage: &S) -> Result<bool> {
        Ok(self.len(storage)? == 0)
    }

    #[inline]
    pub fn at_unchecked(&self, index: usize) -> T::Handler {
        let data_start = self.data_slot();

        let (base_slot, layout_ctx) = if T::BYTES <= 16 {
            let location = calc_element_loc(index, T::BYTES);
            (
                data_start + U256::from(location.offset_slots),
                LayoutCtx::packed(location.offset_bytes),
            )
        } else {
            (data_start + U256::from(index * T::SLOTS), LayoutCtx::FULL)
        };

        T::handle(base_slot, layout_ctx)
    }

    #[inline]
    pub fn at<S: StorageOps>(&self, storage: &S, index: usize) -> Result<Option<T::Handler>> {
        let length = self.len(storage)?;
        if index >= length {
            return Ok(None);
        }

        Ok(Some(self.at_unchecked(index)))
    }
}

#[inline]
fn calc_data_slot(len_slot: U256) -> U256 {
    U256::from_be_bytes(keccak256(len_slot.to_be_bytes::<32>()).0)
}

fn load_packed_elements<T, S>(
    storage: &S,
    data_start: U256,
    length: usize,
    byte_count: usize,
) -> Result<Vec<T>>
where
    T: Storable,
    S: StorageOps,
{
    let slot_count = calc_packed_slot_count(length, byte_count);
    let mut elements = Vec::with_capacity(length);
    let mut current_index = 0;

    for slot_idx in 0..slot_count {
        let slot_value = storage.load(data_start + U256::from(slot_idx))?;
        let slot_packed = PackedSlot(slot_value);

        let elements_in_slot = ((length - current_index) * byte_count).min(32) / byte_count;
        for offset in 0..elements_in_slot {
            let elem = T::load(
                &slot_packed,
                U256::ZERO,
                LayoutCtx::packed(offset * byte_count),
            )?;
            elements.push(elem);
        }

        current_index += elements_in_slot;
    }

    Ok(elements)
}

fn store_packed_elements<T, S>(
    elements: &[T],
    storage: &mut S,
    data_start: U256,
    byte_count: usize,
) -> Result<()>
where
    T: Storable,
    S: StorageOps,
{
    let slot_count = calc_packed_slot_count(elements.len(), byte_count);

    for slot_idx in 0..slot_count {
        let start_elem = slot_idx * (32 / byte_count);
        let end_elem = (start_elem + (32 / byte_count)).min(elements.len());

        let slot_value = build_packed_slot(&elements[start_elem..end_elem], byte_count)?;
        storage.store(data_start + U256::from(slot_idx), slot_value)?;
    }

    Ok(())
}

fn build_packed_slot<T>(elements: &[T], byte_count: usize) -> Result<U256>
where
    T: Storable,
{
    let mut slot_value = PackedSlot(U256::ZERO);
    let mut current_offset = 0;

    for elem in elements {
        elem.store(&mut slot_value, U256::ZERO, LayoutCtx::packed(current_offset))?;
        current_offset += byte_count;
    }

    Ok(slot_value.0)
}

fn load_unpacked_elements<T, S>(
    storage: &S,
    data_start: U256,
    length: usize,
) -> Result<Vec<T>>
where
    T: Storable,
    S: StorageOps,
{
    let mut elements = Vec::with_capacity(length);

    for index in 0..length {
        let slot = data_start + U256::from(index * T::SLOTS);
        let elem = T::load(storage, slot, LayoutCtx::FULL)?;
        elements.push(elem);
    }

    Ok(elements)
}

fn store_unpacked_elements<T, S>(
    elements: &[T],
    storage: &mut S,
    data_start: U256,
) -> Result<()>
where
    T: Storable,
    S: StorageOps,
{
    for (index, elem) in elements.iter().enumerate() {
        let slot = data_start + U256::from(index * T::SLOTS);
        elem.store(storage, slot, LayoutCtx::FULL)?;
    }

    Ok(())
}
