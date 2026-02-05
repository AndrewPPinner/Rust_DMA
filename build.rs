use quote::quote;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use syn::{Attribute, Fields, parse_file};

const FILES_TO_PROCESS: [(&str, &str, &str); 2] = [
    ("player_offsets.rs", "PlayerOffsets", "PLAYER_OFFSETS"),
    ("game_offsets.rs", "GameOffsets", "GAME_OFFSETS"),
];

fn main() {
    let lone_sdk_offsets = fetch_csharp_offsets();
    let offsets_map = parse_csharp_offsets(&lone_sdk_offsets);

    for file in FILES_TO_PROCESS {
        let rust_source = fs::read_to_string(format!("src/constants/{}", file.0)).unwrap();
        let syntax_tree = parse_file(&rust_source).unwrap();

        let header = "// GENERATED FILE!! DO NOT EDIT\n\n";
        let generated = generate_const_from_mapping(&syntax_tree, &offsets_map, file.1, file.2);
        let file_contents = format!("{}{}", header, generated);

        let mut f = File::create(format!("./src/constants/generated_{}", file.0)).unwrap();
        f.write_all(file_contents.as_bytes()).unwrap();
    }

    println!("cargo:rerun-if-changed=src/contants/player_offsets.rs");
    println!("cargo:rerun-if-changed=src/contants/game_offsets.rs");
    println!("cargo:rerun-if-changed=src/contants/unity_offsets.rs");
}

fn fetch_csharp_offsets() -> String {
    return reqwest::blocking::get("https://raw.githubusercontent.com/lone-dma/Lone-EFT-DMA-Radar/refs/heads/master/src/Tarkov/SDK.cs").unwrap().text().unwrap();
}

fn parse_csharp_offsets(content: &str) -> HashMap<(String, String), String> {
    let mut offsets = HashMap::new();
    let mut current_struct = String::new();

    for line in content.lines() {
        let line = line.trim();

        if line.contains("struct") && !line.contains("Offsets") {
            let name_start = line.rfind("struct ").unwrap();
            let name_part = &line[name_start + 7..];

            if let Some(name_end) = name_part.find(|c: char| c.is_whitespace() || c == '{') {
                current_struct = name_part[..name_end].to_string();
            } else {
                current_struct = name_part.to_string();
            }
        }

        if line.starts_with("public const uint") {
            let name_start = line.rfind("uint ").unwrap() + 5;
            let name_end = line.rfind(" =").unwrap();
            let name_str = &line[name_start..name_end];

            let value_start = line.find(|x| x == '=').unwrap();
            let value_end = line.find(|x| x == ';').unwrap();
            let value_str = &line[value_start + 2..value_end];

            offsets.insert(
                (current_struct.clone(), name_str.to_string()),
                value_str.to_string(),
            );
        }
    }

    return offsets;
}

fn generate_const_from_mapping(
    syntax_tree: &syn::File,
    offsets: &HashMap<(String, String), String>,
    struct_to_fill: &str,
    generated_const_name: &str,
) -> proc_macro2::TokenStream {
    for item in &syntax_tree.items {
        if let syn::Item::Struct(item_struct) = item {
            println!(
                "cargo:warning=FINDME {} | {}",
                item_struct.ident, struct_to_fill
            );
            if item_struct.ident == struct_to_fill {
                return generate_const_for_struct(item_struct, offsets, generated_const_name);
            }
        }
    }

    quote! { /* struct not found */ }
}

//This code is all super sloppy but it works so eh, will clean up later
fn generate_const_for_struct(
    item_struct: &syn::ItemStruct,
    offsets: &HashMap<(String, String), String>,
    generated_const_name: &str,
) -> proc_macro2::TokenStream {
    let rust_struct_name = &item_struct.ident;
    if let Fields::Named(fields) = &item_struct.fields {
        let field_values = fields.named.iter().filter_map(|field| {
            let rust_field_name = field.ident.as_ref().unwrap();
            let mut chains = HashMap::<String, (String, String)>::new();

            // Find all props with cfg_attr and get their mapping from parsing Lone's SDK
            let offset_value = field
                .attrs
                .iter()
                .find_map(extract_cfg_attr_mapping)
                .and_then(|(csharp_struct_name, csharp_field_name, is_chain)| {
                    if is_chain {
                        let struct_names: Vec<&str> = csharp_struct_name.split("|").collect();
                        let field_names: Vec<&str> = csharp_field_name.split("|").collect();

                        let v1 = offsets.get(&(
                            struct_names[0].trim().to_string(),
                            field_names[0].trim().to_string(),
                        ));
                        let v2 = offsets.get(&(
                            struct_names[1].trim().to_string(),
                            field_names[1].trim().to_string(),
                        ));

                        chains.insert(
                            rust_field_name.to_string(),
                            (v1.unwrap().to_string(), v2.unwrap().to_string()),
                        );
                    }
                    return offsets.get(&(csharp_struct_name, csharp_field_name));
                });

            //Format to u64 literal instead of string value
            if let Some(v) = offset_value {
                let lit_value = syn::LitInt::new(v, proc_macro2::Span::call_site());
                return Some(quote! { #rust_field_name: #lit_value });
            } else if let Some(v) = chains.get(&rust_field_name.to_string()) {
                let lit_value = syn::LitInt::new(&v.0, proc_macro2::Span::call_site());
                let lit_value2 = syn::LitInt::new(&v.1, proc_macro2::Span::call_site());

                return Some(quote! { #rust_field_name: [#lit_value, #lit_value2] });
            }

            return None;
        });

        let const_name_literal =
            syn::Ident::new(generated_const_name, proc_macro2::Span::call_site());
        return quote! {
            pub const #const_name_literal: #rust_struct_name = #rust_struct_name {
                #(#field_values,)*
            };
        };
    }

    quote! {}
}

fn extract_cfg_attr_mapping(attr: &Attribute) -> Option<(String, String, bool)> {
    // Parse #[cfg_attr(any(), csharp_struct = "Player", csharp_field = "Location")]
    if !attr.path().is_ident("cfg_attr") {
        return None;
    }

    let mut struct_name = None;
    let mut field_name = None;
    let mut is_chain = false;

    if let Ok(meta_list) = &attr.meta.require_list() {
        let tokens_str = meta_list.tokens.to_string();

        for part in tokens_str.split(',') {
            let part = part.trim();
            if part.starts_with("csharp_struct") {
                if let Some(value) = extract_string_attr_value(part) {
                    struct_name = Some(value);
                }
            } else if part.starts_with("csharp_field") {
                if let Some(value) = extract_string_attr_value(part) {
                    field_name = Some(value);
                }
            } else if part.starts_with("is_chain") {
                is_chain = true
            }
        }
    }

    match (struct_name, field_name) {
        (Some(s), Some(f)) => Some((s, f, is_chain)),
        _ => None,
    }
}

fn extract_string_attr_value(s: &str) -> Option<String> {
    let start = s.find('"')?;
    let end = s.rfind('"')?;
    if start < end {
        Some(s[start + 1..end].to_string())
    } else {
        None
    }
}
