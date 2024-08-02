//! CLI for the range-finder.

use clap::{ArgAction, Parser};
use color_eyre::eyre::{eyre, Result};
use kona_derive::{
    online::*,
    types::{L2BlockInfo, StageError},
};
use reqwest::Url;
use std::sync::Arc;
use superchain_registry::ROLLUP_CONFIGS;
use tracing::Level;
use tracing::{debug, error, trace, warn};

const LOG_TARGET: &str = "range_finder";

/// Range Finder Cli
///
/// The CLI struct needs a few RPC URLs that it uses as inputs
/// to the derivation pipeline to derive the range of L2 blocks.
#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Verbosity level (0-4)
    #[arg(long, short, help = "Verbosity level (0-4)", action = ArgAction::Count)]
    pub v: u8,
    /// Starting L2 Block
    #[arg(long, short, help = "Starting L2 Block")]
    pub start_block: u64,
    /// Ending L2 Block
    #[arg(long, short, help = "Ending L2 Block")]
    pub end_block: u64,
    /// The L1 PRC url  for fetching L1 block info.
    #[arg(long, short, help = "The L1 PRC url for fetching L1 block info.")]
    pub l1_rpc_url: String,
    /// An L2 RPC URL to fetch L2 block data from.
    #[clap(long, help = "RPC url to fetch L2 block data from")]
    pub l2_rpc_url: String,
    /// A beacon url for fetching blob information.
    #[arg(long, short, help = "A beacon url for fetching blob information.")]
    pub beacon_url: String,
}

impl Cli {
    /// Initializes telemtry for the application.
    pub fn init_telemetry(self) -> Result<Self> {
        color_eyre::install()?;
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(match self.v {
                0 => Level::ERROR,
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                _ => Level::TRACE,
            })
            .finish();
        tracing::subscriber::set_global_default(subscriber).map_err(|e| eyre!(e))?;
        Ok(self)
    }

    /// Parse the CLI arguments and run the command
    pub async fn run(&self) -> Result<()> {
        // Build the pipeline
        let cfg = Arc::new(self.rollup_config().await?);
        let mut l1_provider = self.l1_provider()?;
        let mut l2_provider = self.l2_provider(cfg.clone())?;
        let attributes = self.attributes(cfg.clone(), &l2_provider, &l1_provider);
        let blob_provider = self.blob_provider();
        let dap = self.dap(l1_provider.clone(), blob_provider, &cfg);
        let mut l2_cursor = self.cursor().await?;
        let l1_tip = l1_provider
            .block_info_by_number(l2_cursor.l1_origin.number)
            .await
            .expect("Failed to fetch genesis L1 block info for pipeline tip");
        let mut pipeline = new_online_pipeline(
            cfg,
            l1_provider,
            dap,
            l2_provider.clone(),
            attributes,
            l1_tip,
        );

        // Run the pipeline
        loop {
            // If the cursor is beyond the end block, break the loop.
            if l2_cursor.block_info.number >= self.end_block {
                trace!(target: "loop", "Cursor is beyond the end block, breaking loop");
                break;
            }

            // Step on the pipeline.
            match pipeline.step(l2_cursor).await {
                StepResult::PreparedAttributes => trace!(target: "loop", "Prepared attributes"),
                StepResult::AdvancedOrigin => trace!(target: "loop", "Advanced origin"),
                StepResult::OriginAdvanceErr(e) => {
                    warn!(target: "loop", "Could not advance origin: {:?}", e)
                }
                StepResult::StepFailed(e) => match e {
                    StageError::NotEnoughData => {
                        debug!(target: "loop", "Not enough data to step derivation pipeline");
                    }
                    _ => {
                        error!(target: "loop", "Error stepping derivation pipeline: {:?}", e);
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
            println!(
                "L2 Block [{}] L1 Range: [{}, {}]",
                derived, l2_block_info.l1_origin.number, origin.number
            );

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
                        error!(target: LOG_TARGET, "Failed to fetch next pending l2 safe head: {}, err: {:?}", l2_cursor.block_info.number + 1, e);
                        // Don't step on the pipeline if we failed to fetch the next l2 safe head.
                        continue;
                    }
                }
            }
        }

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
