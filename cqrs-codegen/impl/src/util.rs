//! Common crate utils used for codegen.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned, Error, Result};

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

/// Renders implementation of a `trait_path` trait for a struct with a given
/// `body`, and optionally renders some arbitrary `impl` block code with a given
/// `additional_code`.
pub(crate) fn render_struct(
    input: &syn::DeriveInput,
    trait_path: TokenStream,
    body: TokenStream,
    additional_code: Option<TokenStream>,
) -> Result<TokenStream> {
    let type_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let additional = additional_code.map(|code| {
        quote! {
            #[automatically_derived]
            impl#impl_generics #type_name#ty_generics #where_clause {
                #code
            }
        }
    });

    Ok(quote! {
        #additional

        #[automatically_derived]
        impl#impl_generics #trait_path for #type_name#ty_generics #where_clause {
            #body
        }
    })
}

/// Checks that no attribute with a given `attr_name` exists.
/// Returns error if found.
#[allow(dead_code)]
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

/// Checks that only given inner arguments `valid_args` are used
/// inside `attr_name` attribute. Passes if attribute doesn't exist at all.
pub(crate) fn assert_valid_attr_args_used(
    attrs: &[syn::Attribute],
    attr_name: &str,
    valid_args: &[&str],
) -> Result<()> {
    let meta = match find_nested_meta(attrs, attr_name)? {
        Some(m) => m,
        None => return Ok(()),
    };

    for m in &meta {
        let meta = match m {
            syn::NestedMeta::Meta(m) => m,
            _ => return Err(Error::new(meta.span(), "Wrong attribute format")),
        };

        if !valid_args.iter().any(|arg| meta.path().is_ident(arg)) {
            return Err(Error::new(meta.span(), "Invalid attribute"));
        }
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

/// Parses specified inner argument `arg` from the given `#[<attr>(...)]` outer
/// attribute, as a flag.
/// Returns `true` if attribute is present, and `false` otherwise.
pub(crate) fn parse_flag(meta: &Meta, arg: &str, valid_args: &[&str], attr: &str) -> Result<bool> {
    let meta = find_arg(meta, arg, valid_args, attr, "")?;

    let flag = match meta {
        None => false,
        Some(syn::Meta::Path(_)) => true,
        _ => return Err(wrong_format(meta, attr, arg, "")),
    };
    Ok(flag)
}

/// Parses specified inner argument `arg` from the given `#[<attr>(...)]` outer
/// attribute, converting it to a type `T` (using [`util::TryInto`])
/// if possible.
pub(crate) fn parse_lit<'meta, T>(
    meta: &'meta Meta,
    arg: &str,
    valid_args: &[&str],
    attr: &str,
    fmt: &str,
) -> Result<&'meta T>
where
    &'meta syn::Lit: TryInto<&'meta T>,
{
    let meta = find_arg(meta, arg, valid_args, attr, fmt)?;

    let meta = meta.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!("Expected to have #[{}({}{})] attribute", attr, arg, fmt,),
        )
    })?;

    let lit = match meta {
        syn::Meta::NameValue(meta) => &meta.lit,
        _ => return Err(wrong_format(meta, attr, arg, fmt)),
    };
    let span = lit.span();
    lit.try_into()
        .ok_or_else(move || wrong_format(span, attr, arg, fmt))
}

/// Finds specified inner argument `arg` from `#[<attr>(...)]` outer attribute.
fn find_arg<'meta>(
    meta: &'meta Meta,
    arg: &str,
    valid_args: &[&str],
    attr: &str,
    fmt: &str,
) -> Result<Option<&'meta syn::Meta>> {
    let mut result = None;

    for meta in meta {
        let meta = match meta {
            syn::NestedMeta::Meta(meta) => meta,
            _ => return Err(wrong_format(meta, attr, arg, fmt)),
        };

        if !valid_args.iter().any(|arg| meta.path().is_ident(arg)) {
            return Err(Error::new(meta.span(), "Invalid attribute"));
        }

        if meta.path().is_ident(arg) && result.replace(meta).is_some() {
            return Err(Error::new(
                meta.span(),
                format!("Only one #[{}({}{})] attribute is allowed", attr, arg, fmt,),
            ));
        }
    }

    Ok(result)
}

/// Constructs error message about wrong attribute format.
fn wrong_format<S>(span: S, attr: &str, arg: &str, fmt: &str) -> Error
where
    S: Spanned,
{
    Error::new(
        span.span(),
        format!(
            "Wrong attribute format; expected #[{}({}{})]",
            attr, arg, fmt,
        ),
    )
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
