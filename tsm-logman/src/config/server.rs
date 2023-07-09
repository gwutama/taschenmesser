use serde::Deserialize;


#[derive(Deserialize, Debug)]
pub struct Server {
    host: Option<String>,
    port: Option<u16>,
}


impl Server {
    pub fn get_host(&self) -> String {
        return self.host.clone().unwrap_or(String::from("localhost"));
    }

    pub fn get_port(&self) -> u16 {
        return self.port.unwrap_or(514);
    }
}
