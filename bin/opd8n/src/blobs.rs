//! Blob Loading Module

use color_eyre::Result;
use tracing::warn;
use alloy::primitives::{Address, TxKind};
use alloy::consensus::{Transaction, TxEip4844Variant, TxEnvelope, TxType};

use kona_derive::traits::BlobProvider;
use kona_derive::types::{Blob, BlockInfo, IndexedBlobHash};
use kona_derive::online::{OnlineBeaconClient, SimpleSlotDerivation, OnlineBlobProvider};

/// Loads blobs for the given block number.
pub async fn load(b: &BlockInfo, txs: &[TxEnvelope], batcher_address: Address, signer: Address, provider: &mut OnlineBlobProvider<OnlineBeaconClient, SimpleSlotDerivation>) -> Result<Vec<Box<Blob>>> {
     let blob_hashes = extract_blob_data(batcher_address, signer, txs);

    // If there are no blob hashes, we can return empty.
    if blob_hashes.is_empty() {
        return Ok(vec![]);
    }

    provider.get_blobs(b, &blob_hashes).await.map_err(|e| {
        warn!(target: "blobs", "Failed to fetch blobs: {e}");
        color_eyre::eyre::eyre!("Failed to fetch blobs: {e}")
    })
}

fn extract_blob_data(batcher_address: Address, signer: Address, txs: &[TxEnvelope]) -> Vec<IndexedBlobHash> {
        let mut index = 0;
        let mut hashes = Vec::new();
        for tx in txs {
            let (tx_kind, calldata, blob_hashes) = match &tx {
                TxEnvelope::Legacy(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip2930(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip1559(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip4844(blob_tx_wrapper) => match blob_tx_wrapper.tx() {
                    TxEip4844Variant::TxEip4844(tx) => {
                        (tx.to(), tx.input.clone(), Some(tx.blob_versioned_hashes.clone()))
                    }
                    TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                        let tx = tx.tx();
                        (tx.to(), tx.input.clone(), Some(tx.blob_versioned_hashes.clone()))
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
                let hash = match &tx {
                    TxEnvelope::Legacy(tx) => Some(tx.hash()),
                    TxEnvelope::Eip2930(tx) => Some(tx.hash()),
                    TxEnvelope::Eip1559(tx) => Some(tx.hash()),
                    TxEnvelope::Eip4844(blob_tx_wrapper) => Some(blob_tx_wrapper.hash()),
                    _ => None,
                };
                warn!(target: "blobs", "Blob tx has calldata, which will be ignored: {hash:?}");
            }
            let blob_hashes = if let Some(b) = blob_hashes {
                b
            } else {
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
