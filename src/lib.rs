// Copyright © 2022 René Kijewski <crates.io@k6i.de>
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
// REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
// AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
// INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
// LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
// OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
// PERFORMANCE OF THIS SOFTWARE.

#![forbid(unsafe_code)]
#![deny(elided_lifetimes_in_paths)]
#![deny(unreachable_pub)]

//! ## askama-enum
//!
//! [![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Kijewski/askama-enum/CI?logo=github)](https://github.com/Kijewski/askama-enum/actions/workflows/ci.yml)
//! [![Crates.io](https://img.shields.io/crates/v/askama-enum?logo=rust)](https://crates.io/crates/askama-enum)
//! ![Minimum supported Rust version](https://img.shields.io/badge/rustc-1.53+-important?logo=rust "Minimum Supported Rust Version")
//! ![License](https://img.shields.io/badge/license-ISC%2FMIT%2FApache--2.0%20WITH%20LLVM--exception-informational?logo=apache)
//!
//! Implement different [Askama](https://crates.io/crates/askama) templates for different enum variants.
//!
//! You can add a default `#[template]` for variants that don't have a specific `#[template]` attribute.
//! If omitted, then every variant needs its own `#[template]` attribute.
//! The `#[template]` attribute is not interpreted, but simply copied to be used by askama.
//!
//! ```rust
//! # #[cfg(feature = "askama")] fn main() {
//! # use askama_enum::EnumTemplate;
//! #[derive(EnumTemplate)]
//! #[template(ext = "html", source = "default")] // default, optional
//! enum MyEnum<'a, T: std::fmt::Display> {
//!     // uses the default `#[template]`
//!     A,
//!
//!     // uses specific `#[template]`
//!     #[template(ext = "html", source = "B")]
//!     B,
//!
//!     // you can use tuple structs
//!     #[template(
//!         ext = "html",
//!         source = "{{self.0}} {{self.1}} {{self.2}} {{self.3}}",
//!     )]
//!     C(u8, &'a u16, u32, &'a u64),
//!
//!     // and named fields, too
//!     #[template(ext = "html", source = "{{some}} {{fields}}")]
//!     D { some: T, fields: T },
//! }
//!
//! assert_eq!(
//!     MyEnum::A::<&str>.to_string(),
//!     "default",
//! );
//! assert_eq!(
//!     MyEnum::B::<&str>.to_string(),
//!     "B",
//! );
//! assert_eq!(
//!     MyEnum::C::<&str>(1, &2, 3, &4).to_string(),
//!     "1 2 3 4",
//! );
//! assert_eq!(
//!     MyEnum::D { some: "some", fields: "fields" }.to_string(),
//!     "some fields",
//! );
//! # }
//! ```
//!

use std::iter::FromIterator;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_quote, DeriveInput, Token};

