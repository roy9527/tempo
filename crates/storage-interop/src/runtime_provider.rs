use alloy_primitives::{Address, LogData, U256};

use crate::Result;

pub trait PrecompileStorageProvider {
    type AccountInfo;
    type Bytecode;
    type Spec;

    fn chain_id(&self) -> u64;
    fn timestamp(&self) -> U256;
    fn beneficiary(&self) -> Address;
    fn is_static(&self) -> bool;

    fn sload(&self, address: Address, slot: U256) -> Result<U256>;
    fn sstore(&mut self, address: Address, slot: U256, value: U256) -> Result<()>;

    fn tload(&self, address: Address, slot: U256) -> Result<U256>;
    fn tstore(&mut self, address: Address, slot: U256, value: U256) -> Result<()>;

    fn set_code(&mut self, address: Address, code: Self::Bytecode) -> Result<()>;
    fn with_account_info(
        &mut self,
        address: Address,
        f: &mut dyn FnMut(&Self::AccountInfo),
    ) -> Result<()>;

    fn emit_event(&mut self, address: Address, log: LogData) -> Result<()>;

    fn deduct_gas(&mut self, gas: u64) -> Result<()>;
    fn refund_gas(&mut self, gas: i64);

    fn gas_used(&self) -> u64;
    fn gas_refunded(&self) -> i64;

    fn spec(&self) -> Self::Spec;
}
