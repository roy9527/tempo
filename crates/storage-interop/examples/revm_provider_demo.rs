#[cfg(feature = "revm")]
mod demo {
    use alloy_evm::{EvmEnv, EvmFactory, EvmInternals};
    use alloy_primitives::{Address, U256};
    use revm::context::CfgEnv;
    use revm::database::{CacheDB, EmptyDB};
    use revm::primitives::hardfork::SpecId;

    use tempo_storage_interop::{RevmStorageProvider, RuntimeContext, Slot};

    pub fn run() -> tempo_storage_interop::Result<()> {
        let db = CacheDB::new(EmptyDB::new());
        let mut evm = EvmFactory::default().create_evm(db, EvmEnv::default());
        let ctx = evm.ctx_mut();

        let internals = EvmInternals::new(&mut ctx.journaled_state, &ctx.block);
        let mut provider = RevmStorageProvider::new_max_gas(
            internals,
            &CfgEnv::<SpecId> {
                chain_id: ctx.cfg.chain_id,
                spec: ctx.cfg.spec,
                ..Default::default()
            },
        );

        let contract = Address::random();
        let mut runtime = RuntimeContext::new(&mut provider, contract);
        let mut ops = runtime.storage_ops();

        let mut slot = Slot::<U256>::new(U256::from(0));
        slot.write(&mut ops, U256::from(42))?;
        let loaded = slot.read(&ops)?;
        assert_eq!(loaded, U256::from(42));

        Ok(())
    }
}

#[cfg(feature = "revm")]
fn main() -> tempo_storage_interop::Result<()> {
    demo::run()
}

#[cfg(not(feature = "revm"))]
fn main() {
    eprintln!("revm feature disabled: run with --features revm");
}
