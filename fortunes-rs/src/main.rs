use std::str::FromStr;

use axum::{extract::State, Json};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio_postgres::Transaction;

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env = "DATABASE_URL")]
    pub database_url: String,

    #[clap(long, env = "DATABASE_POOL_MAX_SIZE", default_value = "4")]
    pub database_pool_max_size: i64,
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
    pub count: i64,
    pub items: Vec<Fortune>,
}

async fn build_pool(config: &Config) -> deadpool_postgres::Pool {
    let database_url = url::Url::from_str(&config.database_url).unwrap();

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

    builder
        .create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .unwrap()
}

async fn get_fortunes(db: &Transaction<'_>, count: i64) -> Vec<Fortune> {
    db.query(
        "SELECT content FROM fortune ORDER BY random() LIMIT $1",
        &[&count],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| Fortune {
        content: row.get("content"),
    })
    .collect()
}

async fn fortunes(
    State(state): State<AppState>,
    request: Json<FortunesListRequest>,
) -> Json<FortunesListResponse> {
    let mut conn = state.pool.get().await.unwrap();
    let db = conn.transaction().await.unwrap();

    // grab stuff from the database
    let items = get_fortunes(&db, request.quantity.unwrap_or(5)).await;

    // ensure you commit here, otherwise when `db` is dropped, it will rollback
    db.commit().await.unwrap();

    Json(FortunesListResponse {
        count: items.len() as _,
        items,
    })
}

#[tokio::main]
async fn main() {
    // load .env and parse into Config
    dotenvy::dotenv().ok();
    let config = Config::parse();
    tracing_subscriber::fmt::init();

    // build a database pool
    let pool = build_pool(&config).await;

    // check the DB is alive, fail here if not
    drop(pool.get().await.unwrap());

    // setup app context, all items should be clone-able or wrapped in Arc
    let ctx = AppState { pool };

    // build a router and attach endpoint and app state
    let addr = std::net::SocketAddr::from_str("127.0.0.1:8001").unwrap();
    let router = axum::Router::new()
        .route("/fortunes", axum::routing::post(fortunes))
        .with_state(ctx);

    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    // run application
    tracing::info!("Listening on {addr}");
    server.await.unwrap();
}
