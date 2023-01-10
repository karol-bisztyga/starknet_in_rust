use std::collections::{HashMap, HashSet};

use num_bigint::BigInt;

use crate::core::errors::state_errors::StateError;
use crate::services::api::contract_class::ContractClass;

use super::state_api_objects::BlockInfo;

use super::state_api::StateReader;

/// (contract_address, key)
pub(crate) type StorageEntry = (BigInt, [u8; 32]);

#[derive(Debug, Default, Clone)]
pub(crate) struct StateCache {
    // Reader's cached information; initial values, read before any write operation (per cell)
    pub(crate) class_hash_initial_values: HashMap<BigInt, Vec<u8>>,
    pub(crate) nonce_initial_values: HashMap<BigInt, BigInt>,
    pub(crate) storage_initial_values: HashMap<StorageEntry, BigInt>,

    // Writer's cached information.
    pub(crate) class_hash_writes: HashMap<BigInt, Vec<u8>>,
    pub(crate) nonce_writes: HashMap<BigInt, BigInt>,
    pub(crate) storage_writes: HashMap<StorageEntry, BigInt>,
}

impl StateCache {
    pub(crate) fn new() -> Self {
        Self {
            class_hash_initial_values: HashMap::new(),
            nonce_initial_values: HashMap::new(),
            storage_initial_values: HashMap::new(),
            class_hash_writes: HashMap::new(),
            nonce_writes: HashMap::new(),
            storage_writes: HashMap::new(),
        }
    }

    pub(crate) fn get_class_hash(&self, contract_address: &BigInt) -> Option<&Vec<u8>> {
        if self.class_hash_writes.contains_key(contract_address) {
            return self.class_hash_writes.get(contract_address);
        }
        self.class_hash_initial_values.get(contract_address)
    }

    pub(crate) fn get_nonce(&self, contract_address: &BigInt) -> Option<&BigInt> {
        if self.nonce_writes.contains_key(contract_address) {
            return self.nonce_writes.get(contract_address);
        }
        self.nonce_initial_values.get(contract_address)
    }

    pub(crate) fn get_storage(&self, storage_entry: &StorageEntry) -> Option<&BigInt> {
        if self.storage_writes.contains_key(storage_entry) {
            return self.storage_writes.get(storage_entry);
        }
        self.storage_initial_values.get(storage_entry)
    }

    pub(crate) fn update_writes_from_other(&mut self, other: &Self) {
        self.class_hash_writes
            .extend(other.class_hash_writes.clone());
        self.nonce_writes.extend(other.nonce_writes.clone());
        self.storage_writes.extend(other.storage_writes.clone());
    }

    pub(crate) fn update_writes(
        &mut self,
        address_to_class_hash: &HashMap<BigInt, Vec<u8>>,
        address_to_nonce: &HashMap<BigInt, BigInt>,
        storage_updates: &HashMap<StorageEntry, BigInt>,
    ) {
        self.class_hash_writes.extend(address_to_class_hash.clone());
        self.nonce_writes.extend(address_to_nonce.clone());
        self.storage_writes.extend(storage_updates.clone());
    }

    pub(crate) fn set_initial_values(
        &mut self,
        address_to_class_hash: &HashMap<BigInt, Vec<u8>>,
        address_to_nonce: &HashMap<BigInt, BigInt>,
        storage_updates: &HashMap<StorageEntry, BigInt>,
    ) -> Result<(), StateError> {
        if !(self.class_hash_initial_values.is_empty()
            && self.class_hash_writes.is_empty()
            && self.nonce_initial_values.is_empty()
            && self.nonce_writes.is_empty()
            && self.storage_initial_values.is_empty()
            && self.storage_writes.is_empty())
        {
            return Err(StateError::StateCacheAlreadyInitialized);
        }
        self.update_writes(address_to_class_hash, address_to_nonce, storage_updates);
        Ok(())
    }

    pub(crate) fn get_accessed_contract_addresses(&self) -> HashSet<BigInt> {
        let mut set: HashSet<BigInt> = HashSet::with_capacity(self.class_hash_writes.len());
        set.extend(self.class_hash_writes.keys().cloned());
        set.extend(self.nonce_writes.keys().cloned());
        set.extend(self.storage_writes.keys().map(|x| x.0.clone()));
        set
    }
}

#[cfg(test)]
mod tests {
    use crate::{bigint, state};

    use super::*;

    #[test]
    fn state_chache_set_initial_values() {
        let mut state_cache = StateCache::new();
        let address_to_class_hash = HashMap::from([(bigint!(10), b"pedersen".to_vec())]);
        let address_to_nonce = HashMap::from([(bigint!(9), bigint!(12))]);
        let storage_updates = HashMap::from([((bigint!(20), [1; 32]), bigint!(18))]);
        assert!(state_cache
            .set_initial_values(&address_to_class_hash, &address_to_nonce, &storage_updates)
            .is_ok());

        assert_eq!(state_cache.class_hash_writes, address_to_class_hash);
        assert_eq!(state_cache.nonce_writes, address_to_nonce);
        assert_eq!(state_cache.storage_writes, storage_updates);

        assert_eq!(
            state_cache.get_accessed_contract_addresses(),
            HashSet::from([bigint!(10), bigint!(9), bigint!(20)])
        );
    }

    #[test]
    fn state_chache_update_writes_from_other() {
        let mut state_cache = StateCache::new();
        let address_to_class_hash = HashMap::from([(bigint!(10), b"pedersen".to_vec())]);
        let address_to_nonce = HashMap::from([(bigint!(9), bigint!(12))]);
        let storage_updates = HashMap::from([((bigint!(20), [1; 32]), bigint!(18))]);
        state_cache
            .set_initial_values(&address_to_class_hash, &address_to_nonce, &storage_updates)
            .expect("Error setting StateCache values");

        let mut other_state_cache = StateCache::new();
        let other_address_to_class_hash = HashMap::from([(bigint!(10), b"sha-3".to_vec())]);
        let other_address_to_nonce = HashMap::from([(bigint!(401), bigint!(100))]);
        let other_storage_updates = HashMap::from([((bigint!(4002), [2; 32]), bigint!(101))]);
        other_state_cache
            .set_initial_values(
                &other_address_to_class_hash,
                &other_address_to_nonce,
                &other_storage_updates,
            )
            .expect("Error setting StateCache values");

        state_cache.update_writes_from_other(&other_state_cache);

        assert_eq!(
            state_cache.get_class_hash(&bigint!(10)),
            Some(&b"sha-3".to_vec())
        );
        assert_eq!(
            state_cache.nonce_writes,
            HashMap::from([(bigint!(9), bigint!(12)), (bigint!(401), bigint!(100))])
        );
        assert_eq!(
            state_cache.storage_writes,
            HashMap::from([
                ((bigint!(20), [1; 32]), bigint!(18)),
                ((bigint!(4002), [2; 32]), bigint!(101))
            ])
        );
    }
}