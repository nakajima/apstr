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
    let apps = App::order(AppColumns::BuiltAt.desc())
        .all()
        .context("loading apps for index page")?;
    views::apps::index(&apps).await
}

pub async fn new() -> AppResult<Html<String>> {
    let app = App::builder()
        .name("")
        .bundle_identifier("")
        .build()
        .context("building empty app form")?;
    views::apps::new(&app).await
}

pub async fn show(Path(id): Path<u64>) -> AppResult<Html<String>> {
    let app = App::find(id).with_context(|| format!("loading app {id}"))?;
    app.clone().refresh().await?;

    views::apps::show(&app).await
}

pub async fn destroy(Path(id): Path<u64>) -> AppResult<Redirect> {
    let app = App::find(id).with_context(|| format!("loading app {id}"))?;
    app.delete().with_context(|| format!("deleting app {id}"))?;

    Ok(Redirect::to("/apps"))
}

pub async fn create(Form(params): Form<AppParams>) -> AppResult<impl IntoResponse> {
    let app = App::new(params.allow([AppColumns::Name, AppColumns::BundleIdentifier]))
        .context("building app from form params")?;
    match app.save() {
        Ok(_) => Ok(Redirect::to("/apps").into_response()),
        Err(e) => Ok(views::apps::new(
            &e.into_invalid()
                .unwrap_or_else(|| unreachable!("we're in the error case")),
        )
        .await
        .into_response()),
    }
}
