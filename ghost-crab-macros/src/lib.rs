extern crate proc_macro;
use ghost_crab_common::config;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, ItemFn};

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

    let config = config::load().expect("config.json not found");
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
    let config = config::load().expect("config.json not found");

    let abi = if is_template {
        let source = config.templates.get(&name).expect("Source not found.");
        Literal::string(&source.abi)
    } else {
        let source = config.data_sources.get(&name).expect("Source not found.");
        Literal::string(&source.abi)
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
                    .expect("Error decoding log data");

                let event = decoded_log.data();

                #fn_body
            }

            fn name(&self) -> String {
                String::from(#data_source)
            }

            fn event_signature(&self) -> String {
                #contract_name::#event_name::SIGNATURE.to_string()
            }
        }
    })
}

#[proc_macro_derive(StructToPGTable)]
pub fn derive_struct_to_postgres_table(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("StructToPGTable only supports structs with named fields"),
        },
        _ => panic!("StructToPGTable only supports structs"),
    };

    let field_defs = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            (stringify!(#name).trim_start_matches('_').to_string(), stringify!(#ty).to_string())
        }
    });

    let expanded = quote! {
        #[automatically_derived]
        impl PostgresDDL for #name {
            fn fields() -> Vec<(String, String)> {
                vec![
                    #(#field_defs),*
                ]
            }

            fn type_to_pg_type(ty: &str) -> &'static str {
                match ty {
                    "i32" => "INTEGER",
                    "i64" => "BIGINT",
                    "f32" => "REAL",
                    "f64" => "DOUBLE PRECISION",
                    "String" => "TEXT",
                    "bool" => "BOOLEAN",
                    _ => panic!("Unsupported type: {}", ty),
                }
            }

            fn ddl() -> String {
                let type_name = std::any::type_name::<Self>();
                let table_name = type_name
                    .split("::")
                    .last()
                    .unwrap_or(type_name)
                    .to_lowercase();

                let columns = Self::fields()
                    .into_iter()
                    .map(|(name, ty)| {
                        let pg_type = Self::type_to_pg_type(&ty);
                        format!("    {} {}", name, pg_type)
                    })
                    .collect::<Vec<_>>()
                    .join(",\n");

                format!("CREATE TABLE IF NOT EXISTS {} (\n{}\n);", table_name, columns)
            }

            fn create_table(conn: &mut postgres::Client) -> Result<(), postgres::Error> {
                conn.batch_execute(&Self::ddl())
            }
        }
    };

    TokenStream::from(expanded)
}
