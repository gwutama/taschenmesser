use serde::Deserialize;


#[derive(Deserialize, Debug)]
pub struct RpcServer {
    enabled: Option<bool>,
    bind_address: Option<String>,
}


impl RpcServer {
    pub fn is_enabled(&self) -> bool {
        return self.enabled.unwrap_or(false);
    }

    pub fn get_bind_address(&self) -> String {
        return self.bind_address.clone().unwrap_or("ipc:///tmp/tsm-unitman.sock".to_string());
    }
}
