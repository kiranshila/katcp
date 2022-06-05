use std::{collections::HashMap, iter::zip};

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DataEnum, DeriveInput, Fields, FieldsNamed, Type, Variant};

fn sort_variants(variants: Vec<Variant>) -> (Option<Variant>, Option<Variant>, Option<Variant>) {
    assert!(
        (1..=4).contains(&variants.len()),
        "There must be between 1 and 3 variants"
    );
    let mut sorted = HashMap::new();
    variants.into_iter().for_each(|variant| {
        sorted.insert(variant.ident.to_string(), variant);
    });
    (
        sorted.get("Request").cloned(),
        sorted.get("Reply").cloned(),
        sorted.get("Inform").cloned(),
    )
}

fn get_named_fields(variant: &Variant) -> Vec<Ident> {
    if let Fields::Named(FieldsNamed { named, .. }) = variant.fields.clone() {
        named
            .iter()
            .map(|f| f.ident.to_owned().expect("Field must be named"))
            .collect()
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
fn generate_serde(variant: &Option<Variant>) -> proc_macro2::TokenStream {
    // Check if variant is None, return empty TokenStream if it is
    let variant = if let Some(v) = variant {
        v
    } else {
        return quote! {};
    };
    let kind = variant.ident.to_owned();
    let kind_str_lower = kind.to_string().to_lowercase();
    // Two function names
    let fn_to_variant = format_ident!("to_{}_variant", kind_str_lower);
    let fn_to_message_args = format_ident!("to_{}_message_args", kind_str_lower);
    // Iterators
    let fields = get_named_fields(variant);
    let types = get_field_types(variant);
    // Iterator for Message -> Variant fn
    let arg_parses = zip(fields.clone(), types)
        .enumerate()
        .map(|(index, (ident, typ))| {
            quote! {
                let #ident = <#typ>::from_argument(           // Perform conversion, assuming field impls FromKatcpArgument
                    msg.arguments
                        .get(#index)                          // Get the index associated with this field
                        .ok_or(KatcpError::MissingArgument)?, // Ensure it exists
                )?;
            }
        });
    // The serde methods
    quote! {
        fn #fn_to_message_args(&self) -> Result<(MessageKind, Vec<String>),KatcpError> {
            if let Self::#kind {
                #(#fields),*
            } = self {
                #(let #fields = #fields.to_argument();)* // Assume field impls ToKatcpArgument
                Ok((MessageKind::#kind, vec![#(#fields),*]))
            } else {
                Err(KatcpError::BadArgument)
            }
        }
        fn #fn_to_variant(msg: &Message) -> Result<Self,KatcpError> {
            #(#arg_parses)*
            Ok(Self::#kind{ #(#fields),* })
        }
    }
}

fn generate_try_from(
    message_name: &Ident,
    sorted_variants: &(Option<Variant>, Option<Variant>, Option<Variant>),
) -> proc_macro2::TokenStream {
    let message_str = message_name.to_string().to_lowercase();
    let request_fn = sorted_variants.0.as_ref().map_or(
        quote! {unimplemented!()},
        |_| quote! {#message_name::to_request_variant(&message)},
    );
    let reply_fn = sorted_variants.1.as_ref().map_or(
        quote! {unimplemented!()},
        |_| quote! {#message_name::to_reply_variant(&message)},
    );
    let inform_fn = sorted_variants.2.as_ref().map_or(
        quote! {unimplemented!()},
        |_| quote! {#message_name::to_inform_variant(&message)},
    );
    quote! {
        impl TryFrom<Message> for #message_name {
            type Error = KatcpError;
            fn try_from(message: Message) -> Result<Self,Self::Error> {
                if message.name != #message_str {
                    return Err(KatcpError::IncorrectType);
                }
                match message.kind {
                    MessageKind::Request => #request_fn,
                    MessageKind::Reply => #reply_fn,
                    MessageKind::Inform => #inform_fn,
                }
            }
        }
    }
}

fn generate_katcp_message_impl(
    message_name: &Ident,
    sorted_variants: &(Option<Variant>, Option<Variant>, Option<Variant>),
) -> proc_macro2::TokenStream {
    let message_str = message_name.to_string().to_lowercase();

    let request_fn = sorted_variants.0.as_ref().map_or(quote! {}, |_| {
        quote! {
            v @ Self::Request { .. } => v.to_request_message_args()?,
        }
    });
    let reply_fn = sorted_variants.1.as_ref().map_or(quote! {}, |_| {
        quote! {
            v @ Self::Reply { .. } => v.to_reply_message_args()?,
        }
    });
    let inform_fn = sorted_variants.2.as_ref().map_or(quote! {}, |_| {
        quote! {
            v @ Self::Inform { .. } => v.to_inform_message_args()?,
        }
    });
    quote! {
        impl KatcpMessage for #message_name {
            fn to_message(&self, id: Option<u32>) -> MessageResult {
                let (kind, args) = match self {
                    #request_fn
                    #reply_fn
                    #inform_fn
                };
                // Safety: all strings have been escaped and core types have been
                // serialized according to the spec, so we shouldn't fail any parser
                // rules here, implying this is ok
                Ok(unsafe { Message::new_unchecked(kind, #message_str, id, args) } )
            }
        }
    }
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
    let sorted_variants = sort_variants(variants);

    // Serialize into args fns
    let serde_req = generate_serde(&sorted_variants.0);
    let serde_reply = generate_serde(&sorted_variants.1);
    let serde_inform = generate_serde(&sorted_variants.2);

    // TryFrom<Message> Block
    let try_from_message = generate_try_from(&message_name, &sorted_variants);

    // impl KatcpMessage Block
    let katcp_message_impl = generate_katcp_message_impl(&message_name, &sorted_variants);

    let generated = quote! {
        #try_from_message
        #katcp_message_impl
        impl TryFrom<&str> for #message_name {
            type Error = KatcpError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                let message: Message = s.try_into()?;
                message.try_into()
            }
        }
        impl #message_name {
            #serde_req
            #serde_reply
            #serde_inform
        }
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
