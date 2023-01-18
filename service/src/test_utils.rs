use std::ops::Deref;

use abi::Config;
use sqlx_db_tester::TestDb;

pub struct TestConfig {
    _db: TestDb,
    pub config: Config,
}

impl TestConfig {
    pub fn new() -> Self {
        let mut config = Config::load("fixtures/config.yaml").unwrap();
        let db = TestDb::new(
            &config.db.host,
            config.db.port,
            &config.db.user,
            &config.db.password,
            "../migrations",
        );

        config.db.dbname = db.dbname.clone();
        Self { config, _db: db }
    }
    pub fn with_server_port(port: u16) -> Self {
        let mut config = TestConfig::default();
        config.config.server.port = port;
        config
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for TestConfig {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}
