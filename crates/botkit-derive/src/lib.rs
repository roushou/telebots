//! Derive macro for [`botkit::CommandSpec`].
//!
//! Generates parsing, the Telegram menu, and `/help` text for a command enum.
//! The generated code references only botkit's own public types, so the
//! consuming bot never depends on teloxide.
//!
//! Supported `#[command(...)]` attributes:
//!
//! - enum: `rename_rule`, `description`, `prefix`, `separator`
//! - variant: `description`, `rename`, `aliases`, `hide`
//!
//! Variants are unit (no arguments) or carry exactly one unnamed field. The
//! field is parsed with `FromStr`; use a raw `String` and validate yourself
//! so usage errors reach the user instead of the update being dropped.

use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr};

/// Options shared by every variant of a command enum.
#[derive(Default)]
struct EnumAttrs {
    rename_rule: Option<String>,
    description: Option<String>,
    prefix: Option<String>,
    separator: Option<String>,
}

/// Per-variant options.
#[derive(Default)]
struct VariantAttrs {
    description: Option<String>,
    rename: Option<String>,
    aliases: Vec<String>,
    hide: bool,
}

/// A fully-resolved command variant ready for codegen.
struct Variant {
    /// The command as Telegram sees it, prefix included (e.g. `/price`).
    prefixed: String,
    /// The bare command name (e.g. `price`).
    name: String,
    /// Bare aliases (no prefix).
    aliases: Vec<String>,
    /// Human-readable description.
    description: String,
    /// The `Self::Variant(...)` constructor (arguments parsed from `args`).
    constructor: TokenStream2,
    /// Hidden from `/help` and the menu, but still parseable.
    hide: bool,
}

#[proc_macro_derive(CommandSpec, attributes(command))]
pub fn derive_command_spec(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let ident = &input.ident;

    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                ident,
                "`CommandSpec` can only be derived for enums",
            ));
        }
    };

    let enum_attrs = EnumAttrs::parse(&input.attrs)?;
    let prefix = enum_attrs.prefix.as_deref().unwrap_or("/");
    let separator = enum_attrs.separator.as_deref().unwrap_or(" ");
    let rename_rule = enum_attrs.rename_rule.as_deref().unwrap_or("identity");

    let mut variants = Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        let attrs = VariantAttrs::parse(&variant.attrs)?;

        let name = match &attrs.rename {
            Some(rename) => rename.clone(),
            None => apply_rename_rule(
                rename_rule,
                &variant.ident.to_string(),
                variant.ident.span(),
            )?,
        };

        variants.push(Variant {
            prefixed: format!("{prefix}{name}"),
            name,
            aliases: attrs.aliases,
            description: attrs.description.unwrap_or_default(),
            constructor: constructor(variant)?,
            hide: attrs.hide,
        });
    }

    let visible = variants.iter().filter(|v| !v.hide).collect::<Vec<_>>();

    let help = build_help(&enum_attrs, prefix, &visible);

    let menu_entries = visible.iter().map(|v| {
        let command = &v.prefixed;
        let description = &v.description;
        quote! {
            ::botkit::MenuEntry {
                command: #command.to_string(),
                description: #description.to_string(),
            }
        }
    });

    let prefixed_matches = variants.iter().map(|v| &v.prefixed);
    let constructors = variants.iter().map(|v| &v.constructor);
    let alias_arms = variants.iter().filter(|v| !v.aliases.is_empty()).map(|v| {
        let aliases = v.aliases.iter().map(|alias| format!("{prefix}{alias}"));
        let constructor = &v.constructor;
        quote! {
            c if [#(#aliases),*].contains(&c) => ::std::option::Option::Some(#constructor),
        }
    });

    Ok(quote! {
        impl ::botkit::CommandSpec for #ident {
            fn help() -> ::std::string::String {
                #help.to_string()
            }

            fn menu() -> ::std::vec::Vec<::botkit::MenuEntry> {
                ::std::vec![#(#menu_entries),*]
            }

            fn parse(s: &str, bot_name: &str) -> ::std::option::Option<Self> {
                use ::std::str::FromStr;

                let mut words = s.splitn(2, #separator);
                let mut full_command = words.next().unwrap().split('@');
                let command = full_command.next().unwrap();

                let bot_username = full_command.next();
                match bot_username {
                    ::std::option::Option::None => {}
                    ::std::option::Option::Some(username) if username.eq_ignore_ascii_case(bot_name) => {}
                    ::std::option::Option::Some(_) => return ::std::option::Option::None,
                }

                let args = words.next().unwrap_or("").to_owned();
                match command {
                    #(#prefixed_matches => ::std::option::Option::Some(#constructors),)*
                    #(#alias_arms)*
                    _ => ::std::option::Option::None,
                }
            }
        }
    })
}

/// The `/help` text: global description (if any), then one line per visible
/// command, formatted exactly as teloxide's command list used to render.
fn build_help(enum_attrs: &EnumAttrs, prefix: &str, visible: &[&Variant]) -> String {
    let mut out = String::new();
    if let Some(global) = &enum_attrs.description {
        out.push_str(global);
        out.push_str("\n\n");
    }
    for (i, variant) in visible.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(prefix);
        out.push_str(&variant.name);
        for alias in &variant.aliases {
            out.push_str(", ");
            out.push_str(prefix);
            out.push_str(alias);
        }
        if !variant.description.is_empty() {
            out.push_str(" — ");
            out.push_str(&variant.description);
        }
    }
    out
}

