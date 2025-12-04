extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Meta, Lit};

#[proc_macro_attribute]
pub fn slint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    // -------- table_name parsing --------
    let mut table_name = struct_name.to_string().to_lowercase();
    if !attr.is_empty() {
        let meta = parse_macro_input!(attr as Meta);
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("table_name") {
                if let syn::Expr::Lit(expr_lit) = nv.value {
                    if let Lit::Str(litstr) = expr_lit.lit {
                        table_name = litstr.value();
                    }
                }
            }
        }
    }

    // -------- fields --------
    let fields = match &input.data {
        syn::Data::Struct(s) => s.fields.iter().collect::<Vec<_>>(),
        _ => vec![],
    };

    let mut cols = Vec::new();

    for f in fields {
    let col_name = f.ident.as_ref().unwrap().to_string();
    let sql_type = "TEXT";
    let mut primary = false;
    let mut unique = false;
    let mut uuid = false;
    let mut not_null = true;

    // Optional extra fields
    let mut default: Option<String> = None;
    let mut foreign_key: Option<String> = None;
    let mut relationship: Option<String> = None;

    for attr in &f.attrs {

        // Existing attribute parsing
        if attr.path().is_ident("slint") {
            let _ = attr.parse_nested_meta(|meta| {
                let ident = meta.path.get_ident().map(|i| i.to_string());

                match ident.as_deref() {
                    Some("uuid") => { uuid = true; primary = true; }
                    Some("primary") => primary = true,
                    Some("unique") => unique = true,
                    Some("not_null") => not_null = true,
                    Some("default") => {
                        if let Ok(lit) = meta.value()?.parse::<Lit>() {
                            if let Lit::Str(litstr) = lit {
                                default = Some(litstr.value());
                            }
                        }
                    }
                    Some("foreign_key") => {
                        if let Ok(lit) = meta.value()?.parse::<Lit>() {
                            if let Lit::Str(litstr) = lit {
                                foreign_key = Some(litstr.value());
                            }
                        }
                    }
                    Some("relationship") => {
                        if let Ok(lit) = meta.value()?.parse::<Lit>() {
                            if let Lit::Str(litstr) = lit {
                                relationship = Some(litstr.value());
                            }
                        }
                    }
                    _ => {}
                }

                Ok(())
            });
        }

        // New: field-level macro
        if attr.path().is_ident("slint_internal_field") {
            attr.parse_nested_meta(|meta| {
                let ident = meta.path.get_ident().unwrap().to_string();

                match ident.as_str() {
                    "primary_key" => primary = true,
                    "uuid" => { uuid = true; primary = true; }
                    "unique" => unique = true,
                    "not_null" => not_null = true,
                    _ => {}
                };

                Ok(())
            }).unwrap();
        }
    }

    // Build ColumnSchema
    cols.push(quote! {
        ColumnSchema {
            name: #col_name,
            sql_type: #sql_type,
            primary: #primary,
            unique: #unique,
            not_null: #not_null,
            uuid: #uuid,
        }
    });
}


    // -------- generate output --------
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







#[proc_macro_attribute]
pub fn slint_field(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = attr.to_string();
    let field = item.to_string();

    // Rewrite into a normal attribute
    let out = format!(
        "#[slint_internal_field({})]\n{}",
        args,
        field
    );

    out.parse().unwrap()
}

