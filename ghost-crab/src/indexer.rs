use crate::block_handler::{process_logs_block, BlockConfig, BlockHandlerInstance};
use crate::handler::{HandleInstance, HandlerConfig};
use crate::process_logs::process_logs;
use alloy::primitives::Address;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone)]
pub struct TemplateManager {
    tx: Sender<HandlerConfig>,
}

pub struct Template {
    pub start_block: u64,
    pub address: Address,
    pub handler: HandleInstance,
}

impl TemplateManager {
    pub async fn start(&self, template: Template) -> Result<(), SendError<HandlerConfig>> {
        self.tx
            .send(HandlerConfig {
                start_block: template.start_block,
                address: template.address.clone(),
                step: 10_000,
                handler: template.handler,
                templates: self.clone(),
            })
            .await
    }
}

pub struct Indexer {
    handlers: Vec<HandlerConfig>,
    block_handlers: Vec<BlockConfig>,
    rx: Receiver<HandlerConfig>,
    templates: TemplateManager,
}

impl Indexer {
    pub fn new() -> Indexer {
        let (tx, rx) = mpsc::channel::<HandlerConfig>(1);

        Indexer {
            handlers: Vec::new(),
            block_handlers: Vec::new(),
            templates: TemplateManager { tx },
            rx,
        }
    }

    pub async fn load_event_handler(&mut self, handler: HandleInstance) {
        if handler.is_template() {
            return;
        }

        self.handlers.push(HandlerConfig {
            start_block: handler.start_block(),
            address: handler.address(),
            step: 10_000,
            handler,
            templates: self.templates.clone(),
        });
    }

    pub async fn load_block_handler(&mut self, handler: BlockHandlerInstance) {
        self.block_handlers.push(BlockConfig { handler, templates: self.templates.clone() });
    }

    pub async fn start(mut self) {
        for block_handler in self.block_handlers {
            tokio::spawn(async move {
                if let Err(error) = process_logs_block(block_handler).await {
                    println!("Error processing logs for block handler: {error}");
                }
            });
        }

        for handler in self.handlers {
            tokio::spawn(async move {
                if let Err(error) = process_logs(handler).await {
                    println!("Error processing logs for handler: {error}");
                }
            });
        }

        // For dynamic sources (Templates)
        while let Some(handler) = self.rx.recv().await {
            tokio::spawn(async move {
                if let Err(error) = process_logs(handler).await {
                    println!("Error processing logs for handler: {error}");
                }
            });
        }
    }
}
