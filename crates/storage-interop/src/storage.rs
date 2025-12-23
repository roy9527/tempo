use alloy_primitives::{Address, B256, U256, keccak256};

use crate::Result;

pub trait StorageOps {
    fn load(&self, slot: U256) -> Result<U256>;
    fn store(&mut self, slot: U256, value: U256) -> Result<()>;
}

pub trait StorageKey {
    fn as_storage_bytes(&self) -> impl AsRef<[u8]>;

    fn mapping_slot(&self, slot: U256) -> U256 {
        let key_bytes = self.as_storage_bytes();
        let key_bytes = key_bytes.as_ref();
        let padded_len = key_bytes.len().div_ceil(32) * 32;
        let mut buf = vec![0u8; padded_len + 32];

        buf[padded_len - key_bytes.len()..padded_len].copy_from_slice(key_bytes);
        buf[padded_len..].copy_from_slice(&slot.to_be_bytes::<32>());

        U256::from_be_bytes(keccak256(&buf).0)
    }
}

impl StorageKey for Address {
    fn as_storage_bytes(&self) -> impl AsRef<[u8]> {
        self.as_slice()
    }
}

impl StorageKey for B256 {
    fn as_storage_bytes(&self) -> impl AsRef<[u8]> {
        self.as_slice()
    }
}

impl StorageKey for U256 {
    fn as_storage_bytes(&self) -> impl AsRef<[u8]> {
        self.to_be_bytes::<32>()
    }
}
