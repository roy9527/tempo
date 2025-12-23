use std::cell::{Cell, RefCell};

use alloy_evm::{EvmInternals, EvmInternalsError};
use alloy_primitives::{Address, Log, LogData, U256};
use revm::{
    context::CfgEnv,
    interpreter::gas,
    primitives::hardfork::SpecId,
    state::{AccountInfo, Bytecode},
};

use crate::{
    InteropError,
    Result,
    runtime_provider::PrecompileStorageProvider,
};

pub struct RevmStorageProvider<'a> {
    internals: RefCell<EvmInternals<'a>>,
    chain_id: u64,
    gas_remaining: Cell<u64>,
    gas_refunded: Cell<i64>,
    gas_limit: u64,
    spec: SpecId,
    is_static: bool,
}

impl<'a> RevmStorageProvider<'a> {
    pub fn new(
        internals: EvmInternals<'a>,
        gas_limit: u64,
        chain_id: u64,
        spec: SpecId,
        is_static: bool,
    ) -> Self {
        Self {
            internals: RefCell::new(internals),
            chain_id,
            gas_remaining: Cell::new(gas_limit),
            gas_refunded: Cell::new(0),
            gas_limit,
            spec,
            is_static,
        }
    }

    pub fn new_max_gas(internals: EvmInternals<'a>, cfg: &CfgEnv<SpecId>) -> Self {
        Self::new(internals, u64::MAX, cfg.chain_id, cfg.spec, false)
    }

    fn ensure_loaded_account(&self, account: Address) -> Result<()> {
        let mut internals = self.internals.borrow_mut();
        internals.load_account(account)?;
        internals.touch_account(account);
        Ok(())
    }

    fn charge_gas(&self, gas_cost: u64) -> Result<()> {
        let remaining = self
            .gas_remaining
            .get()
            .checked_sub(gas_cost)
            .ok_or(InteropError::OutOfGas)?;
        self.gas_remaining.set(remaining);
        Ok(())
    }
}

impl<'a> PrecompileStorageProvider for RevmStorageProvider<'a> {
    type AccountInfo = AccountInfo;
    type Bytecode = Bytecode;
    type Spec = SpecId;

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn timestamp(&self) -> U256 {
        self.internals.borrow().block_timestamp()
    }

    fn beneficiary(&self) -> Address {
        self.internals.borrow().block_env().beneficiary()
    }

    fn sload(&self, address: Address, slot: U256) -> Result<U256> {
        self.ensure_loaded_account(address)?;
        let mut internals = self.internals.borrow_mut();
        let val = internals.sload(address, slot)?;

        self.charge_gas(gas::sload_cost(self.spec, val.is_cold))?;

        Ok(val.data)
    }

    fn sstore(&mut self, address: Address, slot: U256, value: U256) -> Result<()> {
        self.ensure_loaded_account(address)?;
        let mut internals = self.internals.borrow_mut();
        let result = internals.sstore(address, slot, value)?;

        self.charge_gas(gas::sstore_cost(self.spec, &result.data, result.is_cold))?;

        let refund = gas::sstore_refund(self.spec, &result.data);
        self.refund_gas(refund);
        Ok(())
    }

    fn tload(&self, address: Address, slot: U256) -> Result<U256> {
        self.charge_gas(gas::WARM_STORAGE_READ_COST)?;
        Ok(self.internals.borrow_mut().tload(address, slot))
    }

    fn tstore(&mut self, address: Address, slot: U256, value: U256) -> Result<()> {
        self.charge_gas(gas::WARM_STORAGE_READ_COST)?;
        self.internals.borrow_mut().tstore(address, slot, value);
        Ok(())
    }

    fn set_code(&mut self, address: Address, code: Bytecode) -> Result<()> {
        self.ensure_loaded_account(address)?;
        self.charge_gas(code.len() as u64 * gas::CODEDEPOSIT)?;
        self.internals.borrow_mut().set_code(address, code);
        Ok(())
    }

    fn with_account_info(
        &mut self,
        address: Address,
        f: &mut dyn FnMut(&AccountInfo),
    ) -> Result<()> {
        self.ensure_loaded_account(address)?;
        let mut internals = self.internals.borrow_mut();
        let account = internals.load_account_code(address)?.map(|a| &a.info);
        let is_cold = account.is_cold;

        self.charge_gas(gas::warm_cold_cost(is_cold))?;
        f(account.data);
        Ok(())
    }

    fn emit_event(&mut self, address: Address, log: LogData) -> Result<()> {
        let gas_cost = gas::log_cost(log.topics().len() as u8, log.data.len() as u64)
            .unwrap_or(u64::MAX);
        self.charge_gas(gas_cost)?;

        self.internals.borrow_mut().log(Log { address, data: log });
        Ok(())
    }

    fn deduct_gas(&mut self, gas: u64) -> Result<()> {
        self.charge_gas(gas)
    }

    fn refund_gas(&mut self, gas: i64) {
        let refunded = self.gas_refunded.get().saturating_add(gas);
        self.gas_refunded.set(refunded);
    }

    fn gas_used(&self) -> u64 {
        self.gas_limit - self.gas_remaining.get()
    }

    fn gas_refunded(&self) -> i64 {
        self.gas_refunded.get()
    }

    fn spec(&self) -> SpecId {
        self.spec
    }

    fn is_static(&self) -> bool {
        self.is_static
    }
}

impl From<EvmInternalsError> for InteropError {
    fn from(value: EvmInternalsError) -> Self {
        Self::RuntimeError(value.to_string())
    }
}
