use diesel::{
    Connection, PgConnection,
    r2d2::{self, ConnectionManager},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::env;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum DatabaseError {
    ConnectionError(String),
    PoolError(String),
    ConfigurationError(String),
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            DatabaseError::PoolError(msg) => write!(f, "Pool error: {}", msg),
            DatabaseError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

pub fn create_connection_pool() -> Result<DbPool, DatabaseError> {
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| DatabaseError::ConfigurationError("DATABASE_URL not set".to_string()))?;

    let manager = ConnectionManager::<PgConnection>::new(database_url);

    r2d2::Pool::builder()
        .max_size(10) // Maximum number of connections in the pool
        .min_idle(Some(1)) // Minimum number of idle connections
        .build(manager)
        .map_err(|e| DatabaseError::PoolError(e.to_string()))
}

pub fn get_database_connection() -> Result<PgConnection, DatabaseError> {
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| DatabaseError::ConfigurationError("DATABASE_URL not set".to_string()))?;

    PgConnection::establish(&database_url)
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))
}

pub fn get_connection_from_pool(pool: &DbPool) -> Result<DbConnection, DatabaseError> {
    pool.get()
        .map_err(|e| DatabaseError::PoolError(e.to_string()))
}

pub fn run_migrations(conn: &mut PgConnection) -> Result<(), DatabaseError> {
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
    Ok(())
}
