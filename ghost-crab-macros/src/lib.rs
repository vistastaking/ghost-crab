extern crate proc_macro;
use ghost_crab_common::config::{self, ExecutionMode};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal};
use quote::{format_ident, quote};
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

    let config = config::load().unwrap();
    let _ = config.block_handlers.get(name).expect("BlockHandler not found in the config.json");
    let name = Literal::string(name);

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

            fn name(&self) -> String {
                String::from(#name)
            }
        }
    })
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
    let config = config::load().unwrap();

    let abi;
    let network;
    let execution_mode;
    let address;
    let start_block;

    if is_template {
        let source = config.templates.get(&name).expect("Source not found.");

        abi = source.abi.clone();
        network = source.network.clone();
        execution_mode = source.execution_mode.clone().unwrap_or(ExecutionMode::Parallel);
        address = quote! {
            Address::ZERO
        };
        start_block = Literal::u64_suffixed(0);
    } else {
        let source = config.data_sources.get(&name).expect("Source not found.");

        abi = source.abi.clone();
        network = source.network.clone();
        execution_mode = source.execution_mode.clone().unwrap_or(ExecutionMode::Parallel);

        let address_literal = Literal::string(&source.address[2..]);

        address = quote! {
            address!(#address_literal)
        };
        start_block = Literal::u64_suffixed(source.start_block);
    };

    let network_config = config.networks.get(&network).expect("RPC url not found for network");
    let rpc_url = Literal::string(&network_config.rpc_url);
    let requests_per_second = Literal::u64_suffixed(network_config.requests_per_second);

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
            pub fn new() -> Arc<Box<(dyn EventHandler + Send + Sync)>> {
                Arc::new(Box::new(#fn_name {}))
            }
        }

        #[async_trait]
        impl EventHandler for #fn_name {
            async fn handle(&self, #fn_args) {
                let decoded_log = #ctx
                    .log
                    .log_decode::<#contract_name::#event_name>()
                    .unwrap();

                let event = decoded_log.data();

                #fn_body
            }

            fn start_block(&self) -> u64 {
                #start_block
            }

            fn get_source(&self) -> String {
                String::from(#data_source)
            }

            fn is_template(&self) -> bool {
                #is_template
            }

            fn address(&self) -> Address {
                #address
            }

            fn network(&self) -> String {
                String::from(#network)
            }

            fn rpc_url(&self) -> String {
                String::from(#rpc_url)
            }

            fn rate_limit(&self) -> u64 {
                #requests_per_second
            }

            fn execution_mode(&self) -> ExecutionMode {
                #execution_mode
            }

            fn event_signature(&self) -> String {
                #contract_name::#event_name::SIGNATURE.to_string()
            }
        }
    })
}
