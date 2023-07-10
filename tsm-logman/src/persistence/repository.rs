use log::{warn, debug};
use rusqlite::{Connection, Result};
use crate::persistence::SyslogMessage;


pub struct Repository {
    db_name: String,
    conn: Option<Connection>,
}


impl Repository {
    pub fn new(db_name: String) -> Repository {
        Repository {
            db_name,
            conn: None,
        }
    }

    pub fn connect(&mut self) -> bool {
        match Connection::open(self.db_name.clone()) {
            Ok(conn) => {
                self.conn = Some(conn);
                true
            },
            Err(_) => false,
        }
    }

    pub fn init_table_if_not_exist(&self) -> Result<bool, String> {
        // Create SQLite table "syslog" if not exists
        debug!("Creating table if not exists");

        let conn = match self.conn {
            Some(ref conn) => conn,
            None => {
                warn!("No connection to database");
                return Err(String::from("No connection to database"))
            },
        };

        match conn.execute(
            "CREATE TABLE IF NOT EXISTS syslog (
                id INTEGER PRIMARY KEY,
                from_host TEXT NOT NULL,
                from_port INTEGER NOT NULL,
                facility INTEGER NOT NULL,
                severity INTEGER NOT NULL,
                version INTEGER NOT NULL,
                timestamp INTEGER,
                hostname TEXT,
                appname TEXT,
                procid TEXT,
                msgid TEXT,
                sdata TEXT,
                msg TEXT
            )",
            [],
        ) {
            Ok(_) => Ok(true),
            Err(err) => {
                warn!("Error creating table: {}", err.to_string());
                Err(err.to_string())
            },
        }
    }

    pub fn create(&self, msg: SyslogMessage) -> Result<bool, String> {
        let conn = match self.conn {
            Some(ref conn) => conn,
            None => {
                warn!("No connection to database");
                return Err(String::from("No connection to database"))
            },
        };

        let timestamp = match msg.get_timestamp() {
            Some(ts) => ts,
            None => 0, // invalid timestamp
        };

        match conn.execute(
            "INSERT INTO syslog (
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
                msg
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            [
                msg.get_from_host(),
                msg.get_from_port().to_string(),
                msg.get_facility().to_string(),
                msg.get_severity().to_string(),
                msg.get_version().to_string(),
                timestamp.to_string(),
                msg.get_hostname().unwrap_or(String::from("")),
                msg.get_appname().unwrap_or(String::from("")),
                msg.get_procid().unwrap_or(String::from("")),
                msg.get_msgid().unwrap_or(String::from("")),
                msg.get_sdata().unwrap_or(String::from("")),
                msg.get_msg().unwrap_or(String::from("")),
            ],
        ) {
            Ok(_) => Ok(true),
            Err(err) => {
                warn!("Error inserting into table: {}", err.to_string());
                Err(err.to_string())
            },
        }
    }

    pub fn read_many(&self, limit: u32) -> Vec<SyslogMessage> {
        debug!("Reading {} messages from database", limit);

        let conn = match self.conn {
            Some(ref conn) => conn,
            None => {
                warn!("No connection to database");
                return Vec::new()
            },
        };

        let mut stmt = match conn.prepare(
            "SELECT
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
                msg
            FROM syslog
            ORDER BY id ASC
            LIMIT ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => {
                warn!("Error preparing statement");
                return Vec::new()
            },
        };

        let mut messages: Vec<SyslogMessage> = Vec::new();

        let query_result = stmt.query_map([limit.to_string()], |row| {
            let msg = SyslogMessage::new(
                row.get(0).unwrap_or(None),
                row.get(1).unwrap_or(String::from("")),
                row.get(2).unwrap_or(0),
                row.get(3).unwrap_or(0),
                row.get(4).unwrap_or(0),
                row.get(5).unwrap_or(0),
                row.get(6).unwrap_or(None),
                row.get(7).unwrap_or(None),
                row.get(8).unwrap_or(None),
                row.get(9).unwrap_or(None),
                row.get(10).unwrap_or(None),
                row.get(11).unwrap_or(None),
                row.get(12).unwrap_or(None),
            );
            messages.push(msg);
            Ok(())
        });

        match query_result {
            Ok(_) => messages,
            Err(err) => {
                warn!("Error querying table: {}", err.to_string());
                return Vec::new()
            },
        }
    }

    pub fn delete(&self, id: u32) -> Result<bool, String> {
        let conn = match self.conn {
            Some(ref conn) => conn,
            None => {
                warn!("No connection to database");
                return Err(String::from("No connection to database"))
            },
        };

        match conn.execute(
            "DELETE FROM syslog WHERE id = ?1",
            [id.to_string()],
        ) {
            Ok(_) => Ok(true),
            Err(err) => {
                warn!("Error deleting from table: {}", err.to_string());
                Err(err.to_string())
            },
        }
    }
}