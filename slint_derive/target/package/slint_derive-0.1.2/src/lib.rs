extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Meta, Lit, Expr, token, parse::ParseBuffer};

#[proc_macro_attribute]
pub fn slint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    // -------- table_name parsing --------
    let mut table_name = struct_name.to_string().to_lowercase();
    if !attr.is_empty() {
        let meta = parse_macro_input!(attr as Meta);
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("table_name") {
                if let Expr::Lit(expr_lit) = nv.value {
                    if let Lit::Str(litstr) = expr_lit.lit {
                        table_name = litstr.value();
                    }
                }
            }
        }
    }

    // Remove the #[slint] attribute from the struct
    input.attrs.retain(|attr| !attr.path().is_ident("slint"));

    // -------- fields --------
    let fields = match &input.data {
        syn::Data::Struct(s) => s.fields.iter().collect::<Vec<_>>(),
        _ => vec![],
    };

    let mut cols = Vec::new();

    for f in fields {
    let col_name = f.ident.as_ref().unwrap().to_string();
    let sql_type = "TEXT".to_string();
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
            let metas = attr.parse_args_with(|input: &ParseBuffer| {
                let mut metas = Vec::new();
                while !input.is_empty() {
                    metas.push(input.parse::<Meta>()?);
                    if !input.is_empty() {
                        input.parse::<token::Comma>()?;
                    }
                }
                Ok(metas)
            }).unwrap();
            for meta in metas {
                match meta {
                    Meta::NameValue(nv) => {
                        let ident = nv.path.get_ident().unwrap().to_string();
                        match ident.as_str() {
                            "default" => {
                                if let Expr::Lit(expr_lit) = nv.value {
                                    if let Lit::Str(litstr) = expr_lit.lit {
                                        default = Some(litstr.value());
                                    }
                                }
                            }
                            "foreign_key" => {
                                if let Expr::Lit(expr_lit) = nv.value {
                                    if let Lit::Str(litstr) = expr_lit.lit {
                                        foreign_key = Some(litstr.value());
                                    }
                                }
                            }
                            "relationship" => {
                                if let Expr::Lit(expr_lit) = nv.value {
                                    if let Lit::Str(litstr) = expr_lit.lit {
                                        relationship = Some(litstr.value());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Meta::Path(path) => {
                        let ident = path.get_ident().unwrap().to_string();
                        match ident.as_str() {
                            "uuid" => { uuid = true; primary = true; }
                            "primary" => primary = true,
                            "unique" => unique = true,
                            "not_null" => not_null = true,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        // New: field-level macro
        if attr.path().is_ident("slint_internal_field") {
            let metas = attr.parse_args_with(|input: &ParseBuffer| {
                let mut metas = Vec::new();
                while !input.is_empty() {
                    metas.push(input.parse::<Meta>()?);
                    if !input.is_empty() {
                        input.parse::<token::Comma>()?;
                    }
                }
                Ok(metas)
            }).unwrap();
            for meta in metas {
                match meta {
                    Meta::NameValue(nv) => {
                        let ident = nv.path.get_ident().unwrap().to_string();
                        match ident.as_str() {
                            "foreign_key" => {
                                if let Expr::Lit(expr_lit) = nv.value {
                                    if let Lit::Str(litstr) = expr_lit.lit {
                                        foreign_key = Some(litstr.value());
                                    }
                                }
                            }
                            "relationship" => {
                                if let Expr::Lit(expr_lit) = nv.value {
                                    if let Lit::Str(litstr) = expr_lit.lit {
                                        relationship = Some(litstr.value());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Meta::Path(path) => {
                        let ident = path.get_ident().unwrap().to_string();
                        match ident.as_str() {
                            "primary_key" => primary = true,
                            "uuid" => { uuid = true; primary = true; }
                            "unique" => unique = true,
                            "not_null" => not_null = true,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
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

