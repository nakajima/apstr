use axum::response::Html;
use maud::html;
use seekwel_forms::FormMethod;

use crate::{
    error::AppResult,
    helpers::button_to::{ButtonToOptions, button_to},
    models::app::App,
    views::layout::page,
};

pub async fn show(app: &App) -> AppResult<Html<String>> {
    Ok(Html(
        page(
            &app.name,
            html! {
                h1 { (app.name) }
                div {
                    a href=(format!("/apps/{}/edit", app.id)) { "edit "}
                    " | "
                    (
                        button_to(
                            "delete",
                            format!("/apps/{}", app.id).as_str(),
                            ButtonToOptions::default()
                                .with_method(FormMethod::Delete)
                                .confirm("Sure you want to delete this app?")
                        )
                    );
                }
                p { code { (app.bundle_identifier) } }
                p { a href="/" { "back home" } }
            },
        )
        .into_string(),
    ))
}
