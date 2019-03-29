use config::Config;
use db::Connection;
use db::ConnectionType;
use diesel::r2d2::{self, ConnectionManager};
use diesel::PgConnection;
use r2d2::Error as R2D2Error;

type R2D2Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub struct Database {
    connection_pool: R2D2Pool,
}

impl Database {
    pub fn from_config(config: &Config) -> Database {
        Database {
            connection_pool: create_connection_pool(&config),
        }
    }

    pub fn get_connection(&self) -> Result<Connection, R2D2Error> {
        let conn = self.connection_pool.get()?;
        Ok(ConnectionType::R2D2(conn).into())
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Database {
            connection_pool: self.connection_pool.clone(),
        }
    }
}

fn create_connection_pool(config: &Config) -> R2D2Pool {
    let r2d2_config = r2d2::Pool::builder()
        .min_idle(Some(config.connection_pool.min))
        .max_size(config.connection_pool.max);

    let connection_manager = ConnectionManager::new(config.database_url.clone());

    r2d2_config
        .build(connection_manager)
        .expect("Failed to create connection pool.")
}
