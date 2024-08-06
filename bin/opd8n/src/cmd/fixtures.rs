//! Logic for building the derivation fixture blocks.

use crate::cmd::blobs;
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::Address;
use color_eyre::eyre::{eyre, Result};
use kona_derive::online::{
    AlloyChainProvider, OnlineBeaconClient, OnlineBlobProviderWithFallback, SimpleSlotDerivation,
};
use kona_derive::types::SystemConfig;
use kona_derive::traits::ChainProvider;
use op_test_vectors::derivation::FixtureBlock;

/// Constructs [FixtureBlock]s for the given L1 blocks.
pub async fn build_fixture_blocks(
    batcher_address: Address,
    signer: Address,
    blocks: &[u64],
    configs: &[SystemConfig],
    l1_provider: &mut AlloyChainProvider,
    blob_provider: &mut OnlineBlobProviderWithFallback<
        OnlineBeaconClient,
        OnlineBeaconClient,
        SimpleSlotDerivation,
    >,
) -> Result<Vec<FixtureBlock>> {
    let mut fixtures = Vec::with_capacity(blocks.len());
    for (i, b) in blocks.iter().enumerate() {
        let block_info = l1_provider
            .block_info_by_number(*b)
            .await
            .map_err(|e| eyre!(e))?;
        let block_header = l1_provider
            .header_by_hash(block_info.hash)
            .await
            .map_err(|e| eyre!(e))?;
        let (_, txs) = l1_provider
            .block_info_and_transactions_by_hash(block_info.hash)
            .await
            .map_err(|e| eyre!(e))?;
        let mut transactions = Vec::with_capacity(txs.len());
        for tx in txs.as_slice() {
            let mut out = Vec::new();
            tx.encode_2718(&mut out);
            transactions.push(out.into());
        }
        let receipts = l1_provider
            .receipts_by_hash(block_info.hash)
            .await
            .map_err(|e| eyre!(e))?;

        let blobs = blobs::load(
            &block_info,
            txs.as_slice(),
            batcher_address,
            signer,
            blob_provider,
        )
        .await?;

        let fixture = FixtureBlock {
            header: block_header,
            transactions,
            blobs,
            receipts,
            system_config: configs.get(i).cloned().ok_or_else(|| eyre!("Missing config"))?,
        };
        fixtures.push(fixture);
    }
    Ok(fixtures)
}
