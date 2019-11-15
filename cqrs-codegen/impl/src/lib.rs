mod event;
mod util;

use proc_macro2::TokenStream;

#[cfg(not(feature = "watt"))]
/// Re-exports proc macro `$fn` as is.
macro_rules! export {
    ($fn:ident) => {
        pub use event::$fn;
    };
}

#[cfg(feature = "watt")]
/// Re-exports proc macro `$fn` via WASM ABI.
macro_rules! export {
    ($fn:ident) => {
        #[no_mangle]
        pub extern "C" fn $fn(input: TokenStream) -> TokenStream {
            expand(syn::parse2(input), event::$fn)
        }
    };
}

/// Performs expansion of a given proc macro implementation.
pub fn expand<TS: From<TokenStream>>(
    input: syn::Result<syn::DeriveInput>,
    macro_impl: fn(syn::DeriveInput) -> syn::Result<TokenStream>,
) -> TS {
    match input.and_then(macro_impl) {
        Ok(res) => res.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

export!(aggregate_event_derive);
export!(event_derive);
export!(registered_event_derive);
export!(versioned_event_derive);
