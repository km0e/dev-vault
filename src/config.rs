use serde::Deserialize;
use serde::Serialize;
use xcfg::XCfg;
#[derive(Debug, Serialize, Deserialize, XCfg)]
pub struct Config {
    pub name: String,
    pub age: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "John".to_string(),
            age: 30,
        }
    }
}
