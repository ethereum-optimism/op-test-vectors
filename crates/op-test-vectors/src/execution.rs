//! Module containing the execution test fixture.

use alloy::primitives::{Address, Bloom, B256, U256};
use alloy::rpc::types::trace::geth::AccountState;
use alloy::rpc::types::{Log, TransactionReceipt};
use anvil_core::eth::block::Block;
use anvil_core::eth::transaction::{TypedReceipt, TypedTransaction};
use color_eyre::eyre;
use op_alloy_consensus::{OpDepositReceipt, OpDepositReceiptWithBloom, OpReceiptEnvelope};
use op_alloy_rpc_types::{receipt, OpTransactionReceipt, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The execution fixture is the top-level object that contains
/// everything needed to run an execution test.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ExecutionFixture {
    /// The execution environment sets up the current block context.
    pub env: ExecutionEnvironment,
    /// The initial state of the accounts before running the transactions, also called the
    /// "pre-state".
    pub alloc: HashMap<Address, AccountState>,
    /// The expected state of the accounts after running the transactions, also called the
    /// "post-state".
    pub out_alloc: HashMap<Address, AccountState>,
    /// Transactions to execute.
    #[serde(rename = "txs")]
    pub transactions: Vec<TypedTransaction>,
    /// The expected result after executing transactions.
    pub result: ExecutionResult,
}

/// The execution environment is the initial state of the execution context.
/// It's used to set the execution environment current block information.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEnvironment {
    /// The current block coinbase.
    pub current_coinbase: Address,
    /// The current block difficulty.
    pub current_difficulty: U256,
    /// The current block gas limit.
    pub current_gas_limit: U256,
    /// The previous block hash.
    pub previous_hash: B256,
    /// The current block number.
    pub current_number: U256,
    /// The current block timestamp.
    pub current_timestamp: U256,
    /// The block hashes of the previous blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hashes: Option<HashMap<U256, B256>>,
}

impl From<Block> for ExecutionEnvironment {
    fn from(block: Block) -> Self {
        Self {
            current_coinbase: block.header.beneficiary,
            current_difficulty: block.header.difficulty,
            current_gas_limit: U256::from(block.header.gas_limit),
            previous_hash: block.header.parent_hash,
            current_number: U256::from(block.header.number),
            current_timestamp: U256::from(block.header.timestamp),
            block_hashes: None,
        }
    }
}

/// The execution result is the expected result after running the transactions
/// in the execution environment over the pre-state.
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    /// The state root.
    pub state_root: B256,
    /// The transaction root.
    pub tx_root: B256,
    /// The receipt root.
    pub receipt_root: B256,
    /// The logs hash.
    pub logs_hash: B256,
    /// The logs bloom.
    pub logs_bloom: Bloom,
    /// A list of execution receipts for each executed transaction.
    pub receipts: Vec<ExecutionReceipt>,
}

/// An execution receipt is the result of running a transaction in the execution environment.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReceipt {
    /// Cumulative gas used in the block including this transaction
    pub cumulative_gas_used: U256,
    /// The inner log receipt.
    #[serde(flatten)]
    pub inner: TransactionReceipt<OpReceiptEnvelope<Log>>,
}

impl From<OpTransactionReceipt> for ExecutionReceipt {
    fn from(receipt: OpTransactionReceipt) -> Self {
        Self {
            cumulative_gas_used: U256::from(receipt.inner.gas_used),
            inner: receipt.inner,
        }
    }
}

impl From<TransactionReceipt<TypedReceipt<alloy::primitives::Log>>> for ExecutionReceipt {
    fn from(receipt: TransactionReceipt<TypedReceipt<alloy::primitives::Log>>) -> Self {
        let inner = match receipt.inner {
            TypedReceipt::Legacy(receipt) => OpReceiptEnvelope::Legacy(receipt),
            TypedReceipt::EIP2930(receipt) => OpReceiptEnvelope::Eip2930(receipt),
            TypedReceipt::EIP1559(receipt) => OpReceiptEnvelope::Eip1559(receipt),
            TypedReceipt::EIP4844(receipt) => OpReceiptEnvelope::Eip4844(receipt),
            TypedReceipt::Deposit(receipt) => {
                let op_deposit_receipt_with_bloom = OpDepositReceiptWithBloom {
                    receipt: OpDepositReceipt {
                        deposit_nonce: receipt.deposit_nonce,
                        deposit_receipt_version: receipt.deposit_receipt_version,
                        inner: receipt.inner.receipt,
                    },
                    logs_bloom: receipt.inner.logs_bloom,
                };

                OpReceiptEnvelope::Deposit(op_deposit_receipt_with_bloom)
            }
        };

        let transaction_receipt = TransactionReceipt {
            transaction_hash: receipt.transaction_hash,
            transaction_index: receipt.transaction_index,
            block_hash: receipt.block_hash,
            block_number: receipt.block_number,
            gas_used: receipt.gas_used,
            effective_gas_price: receipt.effective_gas_price,
            blob_gas_price: receipt.blob_gas_price,
            blob_gas_used: receipt.blob_gas_used,
            contract_address: receipt.contract_address,
            from: receipt.from,
            to: receipt.to,
            state_root: receipt.state_root,

            inner,
        };

        Self {
            cumulative_gas_used: U256::from(receipt.gas_used),
            inner: transaction_receipt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_serialize_execution_environment() {
        let expected_env = include_str!("./testdata/environment.json");
        let env = serde_json::from_str::<ExecutionEnvironment>(expected_env)
            .expect("failed to parse environment");
        let serialized_env = serde_json::to_string(&env).expect("failed to serialize environment");
        let serialized_value = serde_json::from_str::<Value>(&serialized_env)
            .expect("failed to parse serialized environment");
        let expected_value = serde_json::from_str::<Value>(expected_env)
            .expect("failed to parse expected environment");
        assert_eq!(serialized_value, expected_value);
    }

    #[test]
    fn test_serialize_execution_result() {
        let expected_result = include_str!("./testdata/result.json");
        let execution_result = serde_json::from_str::<ExecutionResult>(expected_result)
            .expect("failed to parse result");
        let serialized_result =
            serde_json::to_string(&execution_result).expect("failed to serialize result");
        let serialized_value = serde_json::from_str::<Value>(&serialized_result)
            .expect("failed to parse serialized result");
        let expected_value = serde_json::from_str::<Value>(expected_result)
            .expect("failed to parse expected result");
        assert_eq!(serialized_value, expected_value);
    }

    #[test]
    fn test_exec_receipt_from_tx_receipt() {
        let tx_receipt_str = include_str!("./testdata/tx_receipt.json");
        let tx_receipt: OpTransactionReceipt =
            serde_json::from_str(tx_receipt_str).expect("failed to parse tx receipt");
        let exec_receipt = ExecutionReceipt::try_from(tx_receipt.clone())
            .expect("failed to convert tx receipt to exec receipt");

        assert_eq!(exec_receipt.inner, tx_receipt.inner);
    }
}
