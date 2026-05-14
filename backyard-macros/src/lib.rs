use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Job, attributes(job))]
pub fn derive_job(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        impl backyard_core::Job for #name {
            const NAME: &'static str = #name_str;
        }

        backyard_core::inventory::submit! {
            backyard_core::registry::JobRegistration {
                name: #name_str,
                handler: |raw, ctx| {
                    Box::pin(async move {
                        let job: #name = serde_json::from_slice(raw)?;
                        job.execute(&ctx).await
                    })
                },
            }
        }
    };

    TokenStream::from(expanded)
}
