use edr_primitives::{Address, HashMap, KECCAK_EMPTY, U256};

use crate::{
    account::{Account, AccountInfo, AccountStatus},
    EvmStorageSlot,
};

/// The difference between two states, which can be applied to a state to get
/// the new state using [`crate::StateCommit::commit`].
#[derive(Clone, Debug, Default)]
pub struct StateDiff {
    inner: HashMap<Address, Account>,
}

/// Checks if the account info has code (non-empty code hash).
fn account_has_code(account_info: &AccountInfo) -> bool {
    account_info.code_hash != KECCAK_EMPTY
}

impl StateDiff {
    /// Applies a single change to this instance, combining it with any existing
    /// change.
    pub fn apply_account_change(&mut self, address: Address, account_info: AccountInfo) {
        // Determine if this account should be marked as Created (has code)
        let new_account_has_code = account_has_code(&account_info);

        self.inner
            .entry(address)
            .and_modify(|account| {
                // If code is being added, mark as Created
                if new_account_has_code && !account_has_code(&account.info) {
                    account.status.insert(AccountStatus::Created);
                }
                account.info = account_info.clone();
            })
            .or_insert_with(|| {
                let status = if new_account_has_code {
                    AccountStatus::Created | AccountStatus::Touched
                } else {
                    AccountStatus::Touched
                };
                Account {
                    info: account_info,
                    storage: HashMap::default(),
                    status,
                    transaction_id: 0,
                }
            });
    }

    /// Applies a single storage change to this instance, combining it with any
    /// existing change.
    ///
    /// If the account corresponding to the specified address hasn't been
    /// modified before, either the value provided in `account_info` will be
    /// used, or alternatively a default account will be created.
    pub fn apply_storage_change(
        &mut self,
        address: Address,
        index: U256,
        slot: EvmStorageSlot,
        account_info: Option<AccountInfo>,
    ) {
        self.inner
            .entry(address)
            .and_modify(|account| {
                account.storage.insert(index, slot.clone());
            })
            .or_insert_with(|| {
                let storage: HashMap<_, _> = std::iter::once((index, slot.clone())).collect();

                Account {
                    info: account_info.unwrap_or_default(),
                    storage,
                    status: AccountStatus::Created | AccountStatus::Touched,
                    transaction_id: 0,
                }
            });
    }

    /// Applies a state diff to this instance, combining with any and all
    /// existing changes.
    pub fn apply_diff(&mut self, diff: HashMap<Address, Account>) {
        for (address, account_diff) in diff {
            self.inner
                .entry(address)
                .and_modify(|account| {
                    account.info = account_diff.info.clone();
                    account.status.insert(account_diff.status);
                    account.storage.extend(account_diff.storage.clone());
                })
                .or_insert(account_diff);
        }
    }

    /// Retrieves the inner hash map.
    pub fn as_inner(&self) -> &HashMap<Address, Account> {
        &self.inner
    }
}

impl From<HashMap<Address, Account>> for StateDiff {
    fn from(value: HashMap<Address, Account>) -> Self {
        Self { inner: value }
    }
}

impl From<StateDiff> for HashMap<Address, Account> {
    fn from(value: StateDiff) -> Self {
        value.inner
    }
}

#[cfg(test)]
mod tests {
    use edr_primitives::Bytecode;

    use super::*;

    /// Creates an AccountInfo without code (like an EOA)
    fn account_info_without_code(balance: U256, nonce: u64) -> AccountInfo {
        AccountInfo {
            balance,
            nonce,
            code_hash: KECCAK_EMPTY,
            code: None,
        }
    }

    /// Creates an AccountInfo with code (like a contract)
    fn account_info_with_code(balance: U256, nonce: u64, code: Bytecode) -> AccountInfo {
        AccountInfo {
            balance,
            nonce,
            code_hash: code.hash_slow(),
            code: Some(code),
        }
    }

    #[test]
    fn apply_account_change_without_code_sets_touched_status() {
        let mut diff = StateDiff::default();
        let address = Address::random();
        let account_info = account_info_without_code(U256::from(1000), 0);

        diff.apply_account_change(address, account_info);

        let account = diff.as_inner().get(&address).expect("account should exist");
        assert_eq!(account.status, AccountStatus::Touched);
        assert!(!account.status.contains(AccountStatus::Created));
    }

    #[test]
    fn apply_account_change_with_code_sets_created_status() {
        let mut diff = StateDiff::default();
        let address = Address::random();
        let code = Bytecode::new_raw(vec![0x60, 0x00, 0x60, 0x00, 0xf3].into()); // Simple contract
        let account_info = account_info_with_code(U256::from(1000), 1, code);

        diff.apply_account_change(address, account_info);

        let account = diff.as_inner().get(&address).expect("account should exist");
        assert!(
            account.status.contains(AccountStatus::Created),
            "account with code should have Created status"
        );
        assert!(
            account.status.contains(AccountStatus::Touched),
            "account should also have Touched status"
        );
    }

