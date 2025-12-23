use alloy_primitives::U256;

use crate::{
    packing::FieldLocation,
    layout::{Handler, LayoutCtx, Storable, StorableType},
    storage::StorageOps,
    Result,
};

#[derive(Debug, Clone)]
pub struct Slot<T> {
    slot: U256,
    ctx: LayoutCtx,
    _ty: std::marker::PhantomData<T>,
}

impl<T> Slot<T> {
    #[inline]
    pub fn new(slot: U256) -> Self {
        Self {
            slot,
            ctx: LayoutCtx::FULL,
            _ty: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn new_with_ctx(slot: U256, ctx: LayoutCtx) -> Self {
        Self {
            slot,
            ctx,
            _ty: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn new_at_offset(base_slot: U256, offset_slots: usize) -> Self {
        Self {
            slot: base_slot + U256::from(offset_slots),
            ctx: LayoutCtx::FULL,
            _ty: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn new_at_loc(base_slot: U256, loc: FieldLocation) -> Self
    where
        T: StorableType,
    {
        debug_assert!(
            T::IS_PACKABLE,
            "Slot::new_at_loc can only be used with packable types"
        );
        Self {
            slot: base_slot + U256::from(loc.offset_slots),
            ctx: LayoutCtx::packed(loc.offset_bytes),
            _ty: std::marker::PhantomData,
        }
    }

    #[inline]
    pub const fn slot(&self) -> U256 {
        self.slot
    }

    #[inline]
    pub const fn offset(&self) -> Option<usize> {
        self.ctx.packed_offset()
    }
}

impl<T: Storable> Handler<T> for Slot<T> {
    fn read<S: StorageOps>(&self, storage: &S) -> Result<T> {
        T::load(storage, self.slot, self.ctx)
    }

    fn write<S: StorageOps>(&mut self, storage: &mut S, value: T) -> Result<()> {
        value.store(storage, self.slot, self.ctx)
    }

    fn delete<S: StorageOps>(&mut self, storage: &mut S) -> Result<()> {
        T::delete(storage, self.slot, self.ctx)
    }
}
