//! Contains logic to generate derivation test fixtures using L1 source block information.

use clap::{ArgAction, Parser};
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use kona_derive::online::{
    AlloyChainProvider, OnlineBeaconClient, OnlineBlobProvider, SimpleSlotDerivation,
};
use kona_derive::{
    online::*,
    types::{L2BlockInfo, StageError},
};
use op_test_vectors::derivation::DerivationFixture;
use reqwest::Url;
use std::path::PathBuf;
use std::sync::Arc;
use superchain_registry::ROLLUP_CONFIGS;
use tracing::{debug, error, info, trace, warn};

/// The logging target to use for [tracing].
const TARGET: &str = "from-l1";

/// CLI arguments for the `from-l1` subcommand of `opd8n`.
#[derive(Parser, Clone, Debug)]
pub struct FromL1 {
    /// The L1 block number to start from
    #[clap(short, long, help = "Starting L1 block number")]
    pub start_block: u64,
    /// The L1 block number to end at
    #[clap(short, long, help = "Ending L1 block number")]
    pub end_block: u64,
    /// An L1 RPC URL to fetch L1 block data from.
    #[clap(long, help = "RPC url to fetch L1 block data from")]
    pub l1_rpc_url: String,
    /// An L2 RPC URL to validate span batches.
    #[clap(long, help = "L2 RPC URL to validate span batches")]
    pub l2_rpc_url: String,
    /// A beacon client to fetch blob data from.
    #[clap(long, help = "Beacon client url to fetch blob data from")]
    pub beacon_url: String,
    /// The output file for the test fixture.
    #[clap(long, help = "Output file for the test fixture")]
    pub output: PathBuf,
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
}

