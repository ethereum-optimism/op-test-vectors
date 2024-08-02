//! Module containing the derivation test fixture.

use alloy_consensus::Blob;
use alloy_primitives::{Bytes, B256};
use kona_derive::types::L2PayloadAttributes;
use serde::{Deserialize, Serialize};

/// The derivation fixture is the top-level object that contains
/// everything needed to run a derivation test.
#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
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
#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixtureBlock {
    /// The block number.
    pub number: u64,
    /// The block hash.
    pub hash: B256,
    /// The block timestamp.
    pub timestamp: u64,
    /// Block Transactions.
    /// EIP-2718 encoded raw transactions
    pub transactions: Vec<Bytes>,
    /// Blobs for this block.
    pub blobs: Vec<Box<Blob>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, bytes};

    fn ref_blocks() -> Vec<FixtureBlock> {
        vec![
            FixtureBlock {
                number: 1,
                hash: b256!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                timestamp: 102,
                transactions: vec![
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                ],
                blobs: vec![],
            },
            FixtureBlock {
                number: 2,
                hash: b256!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"),
                timestamp: 104,
                transactions: vec![
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                ],
                blobs: vec![],
            },
            FixtureBlock {
                number: 2,
                hash: b256!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"),
                timestamp: 106,
                transactions: vec![
                    bytes!("02f870018307c100808476d0a39c82565f94388c818ca8b9251b393131c08a736a67ccb1929787b60572b2eb6c9080c001a033bee682348fa78ffc1027bc9981e7dc60eca03af909c4eb05720e781fdae179a01ccf85367c246082fa09ef748d3b07c90752c2b59034a6b881cf99aca586eaf5"),
                ],
                blobs: vec![],
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
        assert_eq!(fixture.number, 1);
        assert_eq!(
            fixture.hash,
            b256!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
        assert_eq!(fixture.timestamp, 102);
        assert_eq!(fixture.transactions.len(), 1);
        assert_eq!(fixture.blobs.len(), 0);
    }
}
