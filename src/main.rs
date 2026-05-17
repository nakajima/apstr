mod controllers;
mod error;
mod helpers;
mod library;
mod models;
mod views;

#[cfg(not(debug_assertions))]
mod embedded_assets {
    include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));
}

use anyhow::Context;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{RawPathParams, Request, rejection::RawPathParamsRejection},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post},
};
use error::AppResult;
use seekwel::{
    connection::Connection,
    schema::{ApplyMode, SchemaBuilder},
};
#[cfg(debug_assertions)]
use tower_http::services::ServeDir;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tower_method_hax::axum::MethodOverrideExt;

use crate::models::{
    app::App, build::Build, test_flight_build::TestFlightBuild, workflow::Workflow,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_path = std::env::var("DATABASE_PATH").unwrap_or("apstr.sqlite".to_string());

    Connection::file(&database_path).expect("could not init db");
    let plan = SchemaBuilder::new()
        .model::<App>()
        .model::<Build>()
        .model::<TestFlightBuild>()
        .model::<Workflow>()
        .plan()
        .expect("could not plan schema");

    if !plan.ops.is_empty() {
        tracing::info!("applying plan: {plan:?}");
    }

    plan.apply(ApplyMode::AllowDestructive)
        .expect("could not apply plan");

    tracing::info!("starting syncer");
    let syncer = crate::library::syncer::Syncer::new().context("initializing syncer")?;
    tokio::spawn(syncer.start());

    let app = Router::new()
        .merge(asset_routes())
        .route("/", get(controllers::apps::index))
        .route("/apps", get(Redirect::to("/")))
        .route("/apps/new", get(controllers::apps::new))
        .route("/apps/{id}", get(controllers::apps::show))
        .route("/apps/{id}", delete(controllers::apps::destroy))
        .route(
            "/apps/{id}/auto-build",
            post(controllers::apps::update_auto_build),
        )
        .route("/apps/{id}/builds", post(controllers::builds::create))
        .route("/apps", post(controllers::apps::create))
        .route("/_health", get(health))
        .layer(middleware::from_fn(log_request_params))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO))
                .on_failure(()),
        )
        .with_method_override();

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind = format!("0.0.0.0:{}", &port);

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding to {bind}"))?;

    tracing::info!("listening on 0.0.0.0:{}", port);
    axum::serve(listener, app).await.context("serving app")?;

    Ok(())
}

#[cfg(debug_assertions)]
fn asset_routes() -> Router {
    Router::new().nest_service("/assets", ServeDir::new("assets"))
}

#[cfg(not(debug_assertions))]
fn asset_routes() -> Router {
    Router::new().route("/assets/{*path}", get(embedded_asset))
}

#[cfg(not(debug_assertions))]
async fn embedded_asset(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    match embedded_assets::ASSETS
        .iter()
        .find(|asset| asset.path == path)
    {
        Some(asset) => {
            let mut response = Body::from(asset.content).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(asset_content_type(asset.path)),
            );
            response
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(not(debug_assertions))]
fn asset_content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("css") => "text/css; charset=utf-8",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

async fn health() -> AppResult<&'static str> {
    Ok("OK")
}

const MAX_LOGGED_FORM_BYTES: usize = 8 * 1024;

async fn log_request_params(
    path_params: Result<RawPathParams, RawPathParamsRejection>,
    request: Request,
    next: Next,
) -> Response {
    let path_params = path_params
        .map(|params| {
            params
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let query_params = request
        .uri()
        .query()
        .map(parse_urlencoded_params)
        .unwrap_or_default();

    let mut form_params = Vec::new();
    let mut form_params_skipped = None;
    let request = if should_log_form_params(request.headers()) {
        match content_length(request.headers()) {
            Some(length) if length <= MAX_LOGGED_FORM_BYTES => {
                let (parts, body) = request.into_parts();
                let bytes = match to_bytes(body, MAX_LOGGED_FORM_BYTES).await {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        tracing::warn!(error = %error, "failed to read form params for logging");
                        return (StatusCode::BAD_REQUEST, "invalid request body").into_response();
                    }
                };

                form_params = parse_urlencoded_params(std::str::from_utf8(&bytes).unwrap_or(""));
                Request::from_parts(parts, Body::from(bytes))
            }
            Some(length) => {
                form_params_skipped = Some(format!(
                    "content length {length} exceeds logging limit {MAX_LOGGED_FORM_BYTES}"
                ));
                request
            }
            None => {
                form_params_skipped = Some("missing content length".to_string());
                request
            }
        }
    } else {
        request
    };

    if !path_params.is_empty()
        || !query_params.is_empty()
        || !form_params.is_empty()
        || form_params_skipped.is_some()
    {
        tracing::info!(
            path_params = ?path_params,
            query_params = ?query_params,
            form_params = ?form_params,
            form_params_skipped = ?form_params_skipped,
            "request params"
        );
    }

    next.run(request).await
}

fn should_log_form_params(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| {
            value
                .trim()
                .eq_ignore_ascii_case("application/x-www-form-urlencoded")
        })
}

fn content_length(headers: &HeaderMap) -> Option<usize> {
    headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

fn parse_urlencoded_params(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut pair = part.splitn(2, '=');
            let key = decode_urlencoded(pair.next().unwrap_or(""));
            let value = decode_urlencoded(pair.next().unwrap_or(""));
            (key, value)
        })
        .collect()
}

fn decode_urlencoded(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                if let (Some(high), Some(low)) =
                    (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
                {
                    decoded.push((high << 4) | low);
                    index += 3;
                } else {
                    decoded.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
