use crate::handler::{Context, HandleInstance};
use crate::indexer::TemplateManager;
use crate::latest_block_manager::LatestBlockManager;
use alloy::primitives::Address;
use alloy::providers::{Provider, RootProvider};
use alloy::rpc::types::eth::Filter;
use alloy::transports::http::{Client, Http};
use alloy::transports::TransportError;
use ghost_crab_common::config::ExecutionMode;
use std::time::Duration;

#[derive(Clone)]
pub struct ProcessEventsInput {
    pub start_block: u64,
    pub address: Address,
    pub step: u64,
    pub handler: HandleInstance,
    pub templates: TemplateManager,
    pub provider: RootProvider<Http<Client>>,
}

pub async fn process_logs(
    ProcessEventsInput { start_block, step, address, handler, templates, provider }: ProcessEventsInput,
) -> Result<(), TransportError> {
    let execution_mode = handler.execution_mode();
    let event_signature = handler.event_signature();

    let mut current_block = start_block;
    let mut latest_block_manager =
        LatestBlockManager::new(provider.clone(), Duration::from_secs(10));

    loop {
        let mut end_block = current_block + step;
        let latest_block = latest_block_manager.get().await?;

        if end_block > latest_block {
            end_block = latest_block;
        }

        if current_block >= end_block {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            continue;
        }

        let source = handler.get_source();

        println!("[{}] Processing logs from {} to {}", source, current_block, end_block);

        let filter = Filter::new()
            .address(address)
            .event(&event_signature)
            .from_block(current_block)
            .to_block(end_block);

        let logs = provider.get_logs(&filter).await?;

        match execution_mode {
            ExecutionMode::Parallel => {
                for log in logs {
                    let handler = handler.clone();
                    let provider = provider.clone();
                    let templates = templates.clone();

                    tokio::spawn(async move {
                        handler
                            .handle(Context { log, provider, templates, contract_address: address })
                            .await;
                    });
                }
            }
            ExecutionMode::Serial => {
                for log in logs {
                    let templates = templates.clone();
                    let provider = provider.clone();

                    handler
                        .handle(Context { log, provider, templates, contract_address: address })
                        .await;
                }
            }
        }

        current_block = end_block;
    }
}
