extern crate proc_macro;

use std::str::FromStr;

use proc_macro2::Span;
use quote::quote;
use syn::{
    Expr,
    Ident,
    ItemFn,
    ItemImpl,
    ItemTrait,
    Meta,
    NestedMeta,
    parse_macro_input,
    parse_quote,
    spanned::Spanned,
    TraitItemMethod,
};

struct FeignAttr {
    path: String,
    id: String,
    port: Option<u16>,
}

fn extract_feign_attr<'a>(metas: impl Iterator<Item=&'a NestedMeta>) -> FeignAttr {
    let mut path = None;
    let mut id = None;
    let mut port = None;
    for meta in metas {
        match meta {
            NestedMeta::Literal(name) => {
                if path.is_some() {
                    panic!("Multiple paths specified! Should be only one!")
                }
                let fname = quote!(#name).to_string();
                path = Some(fname.as_str()[1..fname.len() - 1].to_owned());
            }
            NestedMeta::Meta(syn::Meta::NameValue(ident)) => {
                match ident.ident.to_string().to_lowercase().as_str() {
                    "id" => match ident.lit {
                        syn::Lit::Str(ref text) => {
                            id = Some(text.value());
                        }
                        _ => panic!("Attribute guard expects literal string!"),
                    },
                    "port" => match ident.lit {
                        syn::Lit::Int(ref int) => {
                            port = Some(int.value() as u16);
                        }
                        _ => panic!("Attribute guard expects literal string!"),
                    },
                    attr => panic!(
                        "Unknown attribute key is specified: {}. Allowed: id, port",
                        attr
                    ),
                }
            }
            attr => panic!("Unknown attribute{:?}", attr)
        }
    }
    let path = path.expect("no path for feign");
    let id = id.expect("no id for feign");
    FeignAttr {
        path,
        id,
        port,
    }
}

enum ReturnType {
    Void,
    Json,
    String,
}

struct MappingAttr {
    method: Expr,
    path: String,
    json: Option<Ident>,
    ret: ReturnType,
}

fn extract_mapping_attr<'a>(method: &Ident, metas: impl Iterator<Item=&'a NestedMeta>, sp: Span) -> MappingAttr {
    let method = reqwest::Method::from_str(&method.to_string().to_uppercase())
        .expect("invalid method");
    let method = format!("reqwest::Method::{}", &method.to_string());
    let method: syn::Expr = syn::parse_str(&method).expect("invalid method");

    let mut path = None;
    let mut json = None;
    let mut ret = None;
    for meta in metas {
        match meta {
            NestedMeta::Literal(name) => {
                if path.is_some() {
                    panic!("Multiple paths specified! Should be only one!")
                }
                let fname = quote!(#name).to_string();
                path = Some(fname.as_str()[1..fname.len() - 1].to_owned());
            }
            NestedMeta::Meta(syn::Meta::NameValue(ident)) => {
                match ident.ident.to_string().to_lowercase().as_str() {
                    "json" => match ident.lit {
                        syn::Lit::Str(ref text) => {
                            json = Some(Ident::new(&text.value(), sp));
                        }
                        _ => panic!("Attribute guard expects literal string!"),
                    },
                    "ret" => match ident.lit {
                        syn::Lit::Str(ref text) => {
                            let x: &str = &text.value();
                            ret = Some(match x {
                                "void" => ReturnType::Void,
                                "json" => ReturnType::Json,
                                "str" => ReturnType::String,
                                other => panic!("Attribute ret is specified: {}. Allowed: void json str", other),
                            });
                        }
                        _ => panic!("Attribute guard expects literal string!"),
                    },
                    attr => panic!(
                        "Unknown attribute key is specified: {}. Allowed: json, ret",
                        attr
                    ),
                }
            }
            attr => panic!("Unknown attribute{:?}", attr)
        }
    }
    let path = path.expect("no path for mapping");
    MappingAttr {
        method,
        path,
        json,
        ret: ret.unwrap_or(ReturnType::Void),
    }
}

fn extract_mapping(item_method: &TraitItemMethod) -> MappingAttr {
    for attr in &item_method.attrs {
        if let Ok(meta) = attr.parse_meta() {
            match meta {
                Meta::List(list) => {
                    return extract_mapping_attr(&list.ident,
                                                list.nested.iter(),
                                                item_method.span());
                }
                Meta::Word(word) => {
                    panic!("word: {}", quote!(#word));
                }
                Meta::NameValue(nv) => {
                    panic!("nv: {}", quote!(#nv));
                }
            }
        } else {
            panic!("parse mapping failed");
        }
    }
    panic!("no attr for mapping");
}

fn impl_build_method(name: &Ident, id: &str) -> ItemFn {
    let build_name = format!("build_{}", name).to_lowercase();
    let build_name = syn::Ident::new(&build_name, name.span());


    let host = format!("{}.service.consul", id).to_lowercase();

    let build_method: syn::ItemFn = parse_quote! {
        fn #build_name(socket_addr: ::std::net::SocketAddr) -> Result<::feign::FeignClient, ::failure::Error>{
            ::feign::FeignClient::builder(socket_addr)?
                .build(#host)
        }
    };
    build_method
}

fn impl_trait(trait_ast: &ItemTrait) -> ItemImpl {
    let name = &trait_ast.ident;
    let generics = trait_ast.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let item_impl: syn::ItemImpl = parse_quote! {
        impl #impl_generics #name #ty_generics for ::feign::FeignClient #where_clause {
        }
    };
    item_impl
}

#[proc_macro_attribute]
pub fn put(_args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input
}

#[proc_macro_attribute]
pub fn get(_args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input
}

#[proc_macro_attribute]
pub fn post(_args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    input
}

#[proc_macro_attribute]
pub fn feign(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {

    // Parse the input tokens into a syntax tree.
    let ast = parse_macro_input!(args as syn::AttributeArgs);

    let feign = extract_feign_attr(ast.iter());

    let trait_ast: ItemTrait = parse_macro_input!(input as syn::ItemTrait);

    let name = &trait_ast.ident;

    let mut item_impl = impl_trait(&trait_ast);
    for item in &trait_ast.items {
        if let syn::TraitItem::Method(item_method) = item {
            if item_method.default.is_none() {
                let sig = &item_method.sig;
                let mapping = extract_mapping(item_method);
                let block = impl_call(&feign, &mapping);
                let impl_item_method: syn::ImplItem = parse_quote! { #sig #block };
                item_impl.items.push(impl_item_method);
            }
        }
    }

    let build_method = impl_build_method(name, &feign.id);

    let output = quote! {
        #trait_ast

        #build_method

        #item_impl
    };

    // Hand the output tokens back to the compiler.
    output.into()
}


fn impl_call(feign: &FeignAttr, mapping: &MappingAttr) -> syn::Block {
    let template: &str = &format!("http://{{}}:{{}}/{}{}", &feign.path, &mapping.path);
    let method = &mapping.method;

    let url: syn::Expr = if let Some(port) = feign.port {
        parse_quote! {
            {
                let (ip, _) = self.next_addr()?;
                format!(#template, ip, #port)
            }
        }
    } else {
        parse_quote! {
            {
                let (ip, port) = self.next_addr()?;
                format!(#template, ip, port)
            }
        }
    };

    let request: syn::Expr = parse_quote! {
        {
            let url = #url;
            self.client.request(#method, &url)
        }
    };

    let call: syn::Block = if let Some(ref json) = mapping.json {
        parse_quote! {
            {
                #request.json(#json).send()?
            }
        }
    } else {
        parse_quote! {
            {
                #request.send()?
            }
        }
    };

    match mapping.ret {
        ReturnType::Void => {
            parse_quote! {
                {
                    let text = #call.text()?;
                    println!("result: {}", &text);
                    Ok(())

                }
            }
        }
        ReturnType::Json => {
            parse_quote! {
                Ok(#call.json()?)
            }
        }
        ReturnType::String => {
            parse_quote! {
                Ok(#call.text()?)
            }
        }
    }
}