/// Implement different Askama templates for different enum variants
///
/// Please see the [crate] documentation for more examples.
#[proc_macro_derive(EnumTemplate, attributes(template))]
pub fn derive_enum_template(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let data = match &ast.data {
        syn::Data::Enum(data) => data,
        syn::Data::Struct(data) => {
            return fail_at(
                data.struct_token,
                "#[derive(EnumTemplate)] can only be used with enums",
            );
        }
        syn::Data::Union(data) => {
            return fail_at(
                data.union_token,
                "#[derive(EnumTemplate)] can only be used with enums",
            );
        }
    };

    let mut global_meta = None;
    for attr in &ast.attrs {
        let meta_list = match attr.parse_meta() {
            Ok(syn::Meta::List(attr)) => attr,
            _ => continue,
        };
        if meta_list.path.is_ident("template") {
            if global_meta.is_some() {
                return fail_at(
                    meta_list.path,
                    "cannot have more than one #[template] attribute for a type",
                );
            }
            global_meta = Some(attr);
        }
    }

    let mut default_variant_name = None;
    let variant_definitions =
        make_variant_definitions(global_meta, &ast, data, &mut default_variant_name);
    let variant_definitions = match variant_definitions {
        Ok(variant_definitions) => variant_definitions,
        Err(err) => return err,
    };
    let match_render_impl = make_render_impl(&ast, data, "render", Punctuated::new());
    let match_render_into_impl = make_render_impl(
        &ast,
        data,
        "render_into",
        Punctuated::from_iter([syn::Expr::Path(parse_quote!(writer))]),
    );
    let dflt_or_fst_variant_name =
        default_variant_name.unwrap_or_else(|| variant_definitions[0].ident.clone());

    let mut static_ty_generics = quote!(::<);
    for g in ast.generics.params.iter() {
        match g {
            syn::GenericParam::Type(param) => {
                param.ident.to_tokens(&mut static_ty_generics);
            }
            syn::GenericParam::Const(param) => {
                param.ident.to_tokens(&mut static_ty_generics);
            }
            _ => (),
        }
    }
    static_ty_generics.extend(quote!(>));

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let enum_name = &ast.ident;
    let mut result = quote! {
        impl #impl_generics askama::Template for #enum_name #ty_generics #where_clause {
            fn render(&self) -> askama::Result<::std::string::String> {
                #match_render_impl
            }

            fn render_into(
                &self,
                writer: &mut (impl ::std::fmt::Write + ?::std::marker::Sized),
            ) -> askama::Result<()> {
                #match_render_into_impl
            }

            const EXTENSION: ::std::option::Option<&'static str> =
                <#dflt_or_fst_variant_name #static_ty_generics as askama::Template>::EXTENSION;
            const SIZE_HINT: ::std::primitive::usize =
                <#dflt_or_fst_variant_name #static_ty_generics as askama::Template>::SIZE_HINT;
            const MIME_TYPE: &'static ::std::primitive::str =
                <#dflt_or_fst_variant_name #static_ty_generics as askama::Template>::MIME_TYPE;
        }
    };
    for variant_definition in variant_definitions {
        variant_definition.to_tokens(&mut result);
    }
    let result = quote! {
        #[allow(non_camel_case_types, non_snake_case, unused_qualifications)]
        const _: () = {
            #result

            impl #impl_generics ::std::fmt::Display for #enum_name #ty_generics #where_clause {
                #[inline]
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    askama::Template::render_into(self, f).map_err(|_| ::std::fmt::Error {})
                }
            }
        };
    };
    result.into()
}

