use alloy::{
    eips::BlockId,
    rpc::types::{
        anvil::Forking,
        trace::geth::{PreStateConfig, PreStateFrame},
    },
};

use anvil::{cmd::NodeArgs, eth::EthApi, NodeConfig, NodeHandle};
use anvil_core::eth::block::Block;
use anvil_core::eth::transaction::PendingTransaction;
use cast::traces::{GethTraceBuilder, TracingInspectorConfig};
use forge_script::ScriptArgs;
use inquire::InquireError;
use std::{
    fs::{self, File},
    path::PathBuf,
};

use clap::{CommandFactory, FromArgMatches, Parser};
use color_eyre::eyre::{ensure, eyre, Result};
use futures::StreamExt;
use op_test_vectors::execution::{ExecutionFixture, ExecutionReceipt, ExecutionResult};
use revm::{
    db::{AlloyDB, CacheDB},
    primitives::{BlobExcessGasAndPrice, BlockEnv, U256},
    DatabaseCommit, EvmBuilder,
};
use serde::{Deserialize, Serialize};

pub struct Opt8n {
    pub eth_api: EthApi,
    pub node_handle: NodeHandle,
    pub execution_fixture: ExecutionFixture,
    pub fork: Option<Forking>,
    pub node_config: NodeConfig,
    pub output_file: PathBuf,
}

impl Opt8n {
    pub async fn new(
        node_config: Option<NodeConfig>,
        fork: Option<Forking>,
        output_file: PathBuf,
        genesis: Option<PathBuf>,
    ) -> Result<Self> {
        let genesis = if let Some(genesis) = genesis.as_ref() {
            serde_json::from_reader(File::open(genesis)?)?
        } else {
            None
        };

        let node_config = node_config
            .unwrap_or_default()
            .with_optimism(true)
            .with_no_mining(true)
            .with_genesis(genesis);

        let (eth_api, node_handle) = anvil::spawn(node_config.clone()).await;
        eth_api.anvil_set_logging(false).await?;

        Ok(Self {
            eth_api,
            node_handle,
            execution_fixture: ExecutionFixture::default(),
            fork,
            node_config,
            output_file,
        })
    }

