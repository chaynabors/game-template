use std::borrow::Cow;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::asset::Asset;

#[derive(Debug, Deserialize, Serialize)]
enum StateType {
    Float(f64),
    Int(i64),
    String(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct State(HashMap<Cow<'static, str>, StateType>);

impl Asset for State {
    const BACKEND: crate::asset::Backend = crate::asset::Backend::Yaml;
}
