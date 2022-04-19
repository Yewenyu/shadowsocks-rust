use std::{
    collections::HashMap,
    fs::File,
    iter::Map,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{file, yaml};

#[derive(Serialize, Deserialize)]
struct Config {
    pub locals: [Option<SSConfig>; 1],
}
#[derive(Serialize, Deserialize, Clone)]
pub struct SSConfig {
    pub server: String,
    pub server_port: i64,
    pub password: String,
    pub method: String,
    pub local_address: String,
    pub local_port: i64,
    pub mode: String,
    pub local_udp_address: String,
    pub local_udp_port: i64,
    pub acl: String,
    pub log: Option<Log>,
}
impl SSConfig {
    fn empty() -> SSConfig {
        return SSConfig {
            server: "".to_string(),
            server_port: 0,
            password: "".to_string(),
            method: "".to_string(),
            local_address: "".to_string(),
            local_port: 0,
            mode: "".to_string(),
            local_udp_address: "".to_string(),
            local_udp_port: 0,
            acl: "".to_string(),
            log: None,
        };
    }

    pub fn load_from_file(path: String) -> SSConfig {
        let content = file::get_content(path);
        let map: SSConfig = json5::from_str(&content).unwrap();
        // if let Some(Some(some)) = map.locals.get(0) {
        //     let config = SSConfig {
        //         server: some.server.clone(),
        //         server_port: some.server_port,
        //         password: some.password.clone(),
        //         method: some.method.clone(),
        //         local_address: some.local_address.clone(),
        //         local_port: some.local_port,
        //         mode: some.mode.clone(),
        //         local_udp_address: some.local_udp_address.clone(),
        //         local_udp_port: some.local_udp_port,
        //         acl: some.acl.clone(),
        //         log: some.log.clone(),
        //     };
        //     return config;
        // }
        map.handle_log();

        return map;
    }

    fn handle_log(&self) {
        if let Some(log) = &self.log {
            let config_path = PathBuf::from(log.config_path.clone());
            let dir = config_path.as_path();
            let mut sample = yaml::LogYaml::sample();
            let mut level = "info".to_string();
            if log.level == 0 {
                level = "debug".to_string();
            }
            let mut log_path = PathBuf::from_str(dir.parent().unwrap().to_str().unwrap()).unwrap();
            log_path.push("log");
            log_path.set_extension("log");

            let _ = File::create(log_path.clone());
            sample.appenders.requests.path = log_path.to_str().unwrap().to_string();
            sample.root.level = level.clone();
            sample.loggers.app_backend_db.level = level.clone();
            sample.loggers.app_requests.level = level.clone();
            sample.writeToPath(log.config_path.clone());
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Log {
    pub level: i64,
    pub format: Format,
    pub config_path: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Format {
    pub without_time: bool,
}