    /// Listens for commands, and new blocks from the block stream.
    pub async fn repl(&mut self) -> Result<()> {
        let mut new_blocks = self.eth_api.backend.new_block_notifications();

        loop {
            tokio::select! {
                command = self.receive_command() => {
                    match command {
                        Ok(ReplCommand::Exit) => break,
                        Ok(command) => {
                            if let Err(r) = self.execute(command).await {
                                eprintln!("{:?}", r);
                            }
                        },
                        Err(e) => {
                            eprintln!("{:?}", e);
                            // This happens with ctrl-c
                            if e.downcast_ref::<InquireError>().is_some() {
                                break
                            }
                        },
                    }
                }

                new_block = new_blocks.next() => {
                    if let Some(new_block) = new_block {
                        if let Some(block) = self.eth_api.backend.get_block_by_hash(new_block.hash) {
                            self.generate_execution_fixture(block).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Run a Forge script with the given arguments, and generate an execution fixture
    /// from the broadcasted transactions.
    pub async fn run_script(self, script_args: Box<ScriptArgs>) -> Result<()> {
        let mut new_blocks = self.eth_api.backend.new_block_notifications();

        // Run the forge script and broadcast the transactions to the anvil node
        let mut opt8n = self.broadcast_transactions(script_args).await?;

        // Mine the block and generate the execution fixture
        opt8n.mine_block().await;

        let block = new_blocks.next().await.ok_or(eyre!("No new block"))?;
        if let Some(block) = opt8n.eth_api.backend.get_block_by_hash(block.hash) {
            opt8n.generate_execution_fixture(block).await?;
        }

        Ok(())
    }

    async fn broadcast_transactions(self, script_args: Box<ScriptArgs>) -> Result<Self> {
        // Run the script, compile the transactions and broadcast to the anvil instance
        let compiled = script_args.preprocess().await?.compile()?;

        let pre_simulation = compiled
            .link()
            .await?
            .prepare_execution()
            .await?
            .execute()
            .await?
            .prepare_simulation()
            .await?;

        let bundled = pre_simulation.fill_metadata().await?.bundle().await?;

        let tx_count = bundled
            .sequence
            .sequences()
            .iter()
            .fold(0, |sum, sequence| sum + sequence.transactions.len());

        // TODO: break into function
        let broadcast = bundled.broadcast();

        let opt8n = self;

        let pending_transactions = tokio::task::spawn(async move {
            loop {
                let pending_tx_count = opt8n
                    .eth_api
                    .txpool_content()
                    .await
                    .expect("Failed to get txpool content")
                    .pending
                    .len();

                if pending_tx_count == tx_count {
                    return opt8n;
                }
            }
        });

        let opt8n = tokio::select! {
            _ = broadcast => {
                // TODO: Gracefully handle this error
                return Err(eyre!("Script failed early"));
            },
            opt8n = pending_transactions => {
                opt8n?
            }
        };

        Ok(opt8n)
    }

    async fn receive_command(&self) -> Result<ReplCommand> {
        let line = tokio::task::spawn_blocking(|| {
            inquire::Text::new("Enter a command")
                .with_help_message("'help' for more information")
                .prompt()
        })
        .await??;
        let words = shellwords::split(&line)?;

        let matches = ReplCommand::command().try_get_matches_from(words)?;
        Ok(ReplCommand::from_arg_matches(&matches)?)
    }

    async fn execute(&mut self, command: ReplCommand) -> Result<()> {
        match command {
            ReplCommand::Dump => {
                self.mine_block().await;
            }
            ReplCommand::Anvil { mut args } => {
                args.insert(0, "anvil".to_string());
                let command = NodeArgs::command_for_update();
                let matches = command.try_get_matches_from(args)?;
                let node_args = NodeArgs::from_arg_matches(&matches)?;
                node_args.run().await?;
            }
            ReplCommand::Cast { .. } => {}
            ReplCommand::RpcEndpoint => {
                println!("{}", self.node_handle.http_endpoint());
            }
            ReplCommand::Exit => unreachable!(),
        }
        Ok(())
    }

    /// Updates the pre and post state allocations of the [ExecutionFixture] from Revm.
    pub fn capture_pre_post_alloc(&mut self, block: &Block) -> Result<()> {
        let revm_db = CacheDB::new(
            AlloyDB::new(
                self.node_handle.http_provider(),
                BlockId::from(block.header.number - 1),
            )
            .ok_or_else(|| eyre!("Failed to create AlloyDB"))?,
        );

        let block_env = BlockEnv {
            number: U256::from(block.header.number),
            coinbase: block.header.beneficiary,
            timestamp: U256::from(block.header.timestamp),
            difficulty: block.header.difficulty,
            gas_limit: U256::from(block.header.gas_limit),
            prevrandao: Some(block.header.mix_hash),
            basefee: U256::from(block.header.base_fee_per_gas.unwrap_or_default()),
            blob_excess_gas_and_price: block
                .header
                .excess_blob_gas
                .map(|excess_gas| BlobExcessGasAndPrice::new(excess_gas as u64)),
        };

        let mut evm = EvmBuilder::default()
            .with_db(Box::new(revm_db))
            .with_block_env(block_env)
            .build();

        evm.context.evm.env.cfg.chain_id = self.eth_api.chain_id();
        for tx in block.transactions.iter() {
            let pending = PendingTransaction::new(tx.clone().into())?;
            evm.context.evm.env.tx = pending.to_revm_tx_env();
            let result = evm.transact()?;
            let db = &mut evm.context.evm.db;
            let pre_state_frame = GethTraceBuilder::new(vec![], TracingInspectorConfig::default())
                .geth_prestate_traces(
                    &result,
                    PreStateConfig {
                        diff_mode: Some(true),
                    },
                    &db,
                )?;
            db.commit(result.state);

            if let PreStateFrame::Diff(diff) = pre_state_frame {
                diff.pre.into_iter().for_each(|(account, state)| {
                    self.execution_fixture.alloc.entry(account).or_insert(state);
                });
                diff.post.into_iter().for_each(|(account, state)| {
                    self.execution_fixture.out_alloc.insert(account, state);
                });
            }
        }
        Ok(())
    }

    pub async fn mine_block(&mut self) {
        self.eth_api.mine_one().await;
    }

    /// Generates an execution fixture from a block.
    pub async fn generate_execution_fixture(&mut self, block: Block) -> Result<()> {
        self.capture_pre_post_alloc(&block)?;

        // Append block transactions and receipts to the execution fixture
        let mut receipts: Vec<ExecutionReceipt> = Vec::with_capacity(block.transactions.len());
        for tx in block.transactions.iter() {
            if let Some(receipt) = self
                .eth_api
                .backend
                .transaction_receipt(tx.transaction.hash())
                .await?
            {
                receipts.push(receipt.try_into()?);
            }
            self.execution_fixture
                .transactions
                .push(tx.transaction.clone());
        }

        let block_header = &block.header;
        let execution_result = ExecutionResult {
            state_root: block_header.state_root,
            tx_root: block_header.transactions_root,
            receipt_root: block_header.receipts_root,
            logs_bloom: block_header.logs_bloom,
            receipts,
        };

        self.execution_fixture.env = block.into();
        self.execution_fixture.result = execution_result;

        // Ensure pre and post states are different
        ensure!(
            self.execution_fixture.alloc != self.execution_fixture.out_alloc,
            "Pre and post state are the same"
        );

        // Output the execution fixture to file
        let file = fs::File::create(&self.output_file)?;
        serde_json::to_writer_pretty(file, &self.execution_fixture)?;

        Ok(())
    }
}

#[derive(Parser, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[clap(rename_all = "snake_case", infer_subcommands = true, multicall = true)]
pub enum ReplCommand {
    #[command(visible_alias = "a")]
    Anvil {
        #[arg(index = 1, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(visible_alias = "c")]
    Cast {
        #[arg(index = 1, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Dump,
    RpcEndpoint,
    // TODO: implement clear
    // TODO: implement reset
    #[command(visible_alias = "e")]
    Exit,
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn test_update_alloc() {
        // TODO:
    }

    #[tokio::test]
    pub async fn test_dump_execution_fixture() {
        // TODO:
    }
}
