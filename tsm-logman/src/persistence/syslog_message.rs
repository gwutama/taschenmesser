use std::collections::HashMap;
use std::string::String;

use crate::server::syslog;


#[derive(Clone)]
pub struct SyslogMessage {
    id: Option<u64>, // id in database
    from_host: String,
    from_port: u16, // port 0 is undefined
    facility: u8,
    severity: u8,
    version: u8,
    timestamp: Option<u64>, // utc timestamp in milli seconds
    hostname: Option<String>,
    appname: Option<String>,
    procid: Option<String>,
    msgid: Option<String>,
    sdata: Option<String>, // Deserialized HashMap<String, String> as json string,
    msg: Option<String>,
}


impl SyslogMessage {
    pub fn new(
        id: Option<u64>,
        from_host: String,
        from_port: u16,
        facility: u8,
        severity: u8,
        version: u8,
        timestamp: Option<u64>, // utc timestamp in milli seconds
        hostname: Option<String>,
        appname: Option<String>,
        procid: Option<String>,
        msgid: Option<String>,
        sdata: Option<String>,
        msg: Option<String>,
    ) -> SyslogMessage {
        SyslogMessage {
            id,
            from_host,
            from_port,
            facility,
            severity,
            version,
            timestamp,
            hostname,
            appname,
            procid,
            msgid,
            sdata,
            msg,
        }
    }

    pub fn get_id(&self) -> Option<u64> {
        self.id
    }

    pub fn get_from_host(&self) -> String {
        self.from_host.clone()
    }

    pub fn get_from_port(&self) -> u16 {
        self.from_port
    }

    pub fn get_facility(&self) -> u8 {
        self.facility
    }

    pub fn get_severity(&self) -> u8 {
        self.severity
    }

    pub fn get_version(&self) -> u8 {
        self.version
    }

    pub fn get_timestamp(&self) -> Option<u64> {
        // convert from unix epoch to DateTime<Utc>
        // value 0 is invalid, return None
        match self.timestamp {
            Some(ts) => match ts {
                0 => None,
                _ => Some(ts),
            },
            None => None,
        }
    }

    pub fn get_hostname(&self) -> Option<String> {
        self.hostname.clone()
    }

    pub fn get_appname(&self) -> Option<String> {
        self.appname.clone()
    }

    pub fn get_procid(&self) -> Option<String> {
        self.procid.clone()
    }

    pub fn get_msgid(&self) -> Option<String> {
        self.msgid.clone()
    }

    pub fn get_sdata(&self) -> Option<String> {
        self.sdata.clone()
    }

    pub fn get_msg(&self) -> Option<String> {
        self.msg.clone()
    }

    pub fn from_syslogmsg(msg: syslog::SyslogMsg) -> SyslogMessage {
        let timestamp = match msg.get_timestamp() {
            Some(ts) => Some(ts.timestamp_millis() as u64),
            None => None,
        };

        let sdata = Self::convert_sdata_to_json(msg.get_sdata());

        SyslogMessage {
            id: None,
            from_host: msg.get_from().ip().to_string(),
            from_port: msg.get_from().port(),
            facility: msg.get_facility(),
            severity: msg.get_severity(),
            version: msg.get_version(),
            timestamp,
            hostname: msg.get_hostname(),
            appname: msg.get_appname(),
            procid: msg.get_procid(),
            msgid: msg.get_msgid(),
            sdata,
            msg: msg.get_msg(),
        }
    }

    fn convert_sdata_to_json(sdata: Option<HashMap<String, String>>) -> Option<String> {
        match sdata.clone() {
            Some(sdata) => {
                let mut json = String::from("{");
                for (key, value) in sdata {
                    json.push_str(&format!("\"{}\":\"{}\",", key, value));
                }
                json.push('}');
                Some(json)
            },
            None => None,
        }
    }
}