impl FromL1 {
    /// Runs the derivation test fixture generation using the L1 source block information.
    /// This function effectively takes the L1 block info and fetches any calldata or blob
    /// data associated with this block.
    pub async fn run(&self) -> Result<()> {
        ensure!(
            self.end_block > self.start_block,
            "End block must come after the start block"
        );
        trace!(target: "from-l1", "Producing derivation fixture for L1 block range [{}, {}]", self.start_block, self.end_block);

        // Construct a sequential list of block numbers from [start_block, end_block].
        let blocks = (self.start_block..=self.end_block).collect::<Vec<_>>();

        // Construct the providers
        let l1_rpc_url =
            Url::parse(&self.l1_rpc_url).map_err(|e| eyre!("Invalid L1 RPC URL: {}", e))?;
        let mut l1_provider = AlloyChainProvider::new_http(l1_rpc_url);
        let l2_rpc_url =
            Url::parse(&self.l2_rpc_url).map_err(|e| eyre!("Invalid L1 RPC URL: {}", e))?;
        let mut l2_provider =
            AlloyL2ChainProvider::new_http(l2_rpc_url, Arc::new(Default::default()));
        let l2_chain_id = l2_provider.chain_id().await.map_err(|e| eyre!(e))?;
        info!(target: "from-l1", "Using L1 Chain ID: {}", l2_chain_id);
        let config = ROLLUP_CONFIGS
            .get(&l2_chain_id)
            .ok_or_else(|| eyre!("No rollup config found for chain id: {}", l2_chain_id))?;
        let batcher_address = config.batch_inbox_address;
        info!(target: "from-l1", "Using batcher address: {}", batcher_address);
        let signer = config
            .genesis
            .system_config
            .as_ref()
            .map(|sc| sc.batcher_address)
            .unwrap_or_default();
        info!(target: "from-l1", "Using signer address: {}", signer);

        let beacon_client = OnlineBeaconClient::new_http(self.beacon_url.clone());
        let mut blob_provider =
            OnlineBlobProvider::<_, SimpleSlotDerivation>::new(beacon_client, None, None);

        // Construct the derivation fixture.
        let fixture_blocks = crate::cmd::build_fixture_blocks(
            batcher_address,
            signer,
            &blocks,
            &mut l1_provider,
            &mut blob_provider,
        )
        .await?;

        // Build the pipeline
        let cfg = Arc::new(self.rollup_config().await?);
        let mut l1_provider = self.l1_provider()?;
        let mut l2_provider = self.l2_provider(cfg.clone())?;
        let attributes = self.attributes(cfg.clone(), &l2_provider, &l1_provider);
        let blob_provider = self.blob_provider();
        let dap = self.dap(l1_provider.clone(), blob_provider.clone(), &cfg);
        let mut l2_cursor = self.cursor().await?;
        let l1_tip = l1_provider
            .block_info_by_number(l2_cursor.l1_origin.number)
            .await
            .expect("Failed to fetch genesis L1 block info for pipeline tip");
        let mut pipeline = new_online_pipeline(
            cfg.clone(),
            l1_provider.clone(),
            dap,
            l2_provider.clone(),
            attributes,
            l1_tip,
        );

        let mut payloads = Vec::with_capacity((self.end_block - self.start_block) as usize);

        // Run the pipeline
        loop {
            // If the cursor is beyond the end block, break the loop.
            if l2_cursor.block_info.number >= self.end_block {
                trace!(target: TARGET, "Cursor is beyond the end block, breaking loop");
                break;
            }

            // Step on the pipeline.
            match pipeline.step(l2_cursor).await {
                StepResult::PreparedAttributes => trace!(target: "loop", "Prepared attributes"),
                StepResult::AdvancedOrigin => trace!(target: "loop", "Advanced origin"),
                StepResult::OriginAdvanceErr(e) => {
                    warn!(target: TARGET, "Could not advance origin: {:?}", e)
                }
                StepResult::StepFailed(e) => match e {
                    StageError::NotEnoughData => {
                        debug!(target: TARGET, "Not enough data to step derivation pipeline");
                    }
                    _ => {
                        error!(target: TARGET, "Error stepping derivation pipeline: {:?}", e);
                    }
                },
            }

            // Get the attributes if there are some available.
            let Some(attributes) = pipeline.next() else {
                continue;
            };

            // Print the L1 range for this L2 Block.
            let derived = attributes.parent.block_info.number as i64 + 1;
            let l2_block_info = l2_provider
                .l2_block_info_by_number(derived as u64)
                .await
                .map_err(|e| eyre!(e))?;
            let origin = pipeline
                .origin()
                .ok_or(eyre!("Failed to get pipeline l1 origin"))?;
            info!(target: TARGET,
                "L2 Block [{}] L1 Range: [{}, {}]",
                derived, l2_block_info.l1_origin.number, origin.number
            );
            payloads.push(attributes.attributes);

            // Keep trying to advance the cursor in case the fetch fails.
            loop {
                match l2_provider
                    .l2_block_info_by_number(l2_cursor.block_info.number + 1)
                    .await
                {
                    Ok(bi) => {
                        l2_cursor = bi;
                        break;
                    }
                    Err(e) => {
                        error!(target: TARGET, "Failed to fetch next pending l2 safe head: {}, err: {:?}", l2_cursor.block_info.number + 1, e);
                        // Don't step on the pipeline if we failed to fetch the next l2 safe head.
                        continue;
                    }
                }
            }
        }

        let fixture = DerivationFixture::new(fixture_blocks, payloads);
        info!(target: "from-l1", "Successfully built derivation test fixture");

        // Write the derivation fixture to the specified output location.
        let file = std::fs::File::create(&self.output)?;
        serde_json::to_writer_pretty(file, &fixture)?;
        info!(target: "from-l1", "Wrote derivation fixture to: {:?}", self.output);

        Ok(())
    }

    /// Gets the L2 starting block number.
    /// Returns the genesis L2 block number if the start block is less than the genesis block number.
    pub fn start_block(&self, cfg: &RollupConfig) -> u64 {
        if self.start_block < cfg.genesis.l2.number {
            cfg.genesis.l2.number
        } else if self.start_block != 0 {
            self.start_block - 1
        } else {
            self.start_block
        }
    }

