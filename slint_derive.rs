// lib.rs in slint_derive
extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Meta, NestedMeta};

#[proc_macro_attribute]
pub fn slint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    let mut table_name = struct_name.to_string().to_lowercase(); // default table name
    if !attr.is_empty() {
        let meta = parse_macro_input!(attr as Meta);
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("table_name") {
                if let syn::Lit::Str(litstr) = nv.lit {
                    table_name = litstr.value();
                }
            }
        }
    }

    let fields = if let syn::Data::Struct(s) = &input.data {
        s.fields.iter().collect::<Vec<_>>()
    } else { vec![] };

    let mut cols = Vec::new();
    for f in fields {
        let mut col_name = f.ident.as_ref().unwrap().to_string();
        let mut sql_type = "TEXT";
        let mut primary = false;
        let mut unique = false;
        let mut uuid = false;

        for attr in &f.attrs {
            if attr.path.is_ident("slint") {
                let meta = attr.parse_meta().unwrap();
                if let Meta::List(list) = meta {
                    for nm in list.nested.iter() {
                        if let NestedMeta::Meta(Meta::Path(p)) = nm {
                            if p.is_ident("uuid") { uuid = true; primary = true; }
                            if p.is_ident("unique") { unique = true; }
                        }
                    }
                }
            }
        }
        cols.push(quote! {
            ColumnSchema { name: #col_name, sql_type: #sql_type, primary: #primary, unique: #unique, not_null: true, uuid: #uuid }
        });
    }

    let expanded = quote! {
        #input

        impl #struct_name {
            pub fn slint_schema() -> TableSchema {
                TableSchema {
                    name: #table_name,
                    columns: &[#(#cols),*],
                }
            }
        }
    };

    TokenStream::from(expanded)
}
