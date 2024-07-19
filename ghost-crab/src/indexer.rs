use crate::block_handler::{process_logs_block, BlockConfig, BlockHandlerInstance};
use crate::cache::manager::RPC_MANAGER;
use crate::config;
use crate::handler::{HandleInstance, HandlerConfig};
use crate::process_logs::process_logs;
use crate::server::Server;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Clone)]
pub struct TemplateManager {
    tx: Sender<HandlerConfig>,
}

pub struct Template {
    pub start_block: u64,
    pub address: String,
    pub handler: HandleInstance,
}

impl TemplateManager {
    pub async fn start(&self, template: Template) {
        let provider = RPC_MANAGER.lock().await.get(template.handler.network()).await;

        self.tx
            .send(HandlerConfig {
                start_block: template.start_block,
                address: template.address.clone(),
                step: 10_000,
                provider,
                handler: template.handler,
                templates: self.clone(),
            })
            .await
            .unwrap();
    }
}

pub struct Indexer {
    config: config::Config,
    handlers: Vec<HandlerConfig>,
    block_handlers: Vec<BlockConfig>,
    rx: Receiver<HandlerConfig>,
    templates: TemplateManager,
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}

impl Indexer {
    pub fn new() -> Indexer {
        let config = config::load();
        let (tx, rx) = mpsc::channel::<HandlerConfig>(1);

        let templates = TemplateManager { tx };

        let server = Server::new(3000);
        server.start();

        Indexer {
            config: config.clone(),
            handlers: Vec::new(),
            block_handlers: Vec::new(),
            rx,
            templates,
        }
    }

    pub async fn load_event_handler(&mut self, handler: HandleInstance) {
        if handler.is_template() {
            return;
        }

        let provider = RPC_MANAGER.lock().await.get(handler.network()).await;
        let source = self.config.data_sources.get(&handler.get_source()).unwrap();

        self.handlers.push(HandlerConfig {
            start_block: source.start_block,
            address: source.address.clone(),
            step: 10_000,
            provider,
            handler,
            templates: self.templates.clone(),
        });
    }

    pub async fn load_block_handler(&mut self, handler: BlockHandlerInstance) {
        let provider = RPC_MANAGER.lock().await.get(handler.network()).await;

        self.block_handlers.push(BlockConfig {
            handler,
            provider,
            templates: self.templates.clone(),
        });
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
