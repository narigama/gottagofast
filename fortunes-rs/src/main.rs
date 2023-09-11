use std::str::FromStr;

use axum::Json;
use clap::Parser;
use deadpool_postgres::PoolConfig;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Connection, NoTls};

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env = "DATABASE_URL")]
    pub database_url: String,

    #[clap(long, env = "DATABASE_POOL_MIN_SIZE", default_value = "1")]
    pub database_pool_min_size: i64,

    #[clap(long, env = "DATABASE_POOL_MAX_SIZE", default_value = "4")]
    pub database_pool_max_size: i64,
}

#[derive(Debug, Clone)]
pub struct State {
    pub db: deadpool_postgres::Pool,
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

async fn get_fortunes(db: &tokio_postgres::Transaction<'_>, count: i64) -> Vec<Fortune> {
    let query = "SELECT content FROM fortune ORDER BY random() LIMIT $1";
    let rows = db.query(query, &[&count]).await.unwrap();

    let fortunes = rows
        .iter()
        .map(|row| Fortune {
            content: row.get("content"),
        })
        .collect::<Vec<_>>();

    fortunes
}

async fn fortunes(
    state: axum::Extension<State>,
    request: Json<FortunesListRequest>,
) -> Json<FortunesListResponse> {
    // grab a connection from the pool and begin a transaction
    let mut conn = state.db.get().await.unwrap();
    let db = conn.transaction().await.unwrap();

    // grab stuff from the database
    let items = get_fortunes(&db, request.quantity.unwrap_or(5)).await;

    // ensure you commit here, otherwise when `db` is dropped, it rolls back
    db.commit().await.unwrap();

    Json(FortunesListResponse {
        count: items.len() as _,
        items,
    })
}

async fn build_pool(config: &Config) -> deadpool_postgres::Pool {
    let database_url = url::Url::parse(&config.database_url).unwrap();

    let mut builder = deadpool_postgres::Config::new();
    builder.manager = Some(deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Fast,
    });

    builder.pool = Some(PoolConfig {
        max_size: config.database_pool_max_size as _,
        ..Default::default()
    });

    builder.host = database_url.host_str().map(String::from);
    builder.port = database_url.port().or(Some(5432));
    builder.user = Some(database_url.username().into());
    builder.password = database_url.password().map(String::from);
    builder.dbname = Some(database_url.path().trim_start_matches('/').into());

    dbg!(&builder);

    builder
        .create_pool(Some(deadpool_postgres::Runtime::Tokio1), NoTls)
        .unwrap()
}

#[tokio::main]
async fn main() {
    // load .env and parse into Config
    dotenvy::dotenv().ok();
    let config = Config::parse();
    tracing_subscriber::fmt::init();

    // let db = sqlx::postgres::PgPoolOptions::default()
    //     .min_connections(config.database_pool_min_size as _)
    //     .max_connections(config.database_pool_max_size as _)
    //     .connect(&config.database_url)
    //     .await
    //     .unwrap();

    let db = build_pool(&config).await;

    // setup app state, all items should be clone-able or wrapped in Arc
    let state = State { db };

    // build a router and attach endpoint and app state
    let addr = std::net::SocketAddr::from_str("127.0.0.1:8001").unwrap();
    let router = axum::Router::new()
        .route("/fortunes", axum::routing::post(fortunes))
        .layer(axum::Extension(state));

    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    // run application
    tracing::info!("Listening on {addr}");
    server.await.unwrap();
}