fn make_render_impl(
    ast: &DeriveInput,
    data: &syn::DataEnum,
    meth_name: &'static str,
    args: Punctuated<syn::Expr, syn::token::Comma>,
) -> syn::ExprMatch {
    let mut generics = ast.generics.clone();
    generics.params.push(parse_quote!('_));
    let (_, inst_ty_generics, _) = generics.split_for_impl();
    let inst_ty_generics = inst_ty_generics.as_turbofish();

    let match_render_impl = data
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let self_variant_name = &variant.ident;

            let variant_name = &format!("_{}_{}_{}", &ast.ident, index, variant.ident);
            let variant_span = variant.ident.span();
            let variant_name = syn::Ident::new(variant_name, variant_span);

            let (pat, base) = match &variant.fields {
                syn::Fields::Named(fields) => {
                    let tmp_names = fields
                        .named
                        .iter()
                        .enumerate()
                        .map(|(index, field)| syn::Ident::new(&format!("_{}", index), field.span()))
                        .collect::<Vec<_>>();

                    let source_elems = tmp_names
                        .iter()
                        .zip(fields.named.iter())
                        .map(|(dest, source)| syn::FieldPat {
                            attrs: vec![],
                            member: syn::Member::Named(source.ident.clone().unwrap()),
                            colon_token: Some(Token![:](variant_span)),
                            pat: parse_quote!(#dest),
                        })
                        .collect();
                    let pat = syn::Pat::Struct(syn::PatStruct {
                        attrs: vec![],
                        path: parse_quote!(Self::#self_variant_name),
                        brace_token: syn::token::Brace(variant_span),
                        fields: source_elems,
                        dot2_token: None,
                    });

                    let mut fields = tmp_names
                        .iter()
                        .zip(fields.named.iter())
                        .map(|(tmp, source)| syn::FieldValue {
                            attrs: vec![],
                            member: syn::Member::Named(source.ident.clone().unwrap()),
                            colon_token: Some(Token![:](variant_span)),
                            expr: parse_quote!(#tmp),
                        })
                        .collect::<Punctuated<syn::FieldValue, Token![,]>>();
                    fields.push(parse_quote!(#variant_name: ::std::marker::PhantomData));
                    let base = syn::Expr::Struct(syn::ExprStruct {
                        attrs: vec![],
                        path: parse_quote!(#variant_name #inst_ty_generics),
                        brace_token: syn::token::Brace(variant_span),
                        fields,
                        dot2_token: None,
                        rest: None,
                    });

                    (pat, base)
                }
                syn::Fields::Unnamed(fields) => {
                    let tmp_names = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(index, field)| syn::Ident::new(&format!("_{}", index), field.span()))
                        .collect::<Vec<_>>();

                    let source_elems = tmp_names
                        .iter()
                        .map(|ident| {
                            syn::Pat::Ident(syn::PatIdent {
                                attrs: vec![],
                                by_ref: None,
                                mutability: None,
                                ident: ident.clone(),
                                subpat: None,
                            })
                        })
                        .collect();
                    let pat = syn::Pat::TupleStruct(syn::PatTupleStruct {
                        attrs: vec![],
                        path: parse_quote!(Self::#self_variant_name),
                        pat: syn::PatTuple {
                            attrs: vec![],
                            paren_token: syn::token::Paren(variant_span),
                            elems: source_elems,
                        },
                    });

                    let mut args = tmp_names
                        .iter()
                        .map(|field_name| {
                            let expr: syn::Expr = parse_quote!(#field_name);
                            expr
                        })
                        .collect::<Punctuated<syn::Expr, Token![,]>>();
                    args.push(parse_quote!(::std::marker::PhantomData));
                    let base = syn::Expr::Call(syn::ExprCall {
                        attrs: vec![],
                        func: parse_quote!(#variant_name #inst_ty_generics),
                        paren_token: syn::token::Paren(variant_span),
                        args,
                    });

                    (pat, base)
                }
                syn::Fields::Unit => {
                    let pat = parse_quote!(Self :: #self_variant_name);
                    let base =
                        parse_quote!(#variant_name #inst_ty_generics(::std::marker::PhantomData));
                    (pat, base)
                }
            };
            let field = syn::Expr::Field(syn::ExprField {
                attrs: vec![],
                base: Box::new(base),
                dot_token: Token![.](variant_span),
                member: syn::Member::Named(syn::Ident::new(meth_name, variant_span)),
            });
            let call = syn::Expr::Call(syn::ExprCall {
                attrs: vec![],
                func: field.into(),
                paren_token: syn::token::Paren(variant_span),
                args: args.clone(),
            });
            syn::Arm {
                attrs: vec![],
                pat,
                guard: None,
                fat_arrow_token: Token![=>](variant_span),
                body: call.into(),
                comma: Some(Token![,](variant_span)),
            }
        })
        .collect();
    syn::ExprMatch {
        attrs: vec![],
        match_token: Token![match](data.brace_token.span),
        expr: parse_quote!(self),
        brace_token: syn::token::Brace(data.brace_token.span),
        arms: match_render_impl,
    }
}

fn make_variant_definitions(
    global_meta: Option<&syn::Attribute>,
    ast: &DeriveInput,
    data: &syn::DataEnum,
    default_variant_name: &mut Option<syn::Ident>,
) -> Result<Vec<syn::DeriveInput>, TokenStream> {
    data.variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let variant_name = &format!("_{}_{}_{}", &ast.ident, index, variant.ident);
            let variant_span = variant.ident.span();
            let variant_lifetime = syn::Lifetime::new(&format!("'{}", variant_name), variant_span);
            let variant_name = syn::Ident::new(variant_name, variant_span);

            let mut local_meta = None;
            for attr in &variant.attrs {
                let meta_list = match attr.parse_meta() {
                    Ok(syn::Meta::List(attr)) => attr,
                    _ => continue,
                };
                if meta_list.path.is_ident("template") {
                    if local_meta.is_some() {
                        return Err(fail_at(
                            meta_list.path,
                            "cannot have more than one #[template] attribute for a variant",
                        ));
                    }
                    local_meta = Some(attr);
                }
            }
            if local_meta.is_none() && default_variant_name.is_none() {
                *default_variant_name = Some(variant_name.clone());
            }
            let meta = match local_meta.or(global_meta) {
                Some(meta) => meta,
                None => return Err(fail_at(&variant.ident, "need a #[template] attribute")),
            };

            let (_, ty_generics, _) = ast.generics.split_for_impl();
            let enum_name = &ast.ident;
            let phantom_type = parse_quote!(::std::marker::PhantomData::<
                & #variant_lifetime #enum_name #ty_generics,
            >);
            let fields = match &variant.fields {
                syn::Fields::Named(fields) => {
                    let mut fields = fields
                        .named
                        .iter()
                        .map(|field| {
                            let mut field = field.clone();
                            field.ty = syn::Type::Reference(syn::TypeReference {
                                and_token: Token![&](field.span()),
                                lifetime: Some(variant_lifetime.clone()),
                                mutability: None,
                                elem: field.ty.into(),
                            });
                            field
                        })
                        .collect::<Vec<syn::Field>>();
                    fields.push(syn::Field {
                        attrs: vec![],
                        vis: syn::Visibility::Inherited,
                        ident: Some(variant_name.clone()),
                        colon_token: Some(Token![:](variant_span)),
                        ty: phantom_type,
                    });
                    syn::Fields::Named(syn::FieldsNamed {
                        brace_token: syn::token::Brace(variant_span),
                        named: Punctuated::from_iter(fields),
                    })
                }
                syn::Fields::Unnamed(fields) => {
                    let mut fields = fields
                        .unnamed
                        .iter()
                        .map(|field| {
                            let mut field = field.clone();
                            field.ty = syn::Type::Reference(syn::TypeReference {
                                and_token: Token![&](field.span()),
                                lifetime: Some(variant_lifetime.clone()),
                                mutability: None,
                                elem: field.ty.into(),
                            });
                            field
                        })
                        .collect::<Vec<syn::Field>>();
                    fields.push(syn::Field {
                        attrs: vec![],
                        vis: syn::Visibility::Inherited,
                        ident: None,
                        colon_token: None,
                        ty: phantom_type,
                    });
                    syn::Fields::Unnamed(syn::FieldsUnnamed {
                        paren_token: syn::token::Paren(variant_span),
                        unnamed: Punctuated::from_iter(fields),
                    })
                }
                syn::Fields::Unit => syn::Fields::Unnamed(syn::FieldsUnnamed {
                    paren_token: syn::token::Paren(variant_span),
                    unnamed: Punctuated::from_iter([syn::Field {
                        attrs: vec![],
                        vis: syn::Visibility::Inherited,
                        ident: None,
                        colon_token: None,
                        ty: phantom_type,
                    }]),
                }),
            };

            let mut generics = ast.generics.clone();
            generics.params.push(parse_quote!(#variant_lifetime));
            Ok(syn::DeriveInput {
                attrs: vec![
                    parse_quote!(#[::std::prelude::v1::derive(
                        askama::Template,
                        ::std::prelude::v1::Clone,
                        ::std::prelude::v1::Copy,
                        ::std::prelude::v1::Debug,
                    )]),
                    meta.clone(),
                ],
                vis: syn::Visibility::Inherited,
                ident: variant_name,
                generics,
                data: syn::Data::Struct(syn::DataStruct {
                    struct_token: Token![struct](variant_span),
                    fields,
                    semi_token: None,
                }),
            })
        })
        .collect()
}

fn fail_at(spanned: impl Spanned, msg: &str) -> TokenStream {
    syn::Error::new(spanned.span(), msg)
        .into_compile_error()
        .into()
}