    /// This test verifies the fix for the hardhat_loadState bug where:
    /// 1. set_balance creates an account with Touched status (no code)
    /// 2. set_code adds code to the account
    /// 3. The account should now have Created status so that state reconstruction works
    #[test]
    fn apply_account_change_adding_code_to_existing_account_sets_created_status() {
        let mut diff = StateDiff::default();
        let address = Address::random();

        // Step 1: Simulate set_balance - creates account without code
        let account_info_balance = account_info_without_code(U256::from(1000), 0);
        diff.apply_account_change(address, account_info_balance);

        // Verify initial state: only Touched, no Created
        let account = diff.as_inner().get(&address).expect("account should exist");
        assert_eq!(
            account.status,
            AccountStatus::Touched,
            "account without code should only have Touched status"
        );

        // Step 2: Simulate set_code - adds code to existing account
        let code = Bytecode::new_raw(vec![0x60, 0x00, 0x60, 0x00, 0xf3].into());
        let account_info_with_code = account_info_with_code(U256::from(1000), 0, code.clone());
        diff.apply_account_change(address, account_info_with_code);

        // Verify final state: should now have Created status
        let account = diff.as_inner().get(&address).expect("account should exist");
        assert!(
            account.status.contains(AccountStatus::Created),
            "account should have Created status after code is added"
        );
        assert!(
            account.status.contains(AccountStatus::Touched),
            "account should retain Touched status"
        );

        // Verify the code is present
        assert!(account.info.code.is_some(), "account should have code");
        assert_ne!(
            account.info.code_hash, KECCAK_EMPTY,
            "code_hash should not be empty"
        );
    }

    /// Test that updating an account that already has code doesn't lose Created status
    #[test]
    fn apply_account_change_updating_account_with_code_preserves_created_status() {
        let mut diff = StateDiff::default();
        let address = Address::random();

        // Create account with code
        let code = Bytecode::new_raw(vec![0x60, 0x00, 0x60, 0x00, 0xf3].into());
        let account_info = account_info_with_code(U256::from(1000), 1, code.clone());
        diff.apply_account_change(address, account_info);

        // Update the same account with new code
        let new_code = Bytecode::new_raw(vec![0x60, 0x01, 0x60, 0x00, 0xf3].into());
        let updated_account_info = account_info_with_code(U256::from(2000), 2, new_code.clone());
        diff.apply_account_change(address, updated_account_info);

        // Verify Created status is preserved
        let account = diff.as_inner().get(&address).expect("account should exist");
        assert!(
            account.status.contains(AccountStatus::Created),
            "account should retain Created status"
        );
        assert_eq!(account.info.balance, U256::from(2000));
        assert_eq!(account.info.nonce, 2);
    }

    /// Test the full load_state simulation: balance -> nonce -> code -> storage
    #[test]
    fn simulate_load_state_with_contract() {
        let mut diff = StateDiff::default();
        let address = Address::random();

        // Step 1: set_balance
        let account_info_1 = account_info_without_code(U256::from(1000), 0);
        diff.apply_account_change(address, account_info_1);

        // Step 2: set_nonce (still no code)
        let account_info_2 = AccountInfo {
            balance: U256::from(1000),
            nonce: 5,
            code_hash: KECCAK_EMPTY,
            code: None,
        };
        diff.apply_account_change(address, account_info_2);

        // Verify: still only Touched
        let account = diff.as_inner().get(&address).unwrap();
        assert!(!account.status.contains(AccountStatus::Created));

        // Step 3: set_code
        let code = Bytecode::new_raw(vec![0x60, 0x00, 0x60, 0x00, 0xf3].into());
        let account_info_3 = account_info_with_code(U256::from(1000), 5, code.clone());
        diff.apply_account_change(address, account_info_3);

        // Verify: now has Created
        let account = diff.as_inner().get(&address).unwrap();
        assert!(
            account.status.contains(AccountStatus::Created),
            "after set_code, account should have Created status"
        );

        // Step 4: set_storage (using apply_storage_change)
        let slot = EvmStorageSlot::new(U256::from(42), 0);
        diff.apply_storage_change(address, U256::from(0), slot, None);

        // Verify: Created status is preserved after storage change
        let account = diff.as_inner().get(&address).unwrap();
        assert!(
            account.status.contains(AccountStatus::Created),
            "Created status should be preserved after storage change"
        );
        assert!(account.info.code.is_some(), "code should be preserved");
    }
}
