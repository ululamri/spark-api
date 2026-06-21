mod admin;
mod admin_ai;
mod admin_audit;
mod admin_auth;
mod admin_cms;
mod admin_login;
mod admin_onboarding;
mod admin_public_guard;
mod admin_notification_outbox;
mod admin_recovery;
mod admin_reset;
mod admin_social;
mod admin_social_bulk;
mod admin_social_ml;
mod admin_social_ops;
mod admin_team;
mod ai;
mod ai_runtime;
mod auth;
mod community;
mod config;
mod error;
mod health;
mod http;
mod hub;
mod lab;
mod learning;
mod media;
mod media_optimizer;
mod media_optimizer_api;
mod moderation;
mod passport;
mod profile;
mod progress;
mod proof;
mod social;
mod state;

use anyhow::Context;
use config::AppConfig;
use state::AppState;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "spark_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env();
    let bind_addr = config.bind_addr();
    let state = AppState::new(config).context("failed to initialize Spark API state")?;
    let app = http::router(&state.config).with_state(state);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind Spark API on {bind_addr}"))?;

    tracing::info!(%bind_addr, "Spark API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
