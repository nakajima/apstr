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
    let test_flight_build = app.current_test_flight_build()?;
    let workflows = app.workflows_for_build_start()?;

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

                h2 { "Xcode Cloud" }
                p {
                    "automatic builds: "
                    strong {
                        @if app.auto_builds_enabled() {
                            "enabled"
                        } @else {
                            "disabled"
                        }
                    }
                }
                form action=(format!("/apps/{}/auto-build", app.id)) method="post" {
                    @if app.auto_builds_enabled() {
                        input type="hidden" name="enabled" value="0";
                        button type="submit" { "Disable automatic builds" }
                    } @else {
                        input type="hidden" name="enabled" value="1";
                        button type="submit" { "Enable automatic builds" }
                    }
                }
                @if workflows.is_empty() {
                    p { "No workflows synced yet" }
                } @else {
                    div.vstack.gap-2 {
                        @for workflow in &workflows {
                            form action=(format!("/apps/{}/builds", app.id)) method="post" {
                                input type="hidden" name="workflow_id" value=(workflow.asc_id.as_str());
                                div {
                                    strong { (workflow.display_name()) }
                                    @if let Some(description) = &workflow.description {
                                        " " small { (description) }
                                    }
                                }
                                label {
                                    input type="checkbox" name="clean" value="1";
                                    " clean build"
                                }
                                " "
                                @if workflow.can_start() {
                                    button type="submit" { "Start build" }
                                } @else {
                                    button type="submit" disabled { "Start build" }
                                    " " small { "workflow disabled or locked" }
                                }
                            }
                        }
                    }
                }
                @if let Some(requested_at) = app.auto_build_requested_at {
                    p { small { "last automatic build requested: " (requested_at.utc_date()) } }
                }
                @if let Some(auto_build_error) = &app.auto_build_error {
                    p { strong { "auto-build error: " } (auto_build_error) }
                }

                @if let Some(test_flight_build) = &test_flight_build {
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
