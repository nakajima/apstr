mod controllers;
mod error;
mod models;
mod views;

use anyhow::Context;
use axum::{
    Router,
    response::Redirect,
    routing::{get, post},
};
use error::AppResult;
use seekwel::{
    connection::Connection,
    schema::{ApplyMode, SchemaBuilder},
};
use tower_http::services::ServeDir;

use crate::models::app::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    Connection::file("apstr.sqlite").expect("could not init db");
    let plan = SchemaBuilder::new()
        .model::<App>()
        .plan()
        .expect("could not plan schema");

    if !plan.ops.is_empty() {
        tracing::info!("applying plan: {plan:?}");
    }

    plan.apply(ApplyMode::AllowDestructive)
        .expect("could not apply plan");

    let app = Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .route("/", get(controllers::apps::index))
        .route("/apps", get(Redirect::to("/")))
        .route("/apps/new", get(controllers::apps::new))
        .route("/apps", post(controllers::apps::create))
        .route("/_health", get(health));

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind = format!("0.0.0.0:{}", &port);

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding to {bind}"))?;

    tracing::info!("listening on 0.0.0.0:{}", port);
    axum::serve(listener, app).await.context("serving app")?;

    Ok(())
}

async fn health() -> AppResult<&'static str> {
    Ok("OK")
}
