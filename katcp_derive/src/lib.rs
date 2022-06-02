use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::{parse_macro_input, DataEnum, DeriveInput, Fields, FieldsNamed, Variant};

fn sort_variants(variants: Vec<Variant>) -> (Variant, Variant, Variant) {
    assert_eq!(3, variants.len(), "There must be exactly three variants");
    let mut sorted = HashMap::new();
    variants.into_iter().for_each(|variant| {
        sorted.insert(variant.ident.to_string(), variant);
    });
    (
        sorted
            .get("Request")
            .expect("There must be a `Request` variant")
            .to_owned(),
        sorted
            .get("Reply")
            .expect("There must be a `Reply` variant")
            .to_owned(),
        sorted
            .get("Inform")
            .expect("There must be a `Inform` variant")
            .to_owned(),
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

// This requires that arguments implement Display
fn generate_serde(
    message_name: &Ident,
    msg_result_type: &Ident,
    arg_result_type: &Ident,
    variant: &Variant,
) -> proc_macro2::TokenStream {
    let kind = variant.ident.to_owned();
    let kind_str_lower = kind.to_string().to_lowercase();
    let fn_to = format_ident!("args_from_{}", kind_str_lower);
    let fn_from = format_ident!("{}_from_message", kind_str_lower);
    let fields = get_named_fields(variant);
    let arg_parses = fields.iter().enumerate().map(|(index, ident)| {
        quote! {
            let #ident = msg.arguments.get(#index).ok_or(KatcpError::MissingArgument)?.try_into()?;
        }
    });
    quote! {
        fn #fn_to(msg: &#message_name) -> #arg_result_type {
            if let #message_name::#kind {
                #(#fields),*
             } = msg {
                #(let #fields = #fields.to_string();)*
                Ok((MessageKind::#kind, vec![#(#fields),*]))
            } else {
                Err(KatcpError::BadArgument)
            }
        }
        fn #fn_from(msg: Message) -> #msg_result_type {
            #(#arg_parses)*
            Ok(#message_name::#kind{ #(#fields),* })
        }
    }
}

#[proc_macro_derive(Katcp)]
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
    let args_from_request =
        generate_serde(&message_name, &msg_result_type, &args_result_type, &request);
    let args_from_reply =
        generate_serde(&message_name, &msg_result_type, &args_result_type, &reply);
    let args_from_inform =
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
                    MessageKind::Request => request_from_message(message),
                    MessageKind::Reply => reply_from_message(message),
                    MessageKind::Inform => inform_from_message(message),
                }
            }
        }
        impl KatcpMessage for #message_name {
            fn into_message(self, id: Option<u32>) -> MessageResult {
                let (kind, args) = match self {
                    v @ #message_name::Inform { .. } => args_from_inform(&v)?,
                    v @ #message_name::Reply { .. } => args_from_reply(&v)?,
                    v @ #message_name::Request { .. } => args_from_request(&v)?,
                };
                Ok(Message::new(kind, #message_str, id, args))
            }
        }
        impl FromStr for #message_name {
            type Err = KatcpError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.try_into::<Message>()?.try_into()
            }
        }
        impl TryFrom<&str> for #message_name {
            type Error = KatcpError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                Self::from_str(s)
            }
        }
        #args_from_request
        #args_from_reply
        #args_from_inform
    };
    // Return generated code
    TokenStream::from(generated)
}
