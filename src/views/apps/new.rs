use axum::response::Html;
use maud::html;
use seekwel::ModelRecord;
use seekwel_forms::form_for;

use crate::{error::AppResult, views::layout::page};

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
