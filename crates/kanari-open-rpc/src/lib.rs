// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use serde_json::{Value, json};

pub use kanari_open_rpc_macros::{open_rpc, open_rpc_method};

pub const OPENRPC_VERSION: &str = "1.3.2";

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub openrpc: &'static str,
    pub info: InfoObject,
    pub methods: Vec<MethodObject>,
}

impl Project {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: &'static str,
        title: &'static str,
        description: &'static str,
        contact_name: &'static str,
        contact_url: &'static str,
        contact_email: &'static str,
        license_name: &'static str,
        license_url: &'static str,
    ) -> Self {
        Self {
            openrpc: OPENRPC_VERSION,
            info: InfoObject {
                title,
                description,
                version,
                contact: ContactObject {
                    name: contact_name,
                    url: contact_url,
                    email: contact_email,
                },
                license: LicenseObject {
                    name: license_name,
                    url: license_url,
                },
            },
            methods: Vec::new(),
        }
    }

    pub fn add_method(&mut self, method: MethodObject) {
        self.methods.push(method);
    }

    pub fn add_methods<I>(&mut self, methods: I)
    where
        I: IntoIterator<Item = MethodObject>,
    {
        self.methods.extend(methods);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InfoObject {
    pub title: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    pub contact: ContactObject,
    pub license: LicenseObject,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContactObject {
    pub name: &'static str,
    pub url: &'static str,
    pub email: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct LicenseObject {
    pub name: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodObject {
    pub name: &'static str,
    pub summary: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    pub params: Vec<ContentDescriptorObject>,
    pub result: ContentDescriptorObject,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagObject>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentDescriptorObject {
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    pub required: bool,
    pub schema: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagObject {
    pub name: &'static str,
}

pub fn method(
    name: &'static str,
    summary: &'static str,
    description: Option<&'static str>,
    params: Vec<ContentDescriptorObject>,
    result: ContentDescriptorObject,
    tags: &[&'static str],
) -> MethodObject {
    MethodObject {
        name,
        summary,
        description,
        params,
        result,
        tags: tags.iter().map(|name| TagObject { name }).collect(),
    }
}

pub fn param(
    name: &'static str,
    description: &'static str,
    required: bool,
    schema: Value,
) -> ContentDescriptorObject {
    ContentDescriptorObject {
        name,
        description: Some(description),
        required,
        schema,
    }
}

pub fn result(
    name: &'static str,
    description: &'static str,
    schema: Value,
) -> ContentDescriptorObject {
    ContentDescriptorObject {
        name,
        description: Some(description),
        required: true,
        schema,
    }
}

pub fn schema_string() -> Value {
    json!({ "type": "string" })
}

pub fn schema_integer() -> Value {
    json!({ "type": "integer", "minimum": 0 })
}

pub fn schema_boolean() -> Value {
    json!({ "type": "boolean" })
}

pub fn schema_object() -> Value {
    json!({ "type": "object" })
}

pub fn schema_array(items: Value) -> Value {
    json!({
        "type": "array",
        "items": items
    })
}

pub fn optional_schema(schema: Value) -> Value {
    json!({
        "oneOf": [
            schema,
            { "type": "null" }
        ]
    })
}

pub fn object_schema(fields: &[(&'static str, Value)]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for (name, schema) in fields {
        let is_optional = schema
            .get("oneOf")
            .and_then(|value| value.as_array())
            .map(|variants| {
                variants
                    .iter()
                    .any(|variant| variant.get("type") == Some(&Value::String("null".into())))
            })
            .unwrap_or(false);

        if !is_optional {
            required.push(Value::String((*name).to_string()));
        }
        properties.insert((*name).to_string(), schema.clone());
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}