fn apply_rename_rule(rule: &str, name: &str, span: proc_macro2::Span) -> syn::Result<String> {
    let renamed = match rule {
        "lowercase" => name.to_lowercase(),
        "UPPERCASE" => name.to_uppercase(),
        "PascalCase" => name.to_pascal_case(),
        "camelCase" => name.to_lower_camel_case(),
        "snake_case" => name.to_snake_case(),
        "SCREAMING_SNAKE_CASE" => name.to_shouty_snake_case(),
        "kebab-case" => name.to_kebab_case(),
        "SCREAMING-KEBAB-CASE" => name.to_shouty_kebab_case(),
        "identity" => name.to_owned(),
        other => {
            return Err(syn::Error::new(
                span,
                format!(
                    "invalid rename rule `{other}` (supported: `lowercase`, `UPPERCASE`, \
                     `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, \
                     `kebab-case`, `SCREAMING-KEBAB-CASE`, `identity`)"
                ),
            ));
        }
    };
    Ok(renamed)
}

/// The `Self::Variant(...)` expression for a variant, parsing `args` per its
/// fields: unit variants take nothing, a single unnamed field is `FromStr`.
fn constructor(variant: &syn::Variant) -> syn::Result<TokenStream2> {
    let name = &variant.ident;
    match &variant.fields {
        Fields::Unit => Ok(quote! { Self::#name }),
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() != 1 {
                return Err(syn::Error::new_spanned(
                    variant,
                    "`CommandSpec` variants take zero or one unnamed field; use a raw `String` \
                     and parse it in the command object",
                ));
            }
            let ty = &fields.unnamed.first().unwrap().ty;
            Ok(quote! {
                Self::#name(<#ty as ::std::str::FromStr>::from_str(&args).ok()?)
            })
        }
        Fields::Named(_) => Err(syn::Error::new_spanned(
            variant,
            "`CommandSpec` does not support named fields",
        )),
    }
}

impl EnumAttrs {
    fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut out = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("command") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename_rule") {
                    out.rename_rule = Some(meta_string(&meta)?);
                } else if meta.path.is_ident("description") {
                    out.description = Some(meta_string(&meta)?);
                } else if meta.path.is_ident("prefix") {
                    out.prefix = Some(meta_string(&meta)?);
                } else if meta.path.is_ident("separator") {
                    out.separator = Some(meta_string(&meta)?);
                } else {
                    return Err(meta.error("unsupported `command` attribute on enum"));
                }
                Ok(())
            })?;
        }
        Ok(out)
    }
}

impl VariantAttrs {
    fn parse(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut out = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("command") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("description") {
                    out.description = Some(meta_string(&meta)?);
                } else if meta.path.is_ident("rename") {
                    out.rename = Some(meta_string(&meta)?);
                } else if meta.path.is_ident("aliases") {
                    let raw = meta_string(&meta)?;
                    out.aliases = raw
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect();
                } else if meta.path.is_ident("hide") {
                    out.hide = true;
                } else {
                    return Err(meta.error("unsupported `command` attribute on variant"));
                }
                Ok(())
            })?;
        }
        Ok(out)
    }
}

fn meta_string(meta: &syn::meta::ParseNestedMeta) -> syn::Result<String> {
    let lit: LitStr = meta.value()?.parse()?;
    Ok(lit.value())
}
