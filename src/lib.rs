#![deny(warnings)]

use business_logic::{
    execution::{
        execution_entry_point::ExecutionEntryPoint,
        objects::{CallInfo, CallType, TransactionExecutionContext, TransactionExecutionInfo},
    },
    fact_state::state::ExecutionResourcesManager,
    state::{
        cached_state::CachedState,
        state_api::{State, StateReader},
    },
    transaction::{error::TransactionError, transactions::Transaction},
};
use definitions::general_config::StarknetGeneralConfig;
use felt::Felt;
use services::api::contract_class::EntryPointType;
use utils::{Address, ClassHash};

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

pub mod business_logic;
pub mod core;
pub mod definitions;
pub mod hash_utils;
pub mod public;
pub mod services;
pub mod starknet_runner;
pub mod starknet_storage;
pub mod starkware_utils;
pub mod testing;
pub mod utils;

type TransactionResult<T> = Result<T, TransactionError>;

pub struct SimulationFlags;

pub struct Starknet;

impl Starknet {
    pub fn call_contract<T>(
        state: &mut CachedState<T>,
        tx: Transaction,
        config: &StarknetGeneralConfig,
    ) -> TransactionResult<Vec<Felt>>
    where
        T: State + StateReader + Clone + Default,
    {
        let mut state_copy = state.clone();
        tx.execute(&mut state_copy, config)
            .and_then(|tx_exec| {
                tx_exec
                    .call_info
                    .ok_or(TransactionError::StarknetError(
                        "Empty CallInfo.".to_string(),
                    ))
                    .map(|r| r.retdata)
            })
            .map_err(Into::into)
    }

    pub fn estimate_fee<T>(
        state: &CachedState<T>,
        tx: Transaction,
        config: &StarknetGeneralConfig,
    ) -> TransactionResult<u64>
    where
        T: State + StateReader + Clone + Default,
    {
        let mut state_copy = state.clone();
        // TODO: check if the estimate_fee is the actual_fee.
        tx.execute(&mut state_copy, config)
            .map(|tx_exec| tx_exec.actual_fee)
            .map_err(Into::into)
    }

    pub fn execute_tx<T>(
        &self,
        state: &mut CachedState<T>,
        tx: Transaction,
        config: &StarknetGeneralConfig,
    ) -> TransactionResult<TransactionExecutionInfo>
    where
        T: State + StateReader + Clone + Default,
    {
        tx.execute(state, config).map_err(Into::into)
    }

    pub fn simulate_tx<T>(
        state: &CachedState<T>,
        tx: Transaction,
        config: &StarknetGeneralConfig,
        _options: Option<SimulationFlags>,
    ) -> TransactionResult<(TransactionExecutionInfo, u64)>
    where
        T: State + StateReader + Clone + Default,
    {
        let mut state_copy = state.clone();
        // TODO: check if the estimate_fee is the actual_fee.
        tx.execute(&mut state_copy, config)
            .map(|tx_exec| (tx_exec.clone(), tx_exec.actual_fee))
            .map_err(Into::into)
    }
}
