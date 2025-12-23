use std::collections::HashMap;

use alloy_primitives::{Address, U256};

use tempo_storage_interop::{
    FieldLocation, StorageKey, StorageOps, extract_packed_value, insert_packed_value,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PolicyData {
    policy_type: u8,
    admin: Address,
}

impl PolicyData {
    const POLICY_TYPE_LOC: FieldLocation = FieldLocation::new(0, 0, 1);
    const ADMIN_LOC: FieldLocation = FieldLocation::new(0, 1, 20);

    fn encode(&self) -> tempo_storage_interop::Result<U256> {
        let encoded = insert_packed_value(
            U256::ZERO,
            &self.policy_type,
            Self::POLICY_TYPE_LOC.offset_bytes,
            Self::POLICY_TYPE_LOC.size,
        )?;

        insert_packed_value(
            encoded,
            &self.admin,
            Self::ADMIN_LOC.offset_bytes,
            Self::ADMIN_LOC.size,
        )
    }

    fn decode(slot: U256) -> tempo_storage_interop::Result<Self> {
        let policy_type = extract_packed_value::<u8>(
            slot,
            Self::POLICY_TYPE_LOC.offset_bytes,
            Self::POLICY_TYPE_LOC.size,
        )?;
        let admin = extract_packed_value::<Address>(
            slot,
            Self::ADMIN_LOC.offset_bytes,
            Self::ADMIN_LOC.size,
        )?;
        Ok(Self { policy_type, admin })
    }
}

#[derive(Default)]
struct MemoryStorage {
    slots: HashMap<U256, U256>,
}

impl StorageOps for MemoryStorage {
    fn load(&self, slot: U256) -> tempo_storage_interop::Result<U256> {
        Ok(*self.slots.get(&slot).unwrap_or(&U256::ZERO))
    }

    fn store(&mut self, slot: U256, value: U256) -> tempo_storage_interop::Result<()> {
        self.slots.insert(slot, value);
        Ok(())
    }
}

fn mapping_slot(policy_id: U256, base_slot: U256) -> U256 {
    policy_id.mapping_slot(base_slot)
}

fn main() -> tempo_storage_interop::Result<()> {
    let mut storage = MemoryStorage::default();

    // Simplified TIP403 layout
    let policy_data_base_slot = U256::from(1);
    let policy_id = U256::from(2);

    let data = PolicyData {
        policy_type: 1,
        admin: Address::random(),
    };

    let slot = mapping_slot(policy_id, policy_data_base_slot);
    storage.store(slot, data.encode()?)?;

    let loaded_raw = storage.load(slot)?;
    let decoded = PolicyData::decode(loaded_raw)?;
    assert_eq!(decoded, data);

    // Example of packed field access: policy_type at offset 0
    let policy_type = extract_packed_value::<u8>(loaded_raw, 0, 1)?;
    assert_eq!(policy_type, 1);

    // Example of manual packed write into slot
    let updated = insert_packed_value(loaded_raw, &2u8, 0, 1)?;
    storage.store(slot, updated)?;
    let updated_raw = storage.load(slot)?;
    let updated_data = PolicyData::decode(updated_raw)?;
    assert_eq!(updated_data.policy_type, 2);

    Ok(())
}
