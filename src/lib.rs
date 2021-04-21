// Copyright 2021 Gregory Oakes
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![allow(dead_code)]

use core::iter::repeat;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token, Attribute, Field, Ident, Lit, LitInt, Meta, NestedMeta, Token, Visibility,
};

struct Item {
    attrs: Vec<Attribute>,
    vis: Visibility,
    struct_token: Token![struct],
    ident: Ident,
    brace_token: token::Brace,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![struct]) {
            let content;
            Ok(Item {
                attrs,
                vis,
                struct_token: input.parse()?,
                ident: input.parse()?,
                brace_token: braced!(content in input),
                fields: content.parse_terminated(Field::parse_named)?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

struct Assertion {
    start: LitInt,
    colon: token::Colon,
    end: LitInt,
}

impl Parse for Assertion {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Assertion {
            start: input.parse()?,
            colon: input.parse()?,
            end: input.parse()?,
        })
    }
}

#[proc_macro_attribute]
pub fn test_structure(attrs: TokenStream, tokens: TokenStream) -> TokenStream {
    let Item {
        attrs: struct_attrs,
        vis,
        ident,
        mut fields,
        ..
    } = parse_macro_input!(tokens as Item);
    let meta = parse_macro_input!(attrs as NestedMeta);
    let size = match meta {
        NestedMeta::Meta(Meta::NameValue(nv)) if nv.path.is_ident("size") => match &nv.lit {
            Lit::Int(tok) => quote! {
                assert_eq!(#tok, ::core::mem::size_of::<#ident>(), "size of {}", ::core::stringify!(#ident));
            },
            tok => syn::Error::new(tok.span(), "Unexpected size type").to_compile_error(),
        },
        m => syn::Error::new(m.span(), "Unexpected meta item").to_compile_error(),
    };
    let assertions = fields
        .iter()
        .flat_map(|field| {
            field
                .attrs
                .iter()
                .filter(|attr| attr.path.is_ident("loc"))
                .zip(repeat(field))
        })
        .fold(quote! {}, |acc, (attr, field)| {
            let assertion = if let Some(field_ident) = &field.ident {
                let typ = &field.ty;
                match attr.parse_args() {
                    Ok(Assertion { start, end, .. }) => quote! {
                        {
                            let desc = format!(
                                "{}.{} ({})",
                                ::core::stringify!(#ident),
                                ::core::stringify!(#field_ident),
                                ::core::stringify!(#typ),
                            );
                            let offset = ::memoffset::offset_of!(#ident, #field_ident);
                            assert_eq!(#start, offset, "start of {}", desc);
                            assert_eq!(#end, ::core::mem::size_of::<#typ>() + offset - 1, "end of {}", desc);
                        }
                    },
                    Err(e) => e.to_compile_error(),
                }
            } else {
                syn::Error::new(field.span(), "Tuple structs are not supported").to_compile_error()
            };
            quote! { #acc #assertion; }
        });
    let func_name = Ident::new(format!("structure_{}", ident).as_str(), ident.span());
    for field in fields.iter_mut() {
        field.attrs.retain(|attr| !attr.path.is_ident("loc"));
    }
    let struct_attrs_stream = struct_attrs
        .iter()
        .fold(quote! {}, |acc, attr| quote! { #acc #attr });
    let output = quote! {
        #struct_attrs_stream
        #vis struct #ident {
            #fields
        }

        #[cfg(test)]
        #[allow(non_snake_case)]
        #[test]
        fn #func_name() {
            #assertions
            #size
        }
    };
    output.into()
}
