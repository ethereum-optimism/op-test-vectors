//! Module containing the derivation test fixture.

use alloy_consensus::{Blob, Header, Receipt};
use alloy_primitives::Bytes;
use kona_derive::types::{L2PayloadAttributes, SystemConfig};
use serde::{Deserialize, Serialize};

/// The derivation fixture is the top-level object that contains
/// everything needed to run a derivation test.
#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DerivationFixture {
    /// A list of L1 Blocks to derive from.
    pub l1_blocks: Vec<FixtureBlock>,
    /// A list of L2 payload attributes to assert against.
    pub l2_payloads: Vec<L2PayloadAttributes>,
}

impl DerivationFixture {
    /// Constructs a new [DerivationFixture] with the given L1 blocks and L2 Payload Attributes.
    pub fn new(l1_blocks: Vec<FixtureBlock>, l2_payloads: Vec<L2PayloadAttributes>) -> Self {
        Self {
            l1_blocks,
            l2_payloads,
        }
    }
}

/// A fixture block is a minimal block with associated data including blobs
/// to derive from.
#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixtureBlock {
    /// The block header.
    /// The entire header is required to generate the block hash when deriving the l1 block info
    /// tx.
    pub header: Header,
    /// Block Transactions.
    /// EIP-2718 encoded raw transactions
    pub transactions: Vec<Bytes>,
    /// Blobs for this block.
    pub blobs: Vec<Box<Blob>>,
    /// Receipts for this block.
    pub receipts: Vec<Receipt>,
    /// The [SystemConfig] at this block height.
    pub system_config: SystemConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, bytes, uint};

    fn ref_blocks() -> Vec<FixtureBlock> {
        vec![
            FixtureBlock {
                header: Header {
                    number: 1,
                    parent_hash: b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                    ommers_hash: b256!("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347"),
                    beneficiary: address!("0000000000000000000000000000000000000000"),
                    state_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    transactions_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    receipts_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    logs_bloom: Default::default(),
                    difficulty: uint!(0_U256),
                    gas_limit: 0,
                    gas_used: 0,
                    mix_hash: b256!("0000000000000000000000000000000000000000000000000000000000000000"),
                    nonce: Default::default(),
                    extra_data: bytes!(""),
                    timestamp: 102,
                    ..Default::default()
                },
                transactions: vec![
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                ],
                blobs: vec![],
                receipts: vec![
                    Receipt {
                        status: alloy_consensus::Eip658Value::Eip658(true),
                        cumulative_gas_used: 10,
                        logs: vec![
                            alloy_primitives::Log {
                                address: address!("4200000000000000000000000000000000000011"),
                                data: alloy_primitives::LogData::new_unchecked(vec![], bytes!("")),
                            }
                        ],
                    },
                ],
                system_config: SystemConfig {
                    batcher_address: address!("3333333333333333333333333333333333333333"),
                    overhead: uint!(8_U256),
                    scalar: uint!(7_U256),
                    gas_limit: 0,
                    base_fee_scalar: Some(0),
                    blob_base_fee_scalar: Some(0),
                },
            },
            FixtureBlock {
                header: Header {
                    number: 2,
                    parent_hash: b256!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                    ommers_hash: b256!("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347"),
                    beneficiary: address!("0000000000000000000000000000000000000000"),
                    state_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    transactions_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    receipts_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    logs_bloom: Default::default(),
                    difficulty: uint!(0_U256),
                    gas_limit: 0,
                    gas_used: 0,
                    mix_hash: b256!("0000000000000000000000000000000000000000000000000000000000000000"),
                    nonce: Default::default(),
                    extra_data: bytes!(""),
                    timestamp: 104,
                    ..Default::default()
                },
                transactions: vec![
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                ],
                blobs: vec![],
                receipts: vec![
                    Receipt {
                        status: alloy_consensus::Eip658Value::Eip658(true),
                        cumulative_gas_used: 10,
                        logs: vec![
                            alloy_primitives::Log {
                                address: address!("4200000000000000000000000000000000000011"),
                                data: alloy_primitives::LogData::new_unchecked(vec![], bytes!("")),
                            }
                        ],
                    },
                ],
                system_config: SystemConfig {
                    batcher_address: address!("3333333333333333333333333333333333333333"),
                    overhead: uint!(8_U256),
                    scalar: uint!(7_U256),
                    gas_limit: 0,
                    base_fee_scalar: Some(0),
                    blob_base_fee_scalar: Some(0),
                },
            },
            FixtureBlock {
                header: Header {
                    number: 2,
                    parent_hash: b256!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                    ommers_hash: b256!("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347"),
                    beneficiary: address!("0000000000000000000000000000000000000000"),
                    state_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    transactions_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    receipts_root: b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421"),
                    logs_bloom: Default::default(),
                    difficulty: uint!(0_U256),
                    gas_limit: 0,
                    gas_used: 0,
                    mix_hash: b256!("0000000000000000000000000000000000000000000000000000000000000000"),
                    nonce: Default::default(),
                    extra_data: bytes!(""),
                    timestamp: 106,
                    ..Default::default()
                },
                transactions: vec![
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                ],
                blobs: vec![],
                receipts: vec![
                    Receipt {
                        status: alloy_consensus::Eip658Value::Eip658(true),
                        cumulative_gas_used: 10,
                        logs: vec![
                            alloy_primitives::Log {
                                address: address!("4200000000000000000000000000000000000011"),
                                data: alloy_primitives::LogData::new_unchecked(vec![], bytes!("")),
                            }
                        ],
                    },
                ],
                system_config: SystemConfig {
                    batcher_address: address!("3333333333333333333333333333333333333333"),
                    overhead: uint!(8_U256),
                    scalar: uint!(7_U256),
                    gas_limit: 0,
                    base_fee_scalar: Some(0),
                    blob_base_fee_scalar: Some(0),
                },
            },
        ]
    }

    fn ref_payload_attributes() -> Vec<L2PayloadAttributes> {
        vec![
            L2PayloadAttributes {
                timestamp: 1722550777,
                fee_recipient: address!("4200000000000000000000000000000000000011"),
                prev_randao: b256!(
                    "73ce62c38a0714e87a4141f33ec2362dc800d7693d85e42ffe6bdc22a5c84610"
                ),
                parent_beacon_block_root: Some(b256!(
                    "8693a4b644bc68b8562194814d2945e4a78e2b20967c0a5c2f5f8e741be5a379"
                )),
                gas_limit: Some(30000000),
                no_tx_pool: true,
                withdrawals: Some(vec![]),
                ..Default::default()
            },
            L2PayloadAttributes {
                timestamp: 1722550779,
                fee_recipient: address!("4200000000000000000000000000000000000011"),
                prev_randao: b256!(
                    "73ce62c38a0714e87a4141f33ec2362dc800d7693d85e42ffe6bdc22a5c84610"
                ),
                parent_beacon_block_root: Some(b256!(
                    "8693a4b644bc68b8562194814d2945e4a78e2b20967c0a5c2f5f8e741be5a379"
                )),
                gas_limit: Some(30000000),
                withdrawals: Some(vec![]),
                no_tx_pool: true,
                ..Default::default()
            },
        ]
    }

    #[test]
    fn test_derivation_fixture() {
        let fixture_str = include_str!("./testdata/derivation_fixture.json");
        let fixture: DerivationFixture = serde_json::from_str(fixture_str).unwrap();
        let expected = DerivationFixture {
            l1_blocks: ref_blocks(),
            l2_payloads: ref_payload_attributes(),
        };
        assert_eq!(fixture, expected);
    }

    #[test]
    fn test_fixture_block() {
        let fixture_str = include_str!("./testdata/fixture_block.json");
        let fixture: FixtureBlock = serde_json::from_str(fixture_str).unwrap();
        assert_eq!(fixture.header.number, 1);
        assert_eq!(
            fixture.header.parent_hash,
            b256!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(fixture.header.timestamp, 102);
        assert_eq!(fixture.transactions.len(), 1);
        assert_eq!(fixture.blobs.len(), 0);
    }
}
