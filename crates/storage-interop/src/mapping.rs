use alloy_primitives::U256;
use std::marker::PhantomData;

use crate::{
    layout::{Layout, LayoutCtx, StorableType},
    storage::StorageKey,
};

#[derive(Debug, Clone)]
pub struct Mapping<K, V> {
    base_slot: U256,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> Mapping<K, V> {
    #[inline]
    pub fn new(base_slot: U256) -> Self {
        Self {
            base_slot,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn slot(&self) -> U256 {
        self.base_slot
    }

    pub fn at(&self, key: K) -> V::Handler
    where
        K: StorageKey,
        V: StorableType,
    {
        V::handle(key.mapping_slot(self.base_slot), LayoutCtx::FULL)
    }

    #[inline]
    pub fn at_offset(struct_base_slot: U256, field_offset_slots: usize, key: K) -> V::Handler
    where
        K: StorageKey,
        V: StorableType,
    {
        let field_slot = struct_base_slot + U256::from(field_offset_slots);
        V::handle(key.mapping_slot(field_slot), LayoutCtx::FULL)
    }
}

impl<K, V> Default for Mapping<K, V> {
    fn default() -> Self {
        Self::new(U256::ZERO)
    }
}

impl<K, V> StorableType for Mapping<K, V> {
    const LAYOUT: Layout = Layout::Slots(1);
    type Handler = Self;

    fn handle(slot: U256, _ctx: LayoutCtx) -> Self::Handler {
        Self::new(slot)
    }
}
