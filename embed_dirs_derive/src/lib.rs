#![recursion_limit = "128"]
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use crate::proc_macro::TokenStream;
use syn::*;

use std::fs;
use std::path::PathBuf;

mod path;

fn generate_file_list(ident: &syn::Ident, dir_path: &PathBuf, return_files: bool) -> TokenStream {
    let paths = fs::read_dir(dir_path).unwrap();
    let dir_names = paths
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if !return_files && path.is_dir() {
                    path.file_name().and_then(|n| n.to_str().map(String::from))
                } else if return_files && path.is_file() {
                    path.to_str().map(String::from)
                } else {
                    None
                }
            })
        })
        .collect::<Vec<String>>();

    let code = quote! {
      impl #ident {
          pub const DIR_NAMES: &'static [&'static str] = &[#(#dir_names),*];
      }
    };

    code.into()
}

fn help_folder_required() {
    panic!("#[derive(EmbedFileList)] should contain one attribute like this #[embed_dir = \"example/\"]");
}

fn impl_embed_file_list(ast: &syn::DeriveInput) -> TokenStream {
    let ident = &ast.ident;
    if ast.attrs.len() == 0 {
        help_folder_required();
    }

    let attrs = &ast.attrs;

    let return_files = attrs
        .into_iter()
        .map(|attr| attr.parse_meta().unwrap())
        .find_map(|meta| {
            match meta {
                // Match '#[ident = lit]' attributes. Match guard makes it `#[embed_dir = "..."]`
                Meta::NameValue(MetaNameValue {
                    ref ident, ref lit, ..
                }) if ident == "return_files" => {
                    if let Lit::Bool(lit) = lit {
                        Some(lit.value)
                    } else {
                        None
                    }
                }

                _ => None,
            }
        })
        .unwrap_or(false);

    let dir_path = attrs
        .into_iter()
        .map(|attr| attr.parse_meta().unwrap())
        .find_map(|meta| {
            match meta {
                // Match '#[ident = lit]' attributes. Match guard makes it `#[embed_dir = "..."]`
                Meta::NameValue(MetaNameValue {
                    ref ident, ref lit, ..
                }) if ident == "embed_dir" => {
                    if let Lit::Str(lit) = lit {
                        Some(lit.value())
                    } else {
                        None
                    }
                }

                _ => None,
            }
        })
        .expect("#[derive(EmbedFileList)] should contain the #[embed_dir = \"...\" attribute");

    let abs_path = path::directory_relative_to_manifest(&dir_path)
        .expect("#[derive(EmbedFileList)] unable to determine relative path");

    if !abs_path.exists() {
        panic!(
            "#[derive(EmbedFileList)] directory '{}' does not exist.",
            abs_path.to_string_lossy()
        );
    }
    generate_file_list(ident, &abs_path, return_files).into()
}

#[proc_macro_derive(EmbedFileList, attributes(embed_dir, return_files))]
pub fn derive_input_object(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_embed_file_list(&ast)
}
