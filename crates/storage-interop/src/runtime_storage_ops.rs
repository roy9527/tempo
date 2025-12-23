use alloy_primitives::{Address, U256};

use crate::{
    runtime_provider::PrecompileStorageProvider,
    storage::StorageOps,
    Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    Persistent,
    Transient,
}

pub struct RuntimeStorageOps<'a, P> {
    provider: &'a mut P,
    address: Address,
    mode: StorageMode,
}

impl<'a, P> RuntimeStorageOps<'a, P>
where
    P: PrecompileStorageProvider,
{
    pub fn new(provider: &'a mut P, address: Address, mode: StorageMode) -> Self {
        Self {
            provider,
            address,
            mode,
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn mode(&self) -> StorageMode {
        self.mode
    }
}

impl<'a, P> StorageOps for RuntimeStorageOps<'a, P>
where
    P: PrecompileStorageProvider,
{
    fn load(&self, slot: U256) -> Result<U256> {
        match self.mode {
            StorageMode::Persistent => self.provider.sload(self.address, slot),
            StorageMode::Transient => self.provider.tload(self.address, slot),
        }
    }

    fn store(&mut self, slot: U256, value: U256) -> Result<()> {
        match self.mode {
            StorageMode::Persistent => self.provider.sstore(self.address, slot, value),
            StorageMode::Transient => self.provider.tstore(self.address, slot, value),
        }
    }
}
