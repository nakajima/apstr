use crate::error::AppResult;
use axum::response::Html;
use maud::{DOCTYPE, html};
use tracing::instrument;

#[instrument]
pub async fn get() -> AppResult<Html<String>> {
    let markup = html! {
        (DOCTYPE)
        html {
            head {
                title { "Sup" }
                link type="stylesheet" href="/assets/normalize.css";
                link type="stylesheet" href="/assets/style.css";
            }
            body {
                "Hello world. Hi. Greetings. Salutations."
            }
        }
    };

    Ok(Html(markup.into_string()))
}
