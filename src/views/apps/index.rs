use anyhow::Context;
use axum::response::Html;
use maud::html;

use crate::{error::AppResult, models::app::App, views::layout::page};

pub async fn index(apps: &[App]) -> AppResult<Html<String>> {
    let asc_issuer_id = std::env::var("ASC_ISSUER_ID").context("loading ASC_ISSUER_ID")?;

    Ok(Html(
        page(
            "apps",
            html! {
                main.vstack.gap-4 {
                    h1 { "apps" }
                    div.vstack.gap-8 {
                        @for app in apps {
                            div.vstack.gap-2 {
                                a href=(format!("/apps/{}", app.id)) {
                                    h3.m-0 { (app.name) };
                                }

                                @if let Some(build) = app.latest_build()? && let Some(number) = build.number {
                                    small.subdue.hstack.middle.gap-2 {
                                        @if build.completion_status == Some("FAILED".into()) {
                                            ion-icon name="alert" aria-hidden="true" {};
                                        } @else if build.completion_status == Some("SUCCEEDED".into()) {
                                            ion-icon name="checkmark" aria-hidden="true" {};
                                        } @else {
                                            ion-icon name="ellipsis-horizontal" aria-hidden="true" {};
                                        }

                                        a target="_blank" class="subdue" href=(format!(
                                            "https://appstoreconnect.apple.com/teams/{}/apps/{}/ci/builds/{}",
                                            asc_issuer_id,
                                            app.asc_id,
                                            build.asc_id
                                        )) {
                                            (format!("build #{}", number))
                                        }
                                        @if let Some(result) = &build.completion_status {
                                            (result)
                                        } @else if let Some(progress) = &build.execution_progress {
                                            (progress)
                                        }

                                        @if let Some(test_flight_build) = app.current_test_flight_build()? {
                                            @if test_flight_build.is_valid() {
                                                span class=(if test_flight_build.expiration_status() == "Expired" { "error" } else { "" }) {
                                                    (test_flight_build.expiration_status())
                                                }
                                            }
                                        }
                                    }
                                }

                                " "
                                small { (app.bundle_identifier) }

                                @if let Some(sync_error) = &app.sync_error {
                                    small { "sync error: " (sync_error) }
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
