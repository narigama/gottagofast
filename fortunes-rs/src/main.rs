use std::str::FromStr;

use axum::Json;
use clap::Parser;
use serde::{Serialize, Deserialize};

#[derive(Debug, Parser)]
pub struct Config {
    #[clap(long, env = "DATABASE_URL")]
    pub database_url: String,
}

#[derive(Debug, Clone)]
pub struct State {
    pub db: sqlx::PgPool,
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

async fn get_fortunes(db: &mut sqlx::Transaction<'_, sqlx::Postgres>, count: i64) -> Vec<Fortune> {
    sqlx::query_as!(
        Fortune,
        "SELECT content FROM fortune ORDER BY random() LIMIT $1",
        count
    )
    .fetch_all(&mut **db)
    .await
    .unwrap()
}

async fn fortunes(
    state: axum::Extension<State>,
    request: Json<FortunesListRequest>,
) -> Json<FortunesListResponse> {
    // grab a connection from the pool and begin a transaction
    let mut db = state.db.begin().await.unwrap();

    // grab stuff from the database
    let items = get_fortunes(&mut db, request.quantity.unwrap_or(5)).await;

    // ensure you commit here, otherwise when `db` is dropped, it rolls back
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

    // setup app state, all items should be clone-able or wrapped in Arc
    let state = State {
        db: sqlx::PgPool::connect(&config.database_url).await.unwrap(),
    };

    // build a router and attach endpoint and app state
    let addr = std::net::SocketAddr::from_str("127.0.0.1:8000").unwrap();
    let router = axum::Router::new()
        .route("/fortunes", axum::routing::post(fortunes))
        .layer(axum::Extension(state));

    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    // run application
    tracing::info!("Listening on {addr}");
    server.await.unwrap();
}