    /// Returns an [L2BlockInfo] cursor for the pipeline.
    pub async fn cursor(&self) -> Result<L2BlockInfo> {
        let cfg = self.rollup_config().await?;
        let start_block = self.start_block(&cfg);
        let mut l2_provider = self.l2_provider(Arc::new(cfg))?;
        let cursor = l2_provider
            .l2_block_info_by_number(start_block)
            .await
            .map_err(|_| eyre!("Failed to fetch genesis L2 block info for pipeline cursor"))?;
        Ok(cursor)
    }

    /// Returns a new [AlloyChainProvider] using the l1 rpc url.
    pub fn l1_provider(&self) -> Result<AlloyChainProvider> {
        Ok(AlloyChainProvider::new_http(self.l1_rpc_url()?))
    }

    /// Returns a new [AlloyL2ChainProvider] using the l2 rpc url.
    pub fn l2_provider(&self, cfg: Arc<RollupConfig>) -> Result<AlloyL2ChainProvider> {
        Ok(AlloyL2ChainProvider::new_http(self.l2_rpc_url()?, cfg))
    }

    /// Returns a new [StatefulAttributesBuilder] using the l1 and l2 providers.
    pub fn attributes(
        &self,
        cfg: Arc<RollupConfig>,
        l2_provider: &AlloyL2ChainProvider,
        l1_provider: &AlloyChainProvider,
    ) -> StatefulAttributesBuilder<AlloyChainProvider, AlloyL2ChainProvider> {
        StatefulAttributesBuilder::new(cfg, l2_provider.clone(), l1_provider.clone())
    }

    /// Returns a new [OnlineBlobProvider] using the beacon url.
    pub fn blob_provider(&self) -> OnlineBlobProvider<OnlineBeaconClient, SimpleSlotDerivation> {
        OnlineBlobProvider::new(
            OnlineBeaconClient::new_http(self.beacon_url.clone()),
            None,
            None,
        )
    }

    /// Returns a new [EthereumDataSource] using the l1 provider and blob provider.
    pub fn dap(
        &self,
        l1_provider: AlloyChainProvider,
        blob_provider: OnlineBlobProvider<OnlineBeaconClient, SimpleSlotDerivation>,
        cfg: &RollupConfig,
    ) -> EthereumDataSource<
        AlloyChainProvider,
        OnlineBlobProvider<OnlineBeaconClient, SimpleSlotDerivation>,
    > {
        EthereumDataSource::new(l1_provider, blob_provider, cfg)
    }

    /// Gets the rollup config from the l2 rpc url.
    pub async fn rollup_config(&self) -> Result<RollupConfig> {
        let mut l2_provider =
            AlloyL2ChainProvider::new_http(self.l2_rpc_url()?, Arc::new(Default::default()));
        let l2_chain_id = l2_provider.chain_id().await.map_err(|e| eyre!(e))?;
        let cfg = ROLLUP_CONFIGS
            .get(&l2_chain_id)
            .cloned()
            .ok_or_else(|| eyre!("No rollup config found for L2 chain ID: {}", l2_chain_id))?;
        Ok(cfg)
    }

    /// Returns the l1 rpc url from CLI or environment variable.
    pub fn l1_rpc_url(&self) -> Result<Url> {
        Url::parse(&self.l1_rpc_url).map_err(|e| eyre!(e))
    }

    /// Returns the l2 rpc url from CLI or environment variable.
    pub fn l2_rpc_url(&self) -> Result<Url> {
        Url::parse(&self.l2_rpc_url).map_err(|e| eyre!(e))
    }

    /// Returns the beacon url from CLI or environment variable.
    pub fn beacon_url(&self) -> String {
        self.beacon_url.clone()
    }
}
