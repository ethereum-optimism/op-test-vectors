//! Blob Loading Module

use alloy_consensus::{Transaction, TxEip4844Variant, TxEnvelope, TxType};
use alloy_primitives::{Address, TxKind};
use color_eyre::Result;
use tracing::warn;

use kona_derive::online::{
    OnlineBeaconClient, OnlineBlobProviderWithFallback, SimpleSlotDerivation,
};
use kona_derive::traits::BlobProvider;
use kona_derive::types::{Blob, BlockInfo, IndexedBlobHash};

/// Loads blobs for the given block number.
pub async fn load(
    b: &BlockInfo,
    txs: &[TxEnvelope],
    batcher_address: Address,
    signer: Address,
    provider: &mut OnlineBlobProviderWithFallback<
        OnlineBeaconClient,
        OnlineBeaconClient,
        SimpleSlotDerivation,
    >,
) -> Result<Vec<Box<Blob>>> {
    let blob_hashes = extract_blob_data(batcher_address, signer, txs);

    // If there are no blob hashes, we can return empty.
    if blob_hashes.is_empty() {
        return Ok(vec![]);
    }

    provider
        .get_blobs(b, &blob_hashes)
        .await
        .map_err(|e| {
            warn!(target: "blobs", "Failed to fetch blobs: {e}");
            color_eyre::eyre::eyre!("Failed to fetch blobs: {e}")
        })
        .map(|blobs| {
            blobs
                .into_iter()
                .map(|b| Box::new(b) as Box<Blob>)
                .collect()
        })
}

fn extract_blob_data(
    batcher_address: Address,
    signer: Address,
    txs: &[TxEnvelope],
) -> Vec<IndexedBlobHash> {
    let mut index = 0;
    let mut hashes = Vec::new();
    for tx in txs {
        let (tx_kind, calldata, blob_hashes) = match &tx {
            TxEnvelope::Legacy(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
            TxEnvelope::Eip2930(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
            TxEnvelope::Eip1559(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
            TxEnvelope::Eip4844(blob_tx_wrapper) => match blob_tx_wrapper.tx() {
                TxEip4844Variant::TxEip4844(tx) => (
                    tx.to(),
                    tx.input.clone(),
                    Some(tx.blob_versioned_hashes.clone()),
                ),
                TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                    let tx = tx.tx();
                    (
                        tx.to(),
                        tx.input.clone(),
                        Some(tx.blob_versioned_hashes.clone()),
                    )
                }
            },
            _ => continue,
        };
        let TxKind::Call(to) = tx_kind else { continue };

        if to != batcher_address {
            index += blob_hashes.map_or(0, |h| h.len());
            continue;
        }
        if tx.recover_signer().unwrap_or_default() != signer {
            index += blob_hashes.map_or(0, |h| h.len());
            continue;
        }
        if tx.tx_type() != TxType::Eip4844 {
            continue;
        }
        if !calldata.is_empty() {
            let hash = tx.tx_hash();
            warn!(target: "blobs", "Blob tx has calldata, which will be ignored: {hash:?}");
        }
        let Some(blob_hashes) = blob_hashes else {
            continue;
        };
        for blob in blob_hashes {
            let indexed = IndexedBlobHash { hash: blob, index };
            hashes.push(indexed);
            index += 1;
        }
    }
    hashes
}
