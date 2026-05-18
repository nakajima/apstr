use anyhow::Context;
use axum::{
    Form,
    extract::Path,
    response::{Html, IntoResponse, Redirect},
};
use seekwel::*;

use crate::{
    error::AppResult,
    models::app::{App, AppColumns, AppParams},
    views,
};

pub async fn index() -> AppResult<Html<String>> {
    let apps = App::order([AppColumns::Archived.asc(), AppColumns::BuiltAt.desc()]).all()?;

    views::apps::index(&apps).await
}

pub async fn show(Path(id): Path<u64>) -> AppResult<Html<String>> {
    let app = App::find(id)?;

    views::apps::show(&app).await
}

pub async fn destroy(Path(id): Path<u64>) -> AppResult<Redirect> {
    let app = App::find(id)?;
    app.delete()?;

    Ok(Redirect::to("/apps"))
}

pub async fn update(
    Path(id): Path<u64>,
    Form(params): Form<AppParams>,
) -> AppResult<impl IntoResponse> {
    let mut app = App::find(id).with_context(|| format!("loading app {id}"))?;
    match app.update(params.allow([
        AppColumns::AutoBuildEnabled,
        AppColumns::HookScript,
        AppColumns::Archived,
    ])) {
        Ok(_) => Ok(Redirect::to(&format!("/apps/{}", app.id)).into_response()),
        Err(e) => Ok(views::apps::new(
            &e.into_invalid()
                .unwrap_or_else(|| unreachable!("we're in the error case")),
        )
        .await
        .into_response()),
    }
}
