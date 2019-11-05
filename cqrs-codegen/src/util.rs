//! Common crate utils used for codegen.

use proc_macro2::TokenStream;
use syn::{punctuated::Punctuated, spanned::Spanned as _, Error, Result};

/// Shorten alias for attribute's meta.
pub(crate) type Meta = Punctuated<syn::NestedMeta, syn::Token![,]>;

/// Dispatches macro `input` to one of implementations (for a struct or for an
/// enum), or returns error if `input` is a union.
pub(crate) fn derive<DS, DE>(
    input: syn::DeriveInput,
    trait_name: &str,
    derive_struct: DS,
    derive_enum: DE,
) -> Result<TokenStream>
where
    DS: Fn(syn::DeriveInput) -> Result<TokenStream>,
    DE: Fn(syn::DeriveInput) -> Result<TokenStream>,
{
    match input.data {
        syn::Data::Struct(_) => derive_struct(input),
        syn::Data::Enum(_) => derive_enum(input),
        syn::Data::Union(data) => Err(Error::new(
            data.union_token.span(),
            format!("Unions are not supported for deriving {}", trait_name),
        )),
    }
}

/// Checks that no attribute with a given `attr_name` exists.
/// Returns error if found.
pub(crate) fn assert_attr_does_not_exist(attrs: &[syn::Attribute], attr_name: &str) -> Result<()> {
    let meta = find_nested_meta_impl(attrs, attr_name)?;
    if let Some((span, _)) = meta {
        return Err(Error::new(
            span,
            format!(
                "Expected no attribute #[{}(...)], but found one.",
                attr_name
            ),
        ));
    }
    Ok(())
}

/// Finds attribute named with a given `attr_name` and returns its inner
/// parameters.
///
/// Errors __if attribute not found__ or if multiple attributes with the same
/// `attr_name` exist.
pub(crate) fn get_nested_meta(attrs: &[syn::Attribute], attr_name: &str) -> Result<Meta> {
    let meta = find_nested_meta(attrs, attr_name)?;
    meta.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "Expected attribute #[{}(...)], but none was found.",
                attr_name
            ),
        )
    })
}

/// Finds attribute named with a given `attr_name` and returns its inner
/// parameters, if found.
///
/// Errors if multiple attributes with the same `attr_name` exist.
pub(crate) fn find_nested_meta(attrs: &[syn::Attribute], attr_name: &str) -> Result<Option<Meta>> {
    let meta_impl = find_nested_meta_impl(attrs, attr_name)?;
    Ok(meta_impl.map(|(_, meta)| meta))
}

/// Finds attribute named with a given `attr_name` and returns its _span
/// (for possible error-reporting)_ and inner parameters, if found.
///
/// Errors if multiple attributes with the same `attr_name` exist.
fn find_nested_meta_impl(
    attrs: &[syn::Attribute],
    attr_name: &str,
) -> Result<Option<(proc_macro2::Span, Meta)>> {
    let mut nested_meta = None;

    for attr in attrs {
        if !attr.path.is_ident(attr_name) {
            continue;
        }

        let meta = match attr.parse_meta()? {
            syn::Meta::List(meta) => meta,
            _ => {
                return Err(Error::new(
                    attr.span(),
                    format!("Wrong attribute format; expected #[{}(...)]", attr_name),
                ))
            }
        };

        if nested_meta.is_some() {
            return Err(Error::new(
                meta.span(),
                format!(
                    "Too many #[{}(...)] attributes specified, \
                     only single attribute is allowed",
                    attr_name
                ),
            ));
        }

        nested_meta.replace((attr.span(), meta.nested));
    }

    Ok(nested_meta)
}

/// Custom simplified [`std::convert::TryInto`] trait, to be implemented on
/// remote types.
///
/// Returns [`Option`] instead of [`Result`], as an error message is expected
/// to be defined at the call site.
pub(crate) trait TryInto<T> {
    /// Performs the possible conversion.
    fn try_into(self) -> Option<T>;
}

/// [`TryInto`] implementations.
mod try_into_impl {
    use super::TryInto;

    /// Generates [`TryInto`] implementation for type `$from` into type `$into`.
    ///
    /// Expects that `$from` is an enum and it's variant `$variant` is a
    /// tuple-variant containing single field of type `$into`.
    macro_rules! try_into_impl {
        ($from:path, $variant:path, $into:path) => {
            impl<'a> TryInto<&'a $into> for &'a $from {
                fn try_into(self) -> Option<&'a $into> {
                    match self {
                        $variant(into) => Some(into),
                        _ => None,
                    }
                }
            }
        };
    }

    try_into_impl! {
        syn::Lit,
        syn::Lit::Str,
        syn::LitStr
    }

    try_into_impl! {
        syn::Lit,
        syn::Lit::Int,
        syn::LitInt
    }
}
