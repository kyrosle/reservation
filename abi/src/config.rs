use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub db: DbConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
    #[serde(default = "default_pool_size")]
    pub max_connections: u32,
}

fn default_pool_size() -> u32 {
    5
}

impl DbConfig {
    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!("postgres://{}@{}:{}", self.user, self.host, self.port)
        } else {
            format!(
                "postgres://{}:{}@{}:{}",
                self.user, self.password, self.host, self.port
            )
        }
    }
    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }
}

impl ServerConfig {
    pub fn url(&self, https: bool) -> String {
        if https {
            format!("https://{}:{}", self.host, self.port)
        } else {
            format!("http://{}:{}", self.host, self.port)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load(filename: impl AsRef<Path>) -> Result<Self, Error> {
        let config =
            std::fs::read_to_string(filename.as_ref()).map_err(|_| Error::ConfigReadError)?;
        serde_yaml::from_str(&config).map_err(|_| Error::ConfigParseError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_should_be_loaded() {
        // let config = include_str!("../fixtures/config.yaml");
        let config = Config::load("fixtures/config.yaml").unwrap();
        assert_eq!(
            config,
            Config {
                db: DbConfig {
                    host: "192.168.30.226".to_string(),
                    port: 55433,
                    user: "postgres".to_string(),
                    password: "1234".to_string(),
                    dbname: "reservation".to_string(),
                    max_connections: 5
                },
                server: ServerConfig {
                    host: "localhost".to_string(),
                    port: 50001
                }
            }
        )
    }
}
