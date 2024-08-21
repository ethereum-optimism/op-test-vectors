//! From L2 Subcommand

use clap::{ArgAction, Parser};
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use hashbrown::HashMap;
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
const TARGET: &str = "from-l2";

/// CLI arguments for the `from-l2` subcommand of `opdn`.
#[derive(Parser, Clone, Debug)]
pub struct FromL2 {
    /// The L2 block number to start from
    #[clap(short, long, help = "Starting L1 block number")]
    pub start_block: u64,
    /// The L2 block number to end at
    #[clap(short, long, help = "Ending L1 block number")]
    pub end_block: u64,
    /// An RPC URL to fetch L1 block data from.
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

impl FromL2 {
    /// Runs the from-l2 subcommand.
    pub async fn run(&self) -> Result<()> {
        ensure!(
            self.end_block > self.start_block,
            "End block before start block"
        );
        trace!(target: TARGET, "Producing derivation fixture for L2 block range [{}, {}]", self.start_block, self.end_block);

        // Build the pipeline
        let cfg = Arc::new(self.rollup_config().await?);
        let mut l1_provider = self.l1_provider()?;
        let mut l2_provider = self.l2_provider(cfg.clone())?;
        let attributes = self.attributes(cfg.clone(), &l2_provider, &l1_provider);
        let mut blob_provider = self.blob_provider();
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

        // Collect the output payload attributes.
        let mut payloads = HashMap::new();

        // Collect reference payloads for span batch validation.
        let mut ref_payloads = HashMap::new();

        // Track the earliest l1 block and last l1 block.
        let first_l1_block = l2_cursor.l1_origin.number;
        let mut last_l1_block = l2_cursor.block_info.number;
        let mut l2_block_infos = HashMap::new();
        let mut configs = HashMap::new();
        let first_system_config = l2_provider
            .system_config_by_number(l2_cursor.block_info.number, Arc::clone(&cfg))
            .await
            .map_err(|e| eyre!(e))?;
        configs.insert(l2_cursor.block_info.number, first_system_config);
        l2_block_infos.insert(l2_cursor.block_info.number, l2_cursor);
        let start_l2_cursor = l2_cursor.block_info.number;

        // TODO: Temporary patch to provide all span batch data to check.
        // 100 blocks before the start block.
        for i in (self.start_block.saturating_sub(500)..self.start_block).rev() {
            let l2_block_info = l2_provider
                .l2_block_info_by_number(i)
                .await
                .map_err(|e| eyre!(e))?;
            let system_config = l2_provider
                .system_config_by_number(i, Arc::clone(&cfg))
                .await
                .map_err(|e| eyre!(e))?;
            configs.insert(i, system_config);
            l2_block_infos.insert(i, l2_block_info);
            // Get reference payloads by l2 block number for span batch validation
            let l2_payload = l2_provider
                .payload_by_number(i)
                .await
                .map_err(|e| eyre!(e))?;
            ref_payloads.insert(i, crate::cmd::util::to_payload_attributes(l2_payload));
        }

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
            payloads.insert(derived as u64, attributes.attributes);
            last_l1_block = origin.number;

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

            // Add the system config
            let system_config = l2_provider
                .system_config_by_number(l2_cursor.block_info.number, Arc::clone(&cfg))
                .await
                .map_err(|e| eyre!(e))?;
            configs.insert(l2_cursor.block_info.number, system_config);
            l2_block_infos.insert(l2_cursor.block_info.number, l2_cursor);
        }

        // Take the full L1 range of blocks and get all needed data.
        let l1_blocks = (first_l1_block..=last_l1_block).collect::<Vec<u64>>();

        // Construct the fixture blocks.
        let blocks = crate::cmd::build_fixture_blocks(
            cfg.batch_inbox_address,
            cfg.genesis
                .system_config
                .as_ref()
                .map(|sc| sc.batcher_address)
                .unwrap_or_default(),
            &l1_blocks,
            &mut l1_provider,
            &mut blob_provider,
        )
        .await?;
        let fixture = DerivationFixture {
            rollup_config: Arc::unwrap_or_clone(cfg),
            l1_blocks: blocks,
            l2_payloads: payloads,
            ref_payloads,
            l2_system_configs: configs,
            l2_block_infos,
            l2_cursor_start: start_l2_cursor,
            l2_cursor_end: self.end_block,
        };
        info!(target: TARGET, "Successfully built derivation test fixture");

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

    /// Returns a new [OnlineBlobProviderWithFallback] using the beacon url.
    pub fn blob_provider(
        &self,
    ) -> OnlineBlobProviderWithFallback<OnlineBeaconClient, OnlineBeaconClient, SimpleSlotDerivation>
    {
        OnlineBlobProviderBuilder::new()
            .with_beacon_client(OnlineBeaconClient::new_http(self.beacon_url.clone()))
            .build()
    }

    /// Returns a new [EthereumDataSource] using the l1 provider and blob provider.
    pub fn dap(
        &self,
        l1_provider: AlloyChainProvider,
        blob_provider: OnlineBlobProviderWithFallback<
            OnlineBeaconClient,
            OnlineBeaconClient,
            SimpleSlotDerivation,
        >,
        cfg: &RollupConfig,
    ) -> EthereumDataSource<
        AlloyChainProvider,
        OnlineBlobProviderWithFallback<
            OnlineBeaconClient,
            OnlineBeaconClient,
            SimpleSlotDerivation,
        >,
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
