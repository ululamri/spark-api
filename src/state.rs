use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::AppConfig;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
}

impl AppState {
    pub fn new(config: AppConfig) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(config.database_max_connections)
            .connect_lazy(&config.database_url)?;

        Ok(Self { config, db })
    }
}
