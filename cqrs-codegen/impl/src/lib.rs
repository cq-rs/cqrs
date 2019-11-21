mod aggregate;
mod command;
mod event;
mod util;

use proc_macro2::TokenStream;

#[cfg(not(feature = "watt"))]
/// Re-exports proc macro `$fn` as is.
macro_rules! export {
    ($mod:ident::$fn:ident) => {
        pub use $mod::$fn;
    };
    ($mod:ident::$fn:ident as $export:ident) => {
        pub use $mod::$fn as $export;
    };
}

#[cfg(feature = "watt")]
/// Re-exports proc macro `$fn` via WASM ABI.
macro_rules! export {
    ($mod:ident::$fn:ident) => {
        export! {$mod::$fn as $fn}
    };
    ($mod:ident::$fn:ident as $export:ident) => {
        #[no_mangle]
        pub extern "C" fn $export(input: TokenStream) -> TokenStream {
            expand(syn::parse2(input), $mod::$fn)
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

export!(aggregate::derive as aggregate_derive);
export!(command::derive as command_derive);
export!(event::aggregate_event_derive);
export!(event::event_derive);
export!(event::registered_event_derive);
export!(event::versioned_event_derive);
