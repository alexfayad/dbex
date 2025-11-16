use std::collections::HashMap;
use crate::BsonValue;

/// BSON Document - a map of string keys to BSON values
pub type Document = HashMap<String, BsonValue>;

// Note: You can't impl a type alias directly, but you can add helper functions
// or use extension traits. For now, Document::new() works because HashMap::new() exists.
// If you need custom methods, you'd need to wrap it in a struct instead.

// Helper functions for Document
impl Document {
    /// Create a new empty document
    pub fn new() -> Self {
        HashMap::new()
    }
}