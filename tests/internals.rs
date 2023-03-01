use felt::{felt_str, Felt};
use lazy_static::lazy_static;
use num_traits::{Num, Zero};
use starknet_rs::{
    business_logic::{
        execution::execution_entry_point::ExecutionEntryPoint,
        fact_state::{contract_state::ContractState, in_memory_state_reader::InMemoryStateReader},
        state::{cached_state::CachedState, state_api::StateReader},
        transaction::objects::internal_invoke_function::InternalInvokeFunction,
    },
    definitions::{
        constants::EXECUTE_ENTRY_POINT_SELECTOR, general_config::StarknetGeneralConfig,
        transaction_type::TransactionType,
    },
    services::api::contract_class::{ContractClass, EntryPointType},
    starknet_storage::dict_storage::DictStorage,
    utils::{felt_to_hash, Address},
};
use std::{collections::HashMap, path::PathBuf};

const ACCOUNT_CONTRACT_PATH: &str = "starknet_programs/account_without_validation.json";
const ERC20_CONTRACT_PATH: &str = "starknet_programs/erc20_contract_without_some_syscalls.json";
const TEST_CONTRACT_PATH: &str = "starknet_programs/test_contract.json";

lazy_static! {
    // Addresses.
    static ref TEST_ACCOUNT_CONTRACT_ADDRESS: Address = Address(felt_str!("257"));
    static ref TEST_CONTRACT_ADDRESS: Address = Address(felt_str!("256"));

    // Class hashes.
    static ref TEST_ACCOUNT_CONTRACT_CLASS_HASH: Felt = felt_str!("273");
    static ref TEST_CLASS_HASH: Felt = felt_str!("272");
    static ref TEST_ERC20_CONTRACT_CLASS_HASH: Felt = felt_str!("4112");

    // Storage keys.
    static ref TEST_ERC20_ACCOUNT_BALANCE_KEY: Felt =
        felt_str!("1192211877881866289306604115402199097887041303917861778777990838480655617515");
    static ref TEST_ERC20_SEQUENCER_BALANCE_KEY: Felt =
        felt_str!("3229073099929281304021185011369329892856197542079132996799046100564060768274");

    // Others.
    static ref ACTUAL_FEE: Felt = 2.into();
}

fn get_contract_class<P>(path: P) -> Result<ContractClass, Box<dyn std::error::Error>>
where
    P: Into<PathBuf>,
{
    Ok(ContractClass::try_from(path.into())?)
}

#[allow(dead_code)]
fn create_account_tx_test_state(
) -> Result<(StarknetGeneralConfig, CachedState<InMemoryStateReader>), Box<dyn std::error::Error>> {
    let general_config = StarknetGeneralConfig::default();

    let test_contract_class_hash = TEST_CLASS_HASH.clone();
    let test_account_class_hash = TEST_ACCOUNT_CONTRACT_CLASS_HASH.clone();
    let test_erc20_class_hash = TEST_ERC20_CONTRACT_CLASS_HASH.clone();
    let class_hash_to_class = HashMap::from([
        (
            test_account_class_hash.clone(),
            get_contract_class(ACCOUNT_CONTRACT_PATH)?,
        ),
        (
            test_contract_class_hash.clone(),
            get_contract_class(TEST_CONTRACT_PATH)?,
        ),
        (
            test_erc20_class_hash.clone(),
            get_contract_class(ERC20_CONTRACT_PATH)?,
        ),
    ]);

    let test_contract_address = TEST_CONTRACT_ADDRESS.clone();
    let test_account_address = TEST_ACCOUNT_CONTRACT_ADDRESS.clone();
    let test_erc20_address = general_config
        .starknet_os_config()
        .fee_token_address()
        .clone();
    let address_to_class_hash = HashMap::from([
        (test_contract_address, test_contract_class_hash),
        (test_account_address, test_account_class_hash),
        (test_erc20_address.clone(), test_erc20_class_hash),
    ]);

    let test_erc20_account_balance_key = TEST_ERC20_ACCOUNT_BALANCE_KEY.clone();
    let test_erc20_sequencer_balance_key = TEST_ERC20_SEQUENCER_BALANCE_KEY.clone();
    let storage_view = HashMap::from([
        (
            (test_erc20_address.clone(), test_erc20_sequencer_balance_key),
            Felt::zero(),
        ),
        (
            (test_erc20_address, test_erc20_account_balance_key),
            ACTUAL_FEE.clone(),
        ),
    ]);

    let cached_state = CachedState::new(
        {
            let mut state_reader = InMemoryStateReader::new(DictStorage::new(), DictStorage::new());

            for (contract_address, class_hash) in address_to_class_hash {
                let storage_keys = storage_view
                    .iter()
                    .filter_map(|((k0, k1), v)| {
                        (k0 == &contract_address).then_some((k1.clone(), v.clone()))
                    })
                    .collect();

                state_reader.contract_states_mut().insert(
                    contract_address,
                    ContractState::new(felt_to_hash(&class_hash), Felt::zero(), storage_keys),
                );
            }

            state_reader
        },
        Some(
            class_hash_to_class
                .into_iter()
                .map(|(k, v)| (felt_to_hash(&k), v))
                .collect(),
        ),
    );

    Ok((general_config, cached_state))
}

#[test]
fn test_create_account_tx_test_state() {
    let (general_config, mut state) = create_account_tx_test_state().unwrap();

    println!("{}", serde_json::to_string(&Felt::zero()).unwrap());

    let value = state
        .get_storage_at(&(
            general_config
                .starknet_os_config()
                .fee_token_address()
                .clone(),
            felt_to_hash(&*TEST_ERC20_ACCOUNT_BALANCE_KEY),
        ))
        .unwrap();
    println!("value = {:?}", value);
    assert_eq!(value, &2.into());

    let class_hash = state.get_class_hash_at(&*TEST_CONTRACT_ADDRESS).unwrap();
    println!("value = {class_hash:?}");
    assert_eq!(class_hash, &felt_to_hash(&*TEST_CLASS_HASH));

    let contract_class = state
        .get_contract_class(&felt_to_hash(&*TEST_ERC20_CONTRACT_CLASS_HASH))
        .unwrap();
    // println!("value = {contract_class:?}");
    assert_eq!(
        contract_class,
        get_contract_class(ERC20_CONTRACT_PATH).unwrap()
    );
}

fn invoke_tx() -> InternalInvokeFunction {
    // TODO Implement selector_from_name
    let entry_point_selector = Felt::from_str_radix(
        "1629174963900209270929724181518648239607275954724935556801269374828746872577",
        10,
    )
    .unwrap();

    let execute_call_data = vec![
        TEST_CONTRACT_ADDRESS.clone().0,
        entry_point_selector.clone(),
        1.into(),
        2.into(),
    ];

    InternalInvokeFunction::new(
        TEST_ACCOUNT_CONTRACT_ADDRESS.clone(),
        EXECUTE_ENTRY_POINT_SELECTOR.clone(),
        EntryPointType::External, //TODO Check this
        execute_call_data,
        TransactionType::InvokeFunction,
        1,
        0.into(),
        0.into(),
        Vec::new(),
        2,
        0.into(),
    )
}

#[test]
fn test_invoke_tx() {
    let (general_config, mut state) = create_account_tx_test_state().unwrap();

    let mut invoke_tx = invoke_tx();

    dbg!(&invoke_tx);
    let calldata = invoke_tx.calldata.clone();
    let sender_address = invoke_tx.contract_address.clone();

    let result = invoke_tx._apply_specific_concurrent_changes(&mut state, &general_config);

    dbg!(&result);

    todo!()
}
