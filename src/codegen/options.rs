use crate::api::generated::types::{AgentOption, AgentOptionTransport};
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use syn::{Type, parse_str};

#[derive(Deserialize, Debug)]
pub struct AgentDefinition {
    // missing: edition
    // missing: agent table
    // missing: runtimes table
    options: HashMap<String, AgentOption>,
}

struct OptionTypeInfo {
    name: String,
    field_name: String,
    optional: bool,
    r#type: String,
    file_system: bool,
}

impl OptionTypeInfo {
    fn new<T>(
        name: String,
        default: Option<T>,
        required: Option<bool>,
        r#type: &str,
        transport: Option<AgentOptionTransport>,
    ) -> Self {
        Self {
            name: name.clone(),
            field_name: name.to_lowercase(),
            optional: default.is_none() && !required.unwrap_or_default(),
            r#type: r#type.to_string(),
            file_system: matches!(transport, Some(AgentOptionTransport::Fs)),
        }
    }

    fn new_list(name: String, r#type: &str, transport: Option<AgentOptionTransport>) -> Self {
        Self {
            name: name.clone(),
            field_name: name.to_lowercase(),
            optional: false,
            r#type: format!("Vec<{}>", r#type),
            file_system: matches!(transport, Some(AgentOptionTransport::Fs)),
        }
    }
}

pub fn generate_option_structure(file: impl Into<PathBuf>) -> String {
    let agent: AgentDefinition =
        toml::from_str(fs::read_to_string(file.into()).unwrap().as_str()).unwrap();

    let info = agent
        .options
        .into_iter()
        .map(|(name, option)| match option {
            AgentOption::Blob { transport, .. } => {
                OptionTypeInfo::new_list(name, "::coral_rs::codegen::__private::Blob", transport)
            }
            AgentOption::ListBlob { transport, .. } => {
                OptionTypeInfo::new_list(name, "::coral_rs::codegen::__private::Blob", transport)
            }
            AgentOption::Bool {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "bool", transport),
            AgentOption::I8 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "i8", transport),
            AgentOption::ListI8 { transport, .. } => {
                OptionTypeInfo::new_list(name, "i8", transport)
            }
            AgentOption::U8 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "u8", transport),
            AgentOption::ListU8 { transport, .. } => {
                OptionTypeInfo::new_list(name, "u8", transport)
            }
            AgentOption::I16 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "i16", transport),
            AgentOption::ListI16 { transport, .. } => {
                OptionTypeInfo::new_list(name, "i16", transport)
            }
            AgentOption::U16 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "u16", transport),
            AgentOption::ListU16 { transport, .. } => {
                OptionTypeInfo::new_list(name, "u16", transport)
            }
            AgentOption::I32 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "i32", transport),
            AgentOption::ListI32 { transport, .. } => {
                OptionTypeInfo::new_list(name, "i32", transport)
            }
            AgentOption::U32 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "u32", transport),
            AgentOption::ListU32 { transport, .. } => {
                OptionTypeInfo::new_list(name, "u32", transport)
            }
            AgentOption::I64 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "i64", transport),
            AgentOption::ListI64 { transport, .. } => {
                OptionTypeInfo::new_list(name, "i64", transport)
            }
            AgentOption::U64 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "u64", transport),
            AgentOption::ListU64 { transport, .. } => {
                OptionTypeInfo::new_list(name, "u64", transport)
            }
            AgentOption::F32 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "f32", transport),
            AgentOption::ListF32 { transport, .. } => {
                OptionTypeInfo::new_list(name, "f32", transport)
            }
            AgentOption::F64 {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "f64", transport),
            AgentOption::ListF64 { transport, .. } => {
                OptionTypeInfo::new_list(name, "f64", transport)
            }
            AgentOption::String {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "String", transport),
            AgentOption::ListString { transport, .. } => {
                OptionTypeInfo::new_list(name, "String", transport)
            }
            AgentOption::Number {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "f64", transport),
            AgentOption::Secret {
                default,
                required,
                transport,
                ..
            } => OptionTypeInfo::new(name, default, required, "String", transport),
        })
        .collect::<Vec<_>>();

    let fields = info.iter().map(|x| {
        let field_name = format_ident!("{}", x.field_name);
        let field_type: Type = if x.optional {
            parse_str(format!("Option<{}>", &x.r#type).as_str()).unwrap()
        } else {
            parse_str(&x.r#type).unwrap()
        };

        quote! {
            #field_name: #field_type,
        }
    });

    let field_initializers = info.iter().map(|x| {
        let field_name = format_ident!("{}", x.field_name);
        let name = x.name.clone();
        let fs = x.file_system;

        let fn_name = if x.r#type.starts_with("Vec") {
            format_ident!("get_options")
        } else {
            format_ident!("get_option")
        };

        if x.optional {
            quote! {
                #field_name: match ::coral_rs::codegen::__private::#fn_name(#name, #fs) {
                    Ok(x) => Some(x),
                    Err(::coral_rs::codegen::__private::Error::MissingOption(_)) => None,
                    Err(e) => return Err(e)
                },
            }
        } else {
            quote! {
                #field_name: ::coral_rs::codegen::__private::#fn_name(#name, #fs)?,
            }
        }
    });

    prettyplease::unparse(
        &syn::parse_file(
            &quote! {
                #[derive(Debug, Clone)]
                pub struct Options {
                    #(#fields)*
                }

                impl Options {
                    pub fn parse() -> Result<Self, ::coral_rs::codegen::__private::Error> {
                        Ok(Self {
                            #(#field_initializers)*
                        })
                    }
                }
            }
            .to_string(),
        )
        .unwrap(),
    )
}
