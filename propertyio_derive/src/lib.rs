extern crate proc_macro;

use std::str::FromStr;

use proc_macro::TokenStream;

use proc_macro2::{Ident, LexError, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parenthesized, parse_macro_input, Data, DataStruct, DeriveInput, Field, GenericArgument, Path,
    PathArguments, Type,
};

#[derive(Debug)]
enum DataType {
    Bool,
    U8,
    U16,
    U32,
    String,
    Binary,
}

#[derive(Debug)]
struct DataTypeError {
    ty: String,
}

impl std::fmt::Display for DataTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid data type ({}) found", self.ty)
    }
}
impl std::error::Error for DataTypeError {}

impl FromStr for DataType {
    type Err = DataTypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bool" => Ok(DataType::Bool),
            "u8" => Ok(DataType::U8),
            "u16" => Ok(DataType::U16),
            "u32" => Ok(DataType::U32),
            "String" => Ok(DataType::String),
            "vec<u8>" => Ok(DataType::Binary),
            _ => Err(DataTypeError { ty: s.to_string() }),
        }
    }
}

fn get_reader_match_expr(
    ty: &Ident,
    generic_ty: &str,
    prop_id_str: &str,
    field_ident: &Ident,
) -> Result<TokenStream2, LexError> {
    let ty_str = ty.to_string();
    let match_expr = match &*ty_str {
        "String" => quote! {
            props.#field_ident = PropertyReader::to_utf8_string(r)?;
            property_len -= PropertySize::from_utf8_string(&props.#field_ident);
        },
        "Vec" => match generic_ty {
            "u8" => quote! {
                props.#field_ident = PropertyReader::to_binary_data(r)?;
                property_len -= PropertySize::from_binary_data(&props.#field_ident);
            },
            "KeyValuePair" => quote! {
                let value = r.read_key_value_pair()?;
                props.#field_ident.push(value);
                property_len -= PropertySize::from_utf8_string_pair(&props.#field_ident);
            },
            _ => panic!(
                "unexpected type found - should be Vec<u8> or Vec<KeyValuePair>, found {}<{}>",
                ty_str, generic_ty
            ),
        },
        _ => {
            let quote_fn = TokenStream2::from_str(&format!("PropertyReader::to_{}", ty_str))?;
            let quote_property_len_fn =
                TokenStream2::from_str(&format!("PropertySize::from_{}", ty_str))?;
            quote! {
                props.#field_ident = #quote_fn(r)?;
                property_len -= #quote_property_len_fn(&props.#field_ident);
            }
        }
    };

    let prop_id_stream = TokenStream2::from_str(prop_id_str)?;
    return Ok(quote! {
        Some(#prop_id_stream) => {
            #match_expr
        },
    });
}

fn get_writer_match_expr(
    ty: &Ident,
    generic_ty: &str,
    prop_id_str: &str,
    field_ident: &Ident,
) -> Result<TokenStream2, LexError> {
    let prop_id_stream = TokenStream2::from_str(prop_id_str)?;
    let ty_str = ty.to_string();
    let match_expr = match &*ty_str {
        "String" => quote! {
            PropertyWriter::from_utf8_string(w, #prop_id_stream, &self.#field_ident)?;
        },
        "Vec" => match generic_ty {
            "u8" => quote! {
                PropertyWriter::from_binary_data(w, #prop_id_stream, &self.#field_ident)?;
            },
            "KeyValuePair" => quote! {
                PropertyWriter::from_utf8_pair(w, #prop_id_stream, &self.#field_ident)?;
            },
            _ => panic!(
                "unexpected type found - should be Vec<u8> or Vec<KeyValuePair>, found {}<{}>",
                ty_str, generic_ty
            ),
        },
        _ => {
            let quote_fn = TokenStream2::from_str(&format!("PropertyWriter::from_{}", ty_str))?;
            quote! {
                #quote_fn(w, #prop_id_stream, &self.#field_ident)?;
            }
        }
    };

    Ok(quote! {
        #match_expr
    })
}

fn get_len_expr(ty: &Ident, generic_ty: &str, field_ident: &Ident) -> TokenStream2 {
    let ty_str = ty.to_string();
    match &*ty_str {
        "String" => quote! {
            property_len += PropertySize::from_utf8_string(&self.#field_ident);
        },
        "Vec" => match generic_ty {
            "u8" => quote! {
                property_len += PropertySize::from_binary_data(&self.#field_ident);
            },
            "KeyValuePair" => quote! {
                property_len += PropertySize::from_utf8_string_pair(&self.#field_ident);
            },
            _ => panic!(
                "unexpected type found - should be Vec<u8> or Vec<KeyValuePair>, found {}<{}>",
                ty_str, generic_ty
            ),
        },
        _ => {
            let quote_fn = TokenStream2::from_str(&format!(
                "property_len += PropertySize::from_{}(&self.{});",
                ty_str,
                field_ident.to_string()
            ));
            if quote_fn.is_err() {
                panic!(
                    "Failed to generate a matcher expression for calculating the length {}",
                    quote_fn.unwrap_err()
                );
            }
            let a = quote_fn.unwrap();
            return a;
        }
    }
}

#[proc_macro_derive(IOOperations, attributes(ioops))]
pub fn derive_io_fns(input: TokenStream) -> TokenStream {
    let mut reader_impls = TokenStream2::new();
    let mut writer_impls = TokenStream2::new();
    let mut len_impls = TokenStream2::new();

    let input = parse_macro_input!(input as DeriveInput);
    let fields = match &input.data {
        Data::Struct(DataStruct { fields, .. }) => fields,
        _ => panic!("expected a struct with named field"),
    };

    let name = &input.ident;

    for field in fields {
        let Field {
            attrs, ident, ty, ..
        } = field;

        let ident = if let Some(ident) = ident {
            ident
        } else {
            continue;
        };

        let resolved_option_type = get_generic_argument_type(&field, "Option");
        let is_option = resolved_option_type.is_some();
        let resolved_type = match is_option {
            true => resolved_option_type.unwrap(),
            _ => ty,
        };

        let type_ident: Option<&Ident> =
            extract_type_path(resolved_type).and_then(|path_segment| {
                let segments = &path_segment.segments;
                if segments.len() == 1 {
                    return Some(&segments[0].ident);
                }
                None
            });

        let type_ident_str = type_ident.unwrap().to_string();
        let mut generic_arg_type: String = Default::default();
        if type_ident_str == "Vec" {
            let resolved_vec_type = get_generic_argument_type(&field, "Vec");
            let resolved_vec_generic_type = resolved_vec_type.and_then(|v| {
                extract_type_path(v).and_then(|path_segment| {
                    let segments = &path_segment.segments;
                    if segments.len() == 1 {
                        return Some(&segments[0].ident);
                    }
                    None
                })
            });
            match resolved_vec_generic_type {
                Some(v) => generic_arg_type = v.to_string(),
                None => {}
            }
        }

        let mut is_varuint32 = false;
        let mut prop_id: Option<String> = None::<String>;
        for attribute in attrs {
            if attribute.path().is_ident("ioops") {
                _ = attribute.parse_nested_meta(|meta| {
                    if meta.path.is_ident("is_varuint32") {
                        is_varuint32 = true;
                        return Ok(());
                    }
                    if meta.path.is_ident("prop_id") {
                        let content: syn::parse::ParseBuffer;
                        parenthesized!(content in meta.input);
                        let mut content_str = content.to_string();
                        content_str.retain(|c| !c.is_whitespace());
                        prop_id = Some(content_str);
                        return Ok(());
                    }
                    Err(meta.error("unrecognized repr"))
                });
            }
        }

        if prop_id.is_none() {
            panic!("prop_id not found for the field {}", ident.to_string());
        }

        let prop_id_str = prop_id.unwrap().to_string();
        let reader_match_expr =
            get_reader_match_expr(type_ident.unwrap(), &generic_arg_type, &prop_id_str, ident);
        if reader_match_expr.is_err() {
            panic!(
                "Failed to generate a matcher expression for the reader {}",
                reader_match_expr.unwrap_err()
            );
        }
        let reader_match_expr_ok = reader_match_expr.unwrap();
        reader_impls.extend(reader_match_expr_ok);

        let writer_match_expr =
            get_writer_match_expr(type_ident.unwrap(), &generic_arg_type, &prop_id_str, ident);
        if writer_match_expr.is_err() {
            panic!(
                "Failed to generate a matcher expression for the writer {}",
                writer_match_expr.unwrap_err()
            );
        }
        let writer_match_expr_ok = writer_match_expr.unwrap();
        writer_impls.extend(writer_match_expr_ok);

        len_impls.extend(get_len_expr(type_ident.unwrap(), &generic_arg_type, ident));
    }

    let tokens = quote! {

        impl #name {
            pub fn read<R: Reader>(r: &mut R) -> Result<Option<#name>, Error> {
                let mut property_len = r.read_varuint32()?;
                if property_len == 0 {
                    return Ok(None);
                }
                let mut props: #name = Default::default();
                while property_len > 0 {
                    let id = r.read_varuint32()?;
                    let property_id = PropertyID::from_u32(id);
                    if property_id.is_none() {
                        return Err(Error::InvalidPropertyID(id));
                    }
                    match property_id {
                        #reader_impls
                        _ => return Err(Error::InvalidPropertyID(id)),
                    }
                }

                return Ok(Some(props));
            }

            pub fn write<W: Writer>(&self, w: &mut W) -> Result<(), Error> {
                #writer_impls
                return Ok(());
            }

            pub fn len(&self) -> u32 {
                let mut property_len: u32 = 0;
                #len_impls
                return property_len;
            }
        }
    };

    tokens.into()
}

fn get_generic_argument_type<'a>(field: &'a Field, id: &str) -> Option<&'a Type> {
    if let Type::Path(tp) = &field.ty {
        let segments = &tp.path.segments;
        if segments.len() == 1 && segments[0].ident == id {
            return segments
                .last()
                .and_then(|path_segment| {
                    let type_params = &path_segment.arguments;
                    match *type_params {
                        PathArguments::AngleBracketed(ref params) => params.args.first(),
                        _ => None,
                    }
                })
                .and_then(|generic_arg| match *generic_arg {
                    GenericArgument::Type(ref ty) => Some(ty),
                    _ => None,
                });
        }
    }
    None
}

fn extract_type_path(ty: &syn::Type) -> Option<&Path> {
    match *ty {
        syn::Type::Path(ref typepath) if typepath.qself.is_none() => Some(&typepath.path),
        _ => None,
    }
}
