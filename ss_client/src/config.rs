use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{file, yaml};

#[derive(Serialize, Deserialize)]
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
    pub fn load_from_file(path: String) -> SSConfig {
        let content = file::get_content(path);
        let config: SSConfig = json5::from_str(&content).unwrap();
        config.handle_log();
        return config;
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

#[derive(Serialize, Deserialize)]
pub struct Log {
    pub level: i64,
    pub format: Format,
    pub config_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct Format {
    pub without_time: bool,
}
