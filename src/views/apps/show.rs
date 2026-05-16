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
                @if let Some(sync_error) = &app.sync_error {
                    p { strong { "sync error: " } (sync_error) }
                }

                @if let Some(test_flight_build) = app.current_test_flight_build()? {
                    h2 { "TestFlight" }
                    dl {
                        dt { "version" }
                        dd {
                            @if let Some(version) = &test_flight_build.version {
                                (version)
                            } @else {
                                "unknown"
                            }
                        }

                        dt { "state" }
                        dd {
                            @if let Some(state) = &test_flight_build.processing_state {
                                (state)
                            } @else {
                                "unknown"
                            }
                        }

                        dt { "uploaded" }
                        dd {
                            @if let Some(uploaded_date) = test_flight_build.uploaded_date {
                                (uploaded_date.utc_date())
                            } @else {
                                "unknown"
                            }
                        }

                        dt { "expiration" }
                        dd {
                            (test_flight_build.expiration_status())
                            @if let Some(expiration_date) = test_flight_build.expiration_date {
                                " (" (expiration_date.utc_date()) ")"
                            }
                        }
                    }
                } @else {
                    p { "No TestFlight build" }
                }

                p { a href="/" { "back home" } }
            },
        )
        .into_string(),
    ))
}
