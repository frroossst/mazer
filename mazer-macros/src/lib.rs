//! Procedural macros for Mazer language function metadata.
//!
//! This crate provides the `FuncMeta` derive macro which generates
//! comprehensive metadata accessors for function enums.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput, Data, Fields, Attribute, Expr, ExprArray,
    ExprLit, Lit, Meta, Token, Ident,
    punctuated::Punctuated,
};

/// Metadata parsed from a `#[func(...)]` attribute
struct FuncAttr {
    names: Vec<String>,
    arity: AritySpec,
    doc: String,
    symbol: Option<String>,
}

/// Arity specification parsed from attribute
#[derive(Clone)]
enum AritySpec {
    Fixed(usize),
    Range(usize, usize),
    AtLeast(usize),
    Variadic,
}

impl Default for FuncAttr {
    fn default() -> Self {
        Self {
            names: Vec::new(),
            arity: AritySpec::Variadic,
            doc: String::new(),
            symbol: None,
        }
    }
}

/// Derive macro for function metadata.
///
/// # Example
///
/// ```ignore
/// use mazer_macros::FuncMeta;
///
/// #[derive(FuncMeta)]
/// pub enum ShowFunc {
///     #[func(names = ["sin"], arity = Fixed(1), doc = "Sine function")]
///     Sin,
///
///     #[func(names = ["arcsin", "asin"], arity = Fixed(1), doc = "Inverse sine")]
///     Arcsin,
///
///     #[func(names = ["+", "add"], arity = Variadic, doc = "Addition")]
///     Add,
///
///     MaybeFunc(String), // No attribute needed for fallback variant
/// }
/// ```
///
/// This generates:
/// - `names(&self) -> &'static [&'static str]` - all string names for a variant
/// - `canonical_name(&self) -> &'static str` - the primary/first name
/// - `arity(&self) -> Arguments` - arity information
/// - `doc(&self) -> &'static str` - documentation string
/// - `symbol(&self) -> Option<&'static str>` - optional MathML symbol override
/// - `from_name(s: &str) -> Self` - parse from string
/// - `all_functions() -> Vec<FuncInfo>` - all functions with metadata
#[proc_macro_derive(FuncMeta, attributes(func))]
pub fn derive_func_meta(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(&input, "FuncMeta can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let mut variant_metas: Vec<(Ident, Option<FuncAttr>, bool)> = Vec::new();

    for variant in &data_enum.variants {
        let has_fields = !matches!(&variant.fields, Fields::Unit);
        let func_attr = parse_func_attr(&variant.attrs);
        variant_metas.push((variant.ident.clone(), func_attr, has_fields));
    }

    let names_impl = generate_names_impl(name, &variant_metas);
    let canonical_name_impl = generate_canonical_name_impl(name, &variant_metas);
    let arity_impl = generate_arity_impl(name, &variant_metas);
    let doc_impl = generate_doc_impl(name, &variant_metas);
    let symbol_impl = generate_symbol_impl(name, &variant_metas);
    let from_name_impl = generate_from_name_impl(name, &variant_metas);
    let all_functions_impl = generate_all_functions_impl(name, &variant_metas);
    let func_info_struct = generate_func_info_struct();

    let expanded = quote! {
        #func_info_struct

        impl #name {
            #names_impl
            #canonical_name_impl
            #arity_impl
            #doc_impl
            #symbol_impl
            #from_name_impl
            #all_functions_impl
        }
    };

    TokenStream::from(expanded)
}

