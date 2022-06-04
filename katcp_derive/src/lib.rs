use std::{collections::HashMap, iter::zip};

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DataEnum, DeriveInput, Fields, FieldsNamed, Type, Variant};

fn sort_variants(variants: Vec<Variant>) -> (Variant, Variant, Variant) {
    assert_eq!(3, variants.len(), "There must be exactly three variants");
    let mut sorted = HashMap::new();
    variants.into_iter().for_each(|variant| {
        sorted.insert(variant.ident.to_string(), variant);
    });
    (
        sorted.get("Request").expect("There must be a `Request` variant").to_owned(),
        sorted.get("Reply").expect("There must be a `Reply` variant").to_owned(),
        sorted.get("Inform").expect("There must be a `Inform` variant").to_owned(),
    )
}

fn get_named_fields(variant: &Variant) -> Vec<Ident> {
    if let Fields::Named(FieldsNamed { named, .. }) = variant.fields.clone() {
        named.iter().map(|f| f.ident.to_owned().expect("Field must be named")).collect()
    } else {
        panic!("Fields in message variants must be named")
    }
}

fn get_field_types(variant: &Variant) -> Vec<Type> {
    if let Fields::Named(FieldsNamed { named, .. }) = variant.fields.clone() {
        named.iter().map(|f| f.ty.to_owned()).collect()
    } else {
        panic!("Fields in message variants must be named")
    }
}

// This requires that arguments implement Display
fn generate_serde(
    message_name: &Ident,
    msg_result_type: &Ident,
    arg_result_type: &Ident,
    variant: &Variant,
) -> (proc_macro2::TokenStream, Ident, Ident) {
    let kind = variant.ident.to_owned();
    let kind_str_lower = kind.to_string().to_lowercase();
    let message_name_lower = message_name.to_string().to_lowercase();
    let fn_to = format_ident!("{}_args_from_{}", message_name_lower, kind_str_lower);
    let fn_from = format_ident!("{}_from_{}_message", kind_str_lower, message_name_lower);
    let fields = get_named_fields(variant);
    let types = get_field_types(variant);
    let arg_parses = zip(fields.clone(), types).enumerate().map(|(index, (ident, typ))| {
        quote! {
            let #ident = <#typ>::from_argument(           // Perform conversion, assuming field impls FromKatcpArgument
                msg.arguments
                    .get(#index)                          // Get the index associated with this field
                    .ok_or(KatcpError::MissingArgument)?, // Ensure it exists
            )?;
        }
    });
    (
        quote! {
            fn #fn_to(msg: &#message_name) -> #arg_result_type {
                if let #message_name::#kind {
                    #(#fields),*
                } = msg {
                    #(let #fields = #fields.to_argument();)* // Assume field impls ToKatcpArgument
                    Ok((MessageKind::#kind, vec![#(#fields),*]))
                } else {
                    Err(KatcpError::BadArgument)
                }
            }
            fn #fn_from(msg: Message) -> #msg_result_type {
                #(#arg_parses)*
                Ok(#message_name::#kind{ #(#fields),* })
            }
        },
        fn_to,
        fn_from,
    )
}

#[proc_macro_derive(KatcpMessage)]
pub fn derive_katcp(tokens: TokenStream) -> TokenStream {
    // We need to parse out the name of the enum,
    // the three variants(inform, reply, request)
    // and the fields of those variants
    let input = parse_macro_input!(tokens as DeriveInput);
    let message_name = input.ident;
    let variants: Vec<_> = match input.data {
        syn::Data::Enum(DataEnum { variants, .. }) => variants.into_iter().collect(),
        _ => panic!("KatcpMessage can only be derived on Enums"),
    };
    // Collect the three variants
    let (request, reply, inform) = sort_variants(variants);

    // Generate code
    let msg_result_type = format_ident!("{}Result", message_name);
    let args_result_type = format_ident!("{}ArgsResult", message_name);
    let message_str = message_name.to_string().to_lowercase();

    // Serialize into args fns
    let (args_from_request, fn_to_request, fn_from_request) =
        generate_serde(&message_name, &msg_result_type, &args_result_type, &request);
    let (args_from_reply, fn_to_reply, fn_from_reply) =
        generate_serde(&message_name, &msg_result_type, &args_result_type, &reply);
    let (args_from_inform, fn_to_inform, fn_from_inform) =
        generate_serde(&message_name, &msg_result_type, &args_result_type, &inform);

    let generated = quote! {
        type #msg_result_type = Result<#message_name,KatcpError>;
        type #args_result_type = Result<(MessageKind, Vec<String>),KatcpError>;
        impl TryFrom<Message> for #message_name {
            type Error = KatcpError;
            fn try_from(message: Message) -> Result<Self,Self::Error> {
                if message.name != #message_str {
                    return Err(KatcpError::IncorrectType);
                }
                match message.kind {
                    MessageKind::Request => #fn_from_request(message),
                    MessageKind::Reply => #fn_from_reply(message),
                    MessageKind::Inform => #fn_from_inform(message),
                }
            }
        }
        impl KatcpMessage for #message_name {
            fn into_message(self, id: Option<u32>) -> MessageResult {
                let (kind, args) = match self {
                    v @ #message_name::Inform { .. } => #fn_to_inform(&v)?,
                    v @ #message_name::Reply { .. } => #fn_to_reply(&v)?,
                    v @ #message_name::Request { .. } => #fn_to_request(&v)?,
                };
                // Safety: all strings have been escaped and core types have been
                // serialized according to the spec, so we shouldn't fail any parser
                // rules here, implying this is ok
                Ok(unsafe { Message::new_unchecked(kind, #message_str, id, args) } )
            }
        }
        impl TryFrom<&str> for #message_name {
            type Error = KatcpError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                let message: Message = s.try_into()?;
                message.try_into()
            }
        }
        #args_from_request
        #args_from_reply
        #args_from_inform
    };
    // Return generated code
    TokenStream::from(generated)
}

#[proc_macro_derive(KatcpDiscrete)]
pub fn derive_katcp_discrete(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let enum_name = input.ident;
    let variants: Vec<_> = match input.data {
        syn::Data::Enum(DataEnum { variants, .. }) => variants.into_iter().collect(),
        _ => panic!("KatcpDiscrete can only be derived on Enums"),
    };
    let to_str_pairs = variants.iter().map(|variant| {
        let ident = variant.ident.clone();
        let ident_lowercase = variant.ident.to_string().to_lowercase();
        quote! {
            #enum_name::#ident => #ident_lowercase
        }
    });
    let from_str_pairs = variants.iter().map(|variant| {
        let ident = variant.ident.clone();
        let ident_lowercase = variant.ident.to_string().to_lowercase();
        quote! {
            #ident_lowercase => #enum_name::#ident
        }
    });
    let generated = quote! {
        impl ToKatcpArgument for #enum_name {
            fn to_argument(&self) -> String {
                match self {
                    #(#to_str_pairs),*
                }.to_owned()
            }
        }
        impl FromKatcpArgument for #enum_name {
            type Err = KatcpError;
            fn from_argument(s: impl AsRef<str>) -> Result<Self, Self::Err> {
                let level = match s.as_ref() {
                    #(#from_str_pairs),*,
                    _ =>  return Err(KatcpError::BadArgument),
                };
                Ok(level)
            }
        }
    };
    TokenStream::from(generated)
}
