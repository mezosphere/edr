//! RPC types for hardhat_dumpState and hardhat_loadState methods.

use edr_primitives::{Address, Bytes, HashMap, U256};
use serde::{Deserialize, Serialize};

fn is_bytes_empty(bytes: &Bytes) -> bool {
    bytes.is_empty()
}

/// Account state for dump/load state operations.
/// Uses Anvil-compatible format for interoperability.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct StateAccount {
    /// Account balance as hex string
    pub balance: U256,
    /// Account bytecode (empty for EOAs)
    #[serde(default, skip_serializing_if = "is_bytes_empty")]
    pub code: Bytes,
    /// Account nonce
    pub nonce: U256,
    /// Account storage slots
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub storage: HashMap<U256, U256>,
}

/// State dump result containing all accounts.
/// Uses Anvil-compatible format.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct StateDump {
    /// Map of address to account state
    pub accounts: HashMap<Address, StateAccount>,
}

impl StateDump {
    /// Creates a new empty state dump.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an account to the state dump.
    pub fn add_account(&mut self, address: Address, account: StateAccount) {
        self.accounts.insert(address, account);
    }
}
