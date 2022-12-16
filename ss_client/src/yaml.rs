use std::{fs::File, io::Write};

use serde::{Deserialize, Serialize};

use crate::file;

#[derive(Serialize, Deserialize)]
pub struct LogYaml {
    pub refresh_rate: String,
    pub appenders: Appenders,
    pub root: Root,
    pub loggers: Loggers,
}
impl LogYaml {
    pub(crate) fn sample() -> LogYaml {
        let content = "
        \nrefresh_rate: 30 seconds\nappenders:\n  stdout:\n    kind: console\n  requests:\n    kind: file\n    path: \"/Users/xiewenyu/Desktop/rust-project/shadowsocks-rust/ss_client/src/log.log\"\n    encoder:\n      pattern: \"{d} - {m}{n}\"\nroot:\n  level: debug\n  appenders:\n    - stdout\n    - requests\nloggers:\n  app::backend::db:\n    level: debug\n  app::requests:\n    level: debug\n    appenders:\n      - requests\n    additive: false
        ";
        return LogYaml::from_str(content.to_string());
    }

    pub(crate) fn from_str(s: String) -> LogYaml {
        return serde_yaml::from_str(s.as_str()).unwrap();
    }

    pub(crate) fn from_path(p: String) -> LogYaml {
        let s = file::get_content(p);
        return LogYaml::from_str(s);
    }

    pub fn toString(&self) -> String {
        let s = serde_yaml::to_string(self);
        return s.unwrap();
    }

    pub fn writeToPath(&self, path: String) {
        let file = File::create(path).unwrap();
        let _ = serde_yaml::to_writer(file, self);
    }
}

#[derive(Serialize, Deserialize)]
pub struct Appenders {
    pub stdout: Stdout,
    pub requests: Requests,
}

#[derive(Serialize, Deserialize)]
pub struct Requests {
    pub kind: String,
    pub path: String,
    pub encoder: Encoder,
}

#[derive(Serialize, Deserialize)]
pub struct Encoder {
    pub pattern: String,
}

#[derive(Serialize, Deserialize)]
pub struct Stdout {
    pub kind: String,
}

#[derive(Serialize, Deserialize)]
pub struct Loggers {
    #[serde(rename = "app::backend::db")]
    pub app_backend_db: AppBackendDb,
    #[serde(rename = "app::requests")]
    pub app_requests: AppRequests,
}

#[derive(Serialize, Deserialize)]
pub struct AppBackendDb {
    pub level: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppRequests {
    pub level: String,
    pub appenders: Vec<String>,
    pub additive: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Root {
    pub level: String,
    pub appenders: Vec<String>,
}
