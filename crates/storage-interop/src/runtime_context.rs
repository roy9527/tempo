use alloy_primitives::Address;

use crate::{
    runtime_provider::PrecompileStorageProvider,
    runtime_storage_ops::{RuntimeStorageOps, StorageMode},
};

pub struct RuntimeContext<'a, P> {
    provider: &'a mut P,
    address: Address,
}

impl<'a, P> RuntimeContext<'a, P>
where
    P: PrecompileStorageProvider,
{
    pub fn new(provider: &'a mut P, address: Address) -> Self {
        Self { provider, address }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn provider(&mut self) -> &mut P {
        self.provider
    }

    pub fn storage_ops(&mut self) -> RuntimeStorageOps<'_, P> {
        RuntimeStorageOps::new(self.provider, self.address, StorageMode::Persistent)
    }

    pub fn transient_ops(&mut self) -> RuntimeStorageOps<'_, P> {
        RuntimeStorageOps::new(self.provider, self.address, StorageMode::Transient)
    }
}
