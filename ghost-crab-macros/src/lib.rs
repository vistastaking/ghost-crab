extern crate proc_macro;
use ghost_crab_common::config::{Config, ExecutionMode};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal};
use quote::{format_ident, quote};
use std::fs;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn event_handler(metadata: TokenStream, input: TokenStream) -> TokenStream {
    create_handler(metadata, input, false)
}

#[proc_macro_attribute]
pub fn template(metadata: TokenStream, input: TokenStream) -> TokenStream {
    create_handler(metadata, input, true)
}

#[proc_macro_attribute]
pub fn block_handler(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let name = metadata.to_string();
    let name = name.trim();

    if name.is_empty() {
        panic!("The source is missing");
    }

    let config = get_config();
    let source = config.block_handlers.get(name).expect("Source not found.");

    let step = Literal::u64_suffixed(source.step);
    let start_block = Literal::u64_suffixed(source.start_block);
    let network = Literal::string(&source.network);

    let execution_mode = match source.execution_mode {
        Some(ExecutionMode::Serial) => quote! { ExecutionMode::Serial },
        _ => quote! { ExecutionMode::Parallel },
    };

    let parsed = parse_macro_input!(input as ItemFn);
    let fn_name = parsed.sig.ident.clone();
    let fn_body = parsed.block;
    let fn_args = parsed.sig.inputs.clone();

    TokenStream::from(quote! {
        pub struct #fn_name;

        impl #fn_name {
            pub fn new() -> Arc<Box<(dyn BlockHandler + Send + Sync)>> {
                Arc::new(Box::new(#fn_name {}))
            }
        }

        #[async_trait]
        impl BlockHandler for #fn_name {
            async fn handle(&self, #fn_args) {
                #fn_body
            }

            fn step(&self) -> u64 {
                #step
            }

            fn network(&self) -> String {
                #network
            }

            fn start_block(&self) -> u64 {
                #start_block
            }

            fn execution_mode(&self) -> ExecutionMode {
                #execution_mode
            }
        }
    })
}

fn get_config() -> Config {
    let current_dir = std::env::current_dir().expect("Current directory not found");
    let config_json_path = current_dir.join("config.json");
    let content = fs::read_to_string(config_json_path).expect("Error reading config file");
    let config: Config = serde_json::from_str(&content).expect("Error parsing config file");
    return config;
}

fn get_source_and_event(metadata: TokenStream) -> (String, Ident) {
    let metadata_string = metadata.to_string();
    let mut metadata_split = metadata_string.split('.');

    let name = metadata_split.next().expect("The source is missing");
    let name = String::from(name.trim());

    if name.is_empty() {
        panic!("The source is empty");
    }

    let event_name = metadata_split.next().expect("The event name is missing");
    let event_name = String::from(event_name.trim());

    if event_name.is_empty() {
        panic!("The event name is empty");
    }

    // Checks that the metadata does not have more than 3 comma separated values
    let should_be_none = metadata_split.next();
    if should_be_none.is_some() {
        panic!("The metadata has too many values");
    }

    let event_name = syn::Ident::new(&event_name, proc_macro2::Span::call_site());
    return (name, event_name);
}

fn get_context_identifier(parsed: ItemFn) -> Ident {
    let first_input = parsed.sig.inputs[0].clone();

    let ctx = if let syn::FnArg::Typed(pat_type) = first_input {
        if let syn::Pat::Ident(pat_ident) = *pat_type.pat {
            pat_ident.ident
        } else {
            panic!("Malformed handler function arguments")
        }
    } else {
        panic!("Malformed handler function arguments")
    };

    return ctx;
}

fn create_handler(metadata: TokenStream, input: TokenStream, is_template: bool) -> TokenStream {
    let (name, event_name) = get_source_and_event(metadata);
    let config = get_config();

    let abi;
    let network;
    let execution_mode;

    if is_template {
        let source = config.templates.get(&name).expect("Source not found.");

        abi = source.abi.clone();
        network = source.network.clone();
        execution_mode = source.execution_mode.clone().unwrap_or(ExecutionMode::Parallel);
    } else {
        let source = config.data_sources.get(&name).expect("Source not found.");

        abi = source.abi.clone();
        network = source.network.clone();
        execution_mode = source.execution_mode.clone().unwrap_or(ExecutionMode::Parallel);
    };

    let abi = Literal::string(&abi);
    let network = Literal::string(&network);

    let execution_mode = match execution_mode {
        ExecutionMode::Parallel => quote! {
            ExecutionMode::Parallel
        },
        ExecutionMode::Serial => quote! {
            ExecutionMode::Serial
        },
    };

    let parsed = parse_macro_input!(input as ItemFn);
    let fn_name = parsed.sig.ident.clone();
    let fn_args = parsed.sig.inputs.clone();
    let fn_body = parsed.block.clone();
    let ctx = get_context_identifier(parsed);

    let contract_name = format_ident!("{}Contract", fn_name);
    let data_source = Literal::string(&name);

    TokenStream::from(quote! {
        sol!(
            #[sol(rpc)]
            #contract_name,
            #abi
        );

        pub struct #fn_name;

        impl #fn_name {
            pub fn new() -> Arc<Box<(dyn Handler + Send + Sync)>> {
                Arc::new(Box::new(#fn_name {}))
            }
        }

        #[async_trait]
        impl Handler for #fn_name {
            async fn handle(&self, #fn_args) {
                let decoded_log = #ctx
                    .log
                    .log_decode::<#contract_name::#event_name>()
                    .unwrap();

                let event = decoded_log.data();

                #fn_body
            }

            fn get_source(&self) -> String {
                String::from(#data_source)
            }

            fn is_template(&self) -> bool {
                #is_template
            }

            fn network(&self) - String {
                String::from(#network)
            }

            fn execution_mode(&self) -> ExecutionMode {
                #execution_mode
            }

            fn get_event_signature(&self) -> String {
                #contract_name::#event_name::SIGNATURE.to_string()
            }
        }
    })
}
