use axum::response::Html;
use maud::html;
use seekwel::ModelRecord;
use seekwel_forms::form_for;

use crate::{error::AppResult, models::app::App, views::layout::page};

pub async fn index(apps: &[App]) -> AppResult<Html<String>> {
    Ok(Html(
        page(
            "apps",
            html! {
                h1 { "apps" }
                a href="/apps/new" { "add" }
                @for app in apps {
                    div {
                        (app.name)
                    }
                }
            },
        )
        .into_string(),
    ))
}

pub async fn new<A: ModelRecord>(app: &A) -> AppResult<Html<String>> {
    Ok(Html(
        page(
            "new app",
            html! {
                div class="v-gap:4" {
                    h1 { "new app" };
                    (form_for(app));
                    a href="/" { "back home" }
                };
            },
        )
        .into_string(),
    ))
}
