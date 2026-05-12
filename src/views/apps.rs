use axum::response::Html;
use maud::html;
use seekwel::ModelRecord;
use seekwel_forms::{FormMethod, form_for};

use crate::{
    error::AppResult,
    helpers::button_to::{ButtonToOptions, button_to},
    models::app::App,
    views::layout::page,
};

pub async fn index(apps: &[App]) -> AppResult<Html<String>> {
    Ok(Html(
        page(
            "apps",
            html! {
                main.vstack.gap-4 {
                    h1 { "apps" }
                    a href="/apps/new" { "add" }
                    div.vstack.gap-4 {
                        @for app in apps {
                            div.vstack.gap-2 {
                                a href=(format!("/apps/{}", app.id)) {
                                    h3.m-0 { (app.name) };
                                }

                                @if let Some(build) = app.builds()?.first() && let Some(number) = build.number {
                                    small { (format!("build #{}", number)) }
                                }

                                " "
                                code { (app.bundle_identifier) }
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
                            }
                        }
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

pub async fn show(app: &App) -> AppResult<Html<String>> {
    Ok(Html(
        page(
            &app.name,
            html! {
                h1 { (app.name) }
                p { code { (app.bundle_identifier) } }
                p { a href="/" { "back home" } }
            },
        )
        .into_string(),
    ))
}
