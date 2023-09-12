use std::str::FromStr;

use color_eyre::eyre;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio_postgres::Transaction;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    status_code: u16,
    message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Postgres: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    #[error("Deadpool: {0}")]
    PoolError(#[from] deadpool_postgres::PoolError),

    #[error("Uncaught Exception")]
    Eyre(#[from] eyre::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("{self}");

        // build a json response and return
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                status_code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                message: self.to_string(),
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env = "DATABASE_URL")]
    pub database_url: String,

    #[clap(long, env = "DATABASE_POOL_MAX_SIZE", default_value = "4")]
    pub database_pool_max_size: usize,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: deadpool_postgres::Pool,
}

#[derive(Debug, Serialize)]
pub struct Fortune {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct FortunesListRequest {
    pub quantity: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct FortunesListResponse {
    pub count: usize,
    pub items: Vec<Fortune>,
}

async fn build_pool(config: &Config) -> eyre::Result<deadpool_postgres::Pool> {
    let database_url = url::Url::from_str(&config.database_url)?;

    let mut builder = deadpool_postgres::Config::new();
    builder.application_name = Some(String::from("gottagofast"));
    builder.manager = Some(deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    });

    builder.pool = Some(deadpool_postgres::PoolConfig {
        max_size: config.database_pool_max_size as _,
        ..Default::default()
    });

    builder.dbname = Some(database_url.path().trim_start_matches('/').into());
    builder.host = database_url.host_str().map(String::from);
    builder.port = database_url.port().or(Some(5432));
    builder.user = Some(database_url.username().into());
    builder.password = database_url.password().map(String::from);

    Ok(builder.create_pool(
        Some(deadpool_postgres::Runtime::Tokio1),
        tokio_postgres::NoTls,
    )?)
}

async fn get_fortunes(db: &Transaction<'_>, count: i64) -> eyre::Result<Vec<Fortune>> {
    Ok(db
        .query(
            "SELECT content FROM fortune ORDER BY random() LIMIT $1",
            &[&count],
        )
        .await?
        .iter()
        .map(|row| Fortune {
            content: row.get("content"),
        })
        .collect())
}

async fn fortunes(
    State(state): State<AppState>,
    request: Json<FortunesListRequest>,
) -> Result<Json<FortunesListResponse>, Error> {
    let mut conn = state.pool.get().await?;
    let db = conn.transaction().await?;

    // grab stuff from the database
    let items = get_fortunes(&db, request.quantity.unwrap_or(20)).await?;

    // ensure you commit here, otherwise when `db` is dropped, it will rollback
    db.commit().await?;

    Ok(Json(FortunesListResponse {
        count: items.len(),
        items,
    }))
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // load .env and parse into Config
    dotenvy::dotenv().ok();
    let config = Config::parse();

    // setup tracing and error reporting
    install_tracing();
    color_eyre::install()?;

    // build a database pool, check the DB is alive, fail here if not
    let pool = build_pool(&config).await?;
    drop(pool.get().await?);

    // setup app context, all items should be clone-able or wrapped in Arc
    let ctx = AppState { pool };

    // build a router and attach endpoint and app state
    let addr = std::net::SocketAddr::from_str("127.0.0.1:8000")?;
    let router = axum::Router::new()
        .route("/fortunes", axum::routing::post(fortunes))
        .with_state(ctx);

    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    // run application
    tracing::info!("Listening on {addr}");
    server.await?;

    Ok(())
}
