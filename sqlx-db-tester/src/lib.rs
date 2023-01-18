use std::{
    fmt::format,
    path::{Path, PathBuf},
    thread,
};

use sqlx::{Connection, Executor, PgConnection, PgPool};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct TestDb {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
}

impl TestDb {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        migration_path: impl Into<String>,
    ) -> Self {
        let uuid = Uuid::new_v4();
        let dbname = format!("test_{uuid}");
        let dbname_clone = dbname.clone();

        let tdb = Self {
            dbname,
            host: host.into(),
            port,
            user: user.into(),
            password: password.into(),
        };

        let server_url = tdb.server_url();
        let url = tdb.url();
        let path = migration_path.into();

        // create database
        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                // connect with server url to create the database
                let mut conn = PgConnection::connect(&server_url).await.unwrap();
                conn.execute(format!(r#"CREATE DATABASE "{dbname_clone}""#).as_str())
                    .await
                    .expect("Failed to create database");

                // now connect to the test database for migrations
                let mut conn = PgConnection::connect(&url).await.unwrap();
                let m = sqlx::migrate::Migrator::new(Path::new(&path))
                    .await
                    .unwrap();
                m.run(&mut conn).await.unwrap();
            });
        })
        .join()
        .unwrap();
        tdb
    }
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
    pub async fn get_pool(&self) -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.url())
            .await
            .unwrap()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let server_url = self.server_url();
        let dbname = self.dbname.clone();

        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = PgConnection::connect(&server_url).await.unwrap();
                // terminate the existing connection
                sqlx::query(&format!(
                    r#"
                                SELECT pg_terminate_backend(pid) 
                                FROM pg_stat_activity 
                                WHERE pid <> pg_backend_pid() 
                                AND datname = '{dbname}'
                            "#
                ))
                .execute(&mut conn)
                .await
                .expect("Terminate all other connections");

                conn.execute(format!(r#"DROP DATABASE "{dbname}""#).as_str())
                    .await
                    .expect("Failed to drop database");
            });
        })
        .join()
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_should_create_and_drop() {
        let tdb = TestDb::new("192.168.30.226", 55433, "postgres", "1234", "./migrations");
        let pool = tdb.get_pool().await;
        // insert todo
        sqlx::query("INSERT INTO todos (title, status) VALUES ('test', 'good')")
            .execute(&pool)
            .await
            .unwrap();
        // get todo
        let (id, title) = sqlx::query_as::<_, (i32, String)>("SELECT id, title FROM todos")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(id >= 1);
        assert_eq!(title, "test");
    }
}
