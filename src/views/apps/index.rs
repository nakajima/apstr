use axum::response::Html;
use maud::html;

use crate::{
    error::AppResult,
    models::{app::App, build::BuildColumns},
    views::layout::page,
};

pub async fn index(apps: &[App]) -> AppResult<Html<String>> {
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
                                    small {
                                      a target="_blank" href=(format!(
                                          "https://appstoreconnect.apple.com/teams/{}/apps/{}/ci/builds/{}",
                                          env!("ASC_ISSUER_ID"),
                                          app.asc_id,
                                          build.asc_id
                                      )) {
                                        (format!("build #{}", number))
                                      }
                                      " - "
                                      @if let Some(result) = &build.completion_status {
                                        (result)
                                      } @else if let Some(progress) = &build.execution_progress {
                                        (progress)
                                      }
                                    }
                                }

                                " "
                                small { (app.bundle_identifier) }
                            }
                        }
                    }
                }
            },
        )
        .into_string(),
    ))
}
