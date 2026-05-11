use axum::{
    Form,
    response::{Html, IntoResponse, Redirect},
};
use seekwel::*;

use crate::{
    error::AppResult,
    models::app::{App, AppColumns, AppParams},
    views,
};

pub async fn index() -> AppResult<Html<String>> {
    let apps = App::all()?;
    views::apps::index(&apps).await
}

pub async fn new() -> AppResult<Html<String>> {
    let app = App::builder().name("").bundle_identifier("").build()?;
    views::apps::new(&app).await
}

pub async fn create(Form(params): Form<AppParams>) -> AppResult<impl IntoResponse> {
    let app = App::new(params.allow([AppColumns::Name, AppColumns::BundleIdentifier]))?;
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
