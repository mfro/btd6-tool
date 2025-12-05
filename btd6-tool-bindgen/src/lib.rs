use core::panic;

use anyhow::Context;
use assert_matches::assert_matches;
use proc_macro::TokenStream;
use proc_macro2::Literal;
use quote::quote;
use regex::Regex;
use syn::{punctuated::Punctuated, Expr, Lit, Meta};

type Result<T> = anyhow::Result<T>;

const DUMP: &'static str = include_str!("../dump.cs");

struct BindingGenerator {
    dump: String,
}

impl BindingGenerator {
    fn new(dump: String) -> Self {
        Self { dump }
    }

    fn load() -> Self {
        Self::new(DUMP.to_string())
    }

    fn get_class(&'_ self, namespace: &str, name: &str) -> Result<ClassBindingGenerator<'_>> {
        let regex = Regex::new(&format!(
            r"// Namespace: {}\n(\[.*\n)*.*class {} .*\n\{{\n((\n|\s+.+\n)*)\}}",
            regex::escape(&namespace),
            regex::escape(&name),
        ))?;

        let captures = regex.captures(&self.dump).context("class not found")?;
        let body = captures.get(2).unwrap().as_str();

        Ok(ClassBindingGenerator { body })
    }
}

struct ClassBindingGenerator<'a> {
    body: &'a str,
}

impl<'a> ClassBindingGenerator<'a> {
    fn get_field_offset(&self, field_name: &str) -> Result<usize> {
        let regex = Regex::new(&format!(
            r"\s([^\s]+) {}; // 0x([A-Z\d]+)\n",
            regex::escape(field_name),
        ))?;

        let captures = regex
            .captures(&self.body)
            .context(format!("field not found: {}", field_name))?;
        let raw_value = captures.get(2).unwrap().as_str();

        let value = usize::from_str_radix(raw_value, 16)?;

        // offsets from il2cpp include the constant-sized class offset of 16 bytes
        Ok(value - 16)
    }
}

#[proc_macro_attribute]
pub fn class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let arguments =
        syn::parse_macro_input!(attr with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    let mut namespace = None;
    let mut base = None;
    let mut rename = None;

    for attr in arguments.iter() {
        match attr {
            Meta::NameValue(value) => {
                assert_eq!(value.path.segments.len(), 1);
                match value.path.segments[0].ident.to_string().as_ref() {
                    "namespace" => {
                        let value = assert_matches!(&value.value, Expr::Lit(v) => v);
                        let literal = assert_matches!(&value.lit, Lit::Str(v) => v);
                        namespace = Some(literal.value());
                    }

                    "base" => {
                        let value = assert_matches!(&value.value, Expr::Path(v) => v);
                        assert_eq!(value.path.segments.len(), 1);
                        base = Some(&value.path.segments[0].ident);
                    }

                    "rename" => {
                        let value = assert_matches!(&value.value, Expr::Lit(v) => v);
                        let literal = assert_matches!(&value.lit, Lit::Str(v) => v);
                        rename = Some(literal.value());
                    }

                    v => panic!("unknown argument: {v}"),
                }
            }

            _ => panic!("invalid arguments"),
        }
    }

    let namespace = namespace.expect("Namespace not specified");

    let item = syn::parse_macro_input!(item as syn::ItemStruct);

    let name = item.ident;

    let csharp_full_name = rename.unwrap_or(name.to_string());
    let csharp_base_name = csharp_full_name
        .split(".")
        .last()
        .expect("invalid identifier");

    let bindgen = BindingGenerator::load();
    let class = bindgen.get_class(&namespace, &csharp_full_name).unwrap();

    let fields = item.fields.iter().map(|field| {
        let name = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        let rename_arg = assert_matches!(&field.attrs[0].meta, Meta::NameValue(v) => v);

        assert_eq!("rename", rename_arg.path.segments[0].ident.to_string());
        let rename_value = assert_matches!(&rename_arg.value, Expr::Lit(v) => v);
        let rename_value = assert_matches!(&rename_value.lit, Lit::Str(v) => v);
        let rename = rename_value.value();

        let offset = class.get_field_offset(&rename).unwrap();
        let offset = Literal::usize_unsuffixed(offset);

        quote! {
            field!(#offset #name: #ty);
        }
    });

    let inheritence = base.map(|base| {
        quote! {
            impl std::ops::Deref for #name {
                type Target = #base;

                fn deref(&self) -> &Self::Target {
                    unsafe { std::mem::transmute(self) }
                }
            }
        }
    });

    let csharp_name = Literal::string(csharp_base_name);

    let output: proc_macro2::TokenStream = quote! {
        object_type!(#name ; #csharp_name);

        impl #name {
            #( #fields )*
        }

        #inheritence
    };

    output.into()
}
