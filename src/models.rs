use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap};

pub type TypeLineResults<'a> = HashMap<Cow<'a, str>, TypeLineCounter>;

#[derive(Deserialize, Debug)]
pub struct TypeLine {
    #[serde(rename(deserialize = "type"))]
    pub linetype: String,
}

#[derive(Debug, Default)]
pub struct TypeLineCounter {
    pub count: usize,
    pub bytes: usize,
}

impl TypeLineCounter {
    pub fn add_bytes(&mut self, bytes: usize) {
        self.count += 1;
        self.bytes += bytes;
    }
}
