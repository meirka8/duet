use crate::predicate::ContextPredicate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Command specification registered in the global engine registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub category: String,
    pub precondition: Option<ContextPredicate>,
    pub args_schema: Option<Value>,
}

impl Command {
    pub fn new(id: impl Into<String>, title: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category: category.into(),
            precondition: None,
            args_schema: None,
        }
    }

    pub fn with_precondition(mut self, precondition: ContextPredicate) -> Self {
        self.precondition = Some(precondition);
        self
    }

    pub fn with_args_schema(mut self, schema: Value) -> Self {
        self.args_schema = Some(schema);
        self
    }
}