fn parse_func_attr(attrs: &[Attribute]) -> Option<FuncAttr> {
    for attr in attrs {
        if !attr.path().is_ident("func") {
            continue;
        }

        let mut func_attr = FuncAttr::default();

        let Ok(nested) = attr.parse_args_with(
            Punctuated::<Meta, Token![,]>::parse_terminated
        ) else {
            continue;
        };

        for meta in nested {
            match &meta {
                Meta::NameValue(nv) if nv.path.is_ident("names") => {
                    if let Expr::Array(ExprArray { elems, .. }) = &nv.value {
                        for elem in elems {
                            if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = elem {
                                func_attr.names.push(s.value());
                            }
                        }
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("arity") => {
                    func_attr.arity = parse_arity_expr(&nv.value);
                }
                Meta::NameValue(nv) if nv.path.is_ident("doc") => {
                    if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                        func_attr.doc = s.value();
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("symbol") => {
                    if let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                        func_attr.symbol = Some(s.value());
                    }
                }
                _ => {}
            }
        }

        return Some(func_attr);
    }

    None
}

fn parse_arity_expr(expr: &Expr) -> AritySpec {
    // Parse expressions like Fixed(1), Range(1, 4), AtLeast(2), Variadic
    let expr_str = quote!(#expr).to_string().replace(" ", "");

    if expr_str == "Variadic" {
        return AritySpec::Variadic;
    }

    if let Some(rest) = expr_str.strip_prefix("Fixed(") {
        if let Some(num_str) = rest.strip_suffix(")") {
            if let Ok(n) = num_str.parse::<usize>() {
                return AritySpec::Fixed(n);
            }
        }
    }

    if let Some(rest) = expr_str.strip_prefix("AtLeast(") {
        if let Some(num_str) = rest.strip_suffix(")") {
            if let Ok(n) = num_str.parse::<usize>() {
                return AritySpec::AtLeast(n);
            }
        }
    }

    if let Some(rest) = expr_str.strip_prefix("Range(") {
        if let Some(nums_str) = rest.strip_suffix(")") {
            let parts: Vec<&str> = nums_str.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(a), Ok(b)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    return AritySpec::Range(a, b);
                }
            }
        }
    }

    AritySpec::Variadic
}

fn generate_names_impl(enum_name: &Ident, variants: &[(Ident, Option<FuncAttr>, bool)]) -> TokenStream2 {
    let arms: Vec<_> = variants.iter().map(|(ident, attr, has_fields)| {
        let pattern = if *has_fields {
            quote! { #enum_name::#ident(_) }
        } else {
            quote! { #enum_name::#ident }
        };

        if let Some(attr) = attr {
            let names = &attr.names;
            quote! { #pattern => &[#(#names),*] }
        } else if *has_fields {
            quote! { #pattern => &[] }
        } else {
            let name_str = ident.to_string().to_lowercase();
            quote! { #pattern => &[#name_str] }
        }
    }).collect();

    quote! {
        /// Returns all string names that map to this function variant.
        ///
        /// For example, `ShowFunc::Add.names()` might return `&["+", "add"]`.
        pub fn names(&self) -> &'static [&'static str] {
            match self {
                #(#arms),*
            }
        }
    }
}

fn generate_canonical_name_impl(enum_name: &Ident, variants: &[(Ident, Option<FuncAttr>, bool)]) -> TokenStream2 {
    let arms: Vec<_> = variants.iter().map(|(ident, attr, has_fields)| {
        let pattern = if *has_fields {
            quote! { #enum_name::#ident(s) }
        } else {
            quote! { #enum_name::#ident }
        };

        if let Some(attr) = attr {
            let canonical = attr.names.first().cloned().unwrap_or_else(|| ident.to_string().to_lowercase());
            quote! { #pattern => #canonical }
        } else if *has_fields {
            quote! { #pattern => s.as_str() }
        } else {
            let name_str = ident.to_string().to_lowercase();
            quote! { #pattern => #name_str }
        }
    }).collect();

    quote! {
        /// Returns the canonical (primary) name for this function.
        ///
        /// This is the first name in the `names` list, used for display purposes.
        pub fn canonical_name(&self) -> &str {
            match self {
                #(#arms),*
            }
        }
    }
}

fn generate_arity_impl(enum_name: &Ident, variants: &[(Ident, Option<FuncAttr>, bool)]) -> TokenStream2 {
    let arms: Vec<_> = variants.iter().map(|(ident, attr, has_fields)| {
        let pattern = if *has_fields {
            quote! { #enum_name::#ident(_) }
        } else {
            quote! { #enum_name::#ident }
        };

        let arity = if let Some(attr) = attr {
            match &attr.arity {
                AritySpec::Fixed(n) => quote! { Arguments::Fixed(#n) },
                AritySpec::Range(a, b) => quote! { Arguments::Range(#a, #b) },
                AritySpec::AtLeast(n) => quote! { Arguments::Atleast(#n) },
                AritySpec::Variadic => quote! { Arguments::Variadic },
            }
        } else {
            quote! { Arguments::Variadic }
        };

        quote! { #pattern => #arity }
    }).collect();

    quote! {
        /// Returns the arity (argument count specification) for this function.
        ///
        /// This indicates how many arguments the function accepts.
        pub fn arity(&self) -> Arguments {
            match self {
                #(#arms),*
            }
        }
    }
}

fn generate_doc_impl(enum_name: &Ident, variants: &[(Ident, Option<FuncAttr>, bool)]) -> TokenStream2 {
    let arms: Vec<_> = variants.iter().map(|(ident, attr, has_fields)| {
        let pattern = if *has_fields {
            quote! { #enum_name::#ident(_) }
        } else {
            quote! { #enum_name::#ident }
        };

        let doc = if let Some(attr) = attr {
            &attr.doc
        } else {
            ""
        };

        quote! { #pattern => #doc }
    }).collect();

    quote! {
        /// Returns the documentation string for this function.
        ///
        /// This provides a human-readable description suitable for LSP hover info.
        pub fn doc(&self) -> &'static str {
            match self {
                #(#arms),*
            }
        }
    }
}

fn generate_symbol_impl(enum_name: &Ident, variants: &[(Ident, Option<FuncAttr>, bool)]) -> TokenStream2 {
    let arms: Vec<_> = variants.iter().map(|(ident, attr, has_fields)| {
        let pattern = if *has_fields {
            quote! { #enum_name::#ident(_) }
        } else {
            quote! { #enum_name::#ident }
        };

        let symbol = if let Some(attr) = attr {
            if let Some(s) = &attr.symbol {
                quote! { Some(#s) }
            } else {
                quote! { None }
            }
        } else {
            quote! { None }
        };

        quote! { #pattern => #symbol }
    }).collect();

    quote! {
        /// Returns an optional symbol override for MathML rendering.
        ///
        /// If `Some`, this symbol should be used instead of the canonical name.
        pub fn symbol(&self) -> Option<&'static str> {
            match self {
                #(#arms),*
            }
        }
    }
}

fn generate_from_name_impl(enum_name: &Ident, variants: &[(Ident, Option<FuncAttr>, bool)]) -> TokenStream2 {
    let mut match_arms: Vec<TokenStream2> = Vec::new();
    let mut fallback_variant: Option<Ident> = None;

    for (ident, attr, has_fields) in variants {
        if *has_fields {
            fallback_variant = Some(ident.clone());
            continue;
        }

        let names = if let Some(attr) = attr {
            attr.names.clone()
        } else {
            vec![ident.to_string().to_lowercase()]
        };

        for name in names {
            match_arms.push(quote! { #name => #enum_name::#ident });
        }
    }

    let fallback = if let Some(fb) = fallback_variant {
        quote! { _ => #enum_name::#fb(s.to_string()) }
    } else {
        // This shouldn't happen if there's a MaybeFunc variant, but just in case
        quote! { _ => panic!("Unknown function: {}", s) }
    };

    quote! {
        /// Parse a function from its string name.
        ///
        /// This matches against all registered names (including aliases).
        /// Unknown names fall back to the `MaybeFunc` variant if present.
        pub fn from_name(s: &str) -> Self {
            match s {
                #(#match_arms,)*
                #fallback
            }
        }
    }
}

fn generate_all_functions_impl(_enum_name: &Ident, variants: &[(Ident, Option<FuncAttr>, bool)]) -> TokenStream2 {
    let mut infos: Vec<TokenStream2> = Vec::new();

    for (ident, attr, has_fields) in variants {
        if *has_fields {
            continue; // Skip variants with fields (like MaybeFunc)
        }

        let (names, arity, doc, symbol) = if let Some(attr) = attr {
            let names = &attr.names;
            let arity = match &attr.arity {
                AritySpec::Fixed(n) => quote! { Arguments::Fixed(#n) },
                AritySpec::Range(a, b) => quote! { Arguments::Range(#a, #b) },
                AritySpec::AtLeast(n) => quote! { Arguments::Atleast(#n) },
                AritySpec::Variadic => quote! { Arguments::Variadic },
            };
            let doc = &attr.doc;
            let symbol = if let Some(s) = &attr.symbol {
                quote! { Some(#s) }
            } else {
                quote! { None }
            };
            (quote! { &[#(#names),*] }, arity, doc.clone(), symbol)
        } else {
            let name_str = ident.to_string().to_lowercase();
            (
                quote! { &[#name_str] },
                quote! { Arguments::Variadic },
                String::new(),
                quote! { None },
            )
        };

        let variant_name = ident.to_string();

        infos.push(quote! {
            FuncInfo {
                variant_name: #variant_name,
                names: #names,
                arity: #arity,
                doc: #doc,
                symbol: #symbol,
            }
        });
    }

    quote! {
        /// Returns metadata for all built-in functions.
        ///
        /// This is useful for LSP completions, documentation generation, etc.
        /// Does not include the fallback `MaybeFunc` variant.
        pub fn all_functions() -> Vec<FuncInfo> {
            vec![
                #(#infos),*
            ]
        }
    }
}

fn generate_func_info_struct() -> TokenStream2 {
    quote! {
        /// Metadata about a function, used for LSP and documentation.
        #[derive(Debug, Clone)]
        pub struct FuncInfo {
            /// The Rust enum variant name (e.g., "Arcsin")
            pub variant_name: &'static str,
            /// All string names that map to this function
            pub names: &'static [&'static str],
            /// Arity specification
            pub arity: Arguments,
            /// Documentation string
            pub doc: &'static str,
            /// Optional MathML symbol override
            pub symbol: Option<&'static str>,
        }

        impl FuncInfo {
            /// Returns the canonical (first) name
            pub fn canonical_name(&self) -> &'static str {
                self.names.first().copied().unwrap_or(self.variant_name)
            }

            /// Returns a formatted arity string for display
            pub fn arity_display(&self) -> String {
                match &self.arity {
                    Arguments::Fixed(n) => format!("{} argument{}", n, if *n == 1 { "" } else { "s" }),
                    Arguments::Range(a, b) => format!("{}-{} arguments", a, b),
                    Arguments::Atleast(n) => format!("at least {} argument{}", n, if *n == 1 { "" } else { "s" }),
                    Arguments::Variadic => "any number of arguments".to_string(),
                }
            }

            /// Generate LSP-style markdown documentation
            pub fn to_markdown(&self) -> String {
                let mut md = String::new();
                md.push_str(&format!("### `{}`\n\n", self.canonical_name()));

                if !self.doc.is_empty() {
                    md.push_str(&format!("{}\n\n", self.doc));
                }

                md.push_str(&format!("**Arity:** {}\n\n", self.arity_display()));

                if self.names.len() > 1 {
                    md.push_str("**Aliases:** ");
                    let aliases: Vec<_> = self.names.iter().skip(1).map(|s| format!("`{}`", s)).collect();
                    md.push_str(&aliases.join(", "));
                    md.push('\n');
                }

                md
            }
        }
    }
}
