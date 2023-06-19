use serde::Deserialize;


#[derive(Deserialize, Debug)]
pub struct RpcServer {
    pub enabled: Option<bool>,
    pub bind_address: Option<String>,
}