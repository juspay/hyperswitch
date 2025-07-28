// crates/smithy-core/types.rs

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmithyModel {
    pub namespace: String,
    pub shapes: HashMap<String, SmithyShape>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SmithyShape {
    #[serde(rename = "structure")]
    Structure {
        members: HashMap<String, SmithyMember>,
        #[serde(skip_serializing_if = "Option::is_none")]
        documentation: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "string")]
    String {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "integer")]
    Integer {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "long")]
    Long {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "boolean")]
    Boolean {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "list")]
    List {
        member: Box<SmithyMember>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "union")]
    Union {
        members: HashMap<String, SmithyMember>,
        #[serde(skip_serializing_if = "Option::is_none")]
        documentation: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "enum")]
    StringEnum {
        values: HashMap<String, SmithyEnumValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        documentation: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmithyMember {
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<SmithyTrait>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmithyEnumValue {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "trait")]
pub enum SmithyTrait {
    #[serde(rename = "smithy.api#pattern")]
    Pattern { pattern: String },
    #[serde(rename = "smithy.api#range")]
    Range { min: Option<i64>, max: Option<i64> },
    #[serde(rename = "smithy.api#required")]
    Required,
    #[serde(rename = "smithy.api#documentation")]
    Documentation { documentation: String },
    #[serde(rename = "smithy.api#length")]
    Length { min: Option<u64>, max: Option<u64> }
}

#[derive(Debug, Clone)]
pub struct SmithyField {
    pub name: String,
    pub smithy_type: String,
    pub constraints: Vec<SmithyConstraint>,
    pub documentation: Option<String>,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct SmithyEnumVariant {
    pub name: String,
    pub fields: Vec<SmithyField>,
    pub constraints: Vec<SmithyConstraint>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SmithyConstraint {
    Pattern(String),
    Range(Option<i64>, Option<i64>),
    Length(Option<u64>, Option<u64>),
    Required
}

pub trait SmithyModelGenerator {
    fn generate_smithy_model() -> SmithyModel;
}