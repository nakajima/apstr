use anyhow::Context;
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
    let asc_issuer_id = std::env::var("ASC_ISSUER_ID").context("loading ASC_ISSUER_ID")?;
    let test_flight_build = app.current_test_flight_build()?;
    let workflows = app.workflows_for_build_start()?;
    let builds = app.recent_builds(10)?;

    Ok(Html(
        page(
            &app.name,
            html! {
                main.vstack.gap-8 {
                    header.vstack.gap-2 {
                        h1.m-0 { (app.name) }
                        div.hstack.gap-4 {
                            code { (app.bundle_identifier) }
                            a href=(format!("/apps/{}/edit", app.id)) { "edit" }
                            (
                                button_to(
                                    "delete",
                                    format!("/apps/{}", app.id).as_str(),
                                    ButtonToOptions::default()
                                        .with_method(FormMethod::Delete)
                                        .confirm("Sure you want to delete this app?")
                                )
                            )
                        }
                    }

                    @if let Some(sync_error) = &app.sync_error {
                        section.vstack.gap-2 {
                            h2.m-0 { "Sync error" }
                            p.m-0 { (sync_error) }
                        }
                    }

                    section.vstack.gap-4 {
                        h2.m-0 { "Xcode Cloud" }

                        div.vstack.gap-2 {
                            p.m-0 {
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
                            form.vstack.gap-2 action=(format!("/apps/{}/hook-script", app.id)) method="post" {
                                label {
                                    "hook script"
                                    input type="text" name="hook_script" value=(app.hook_script.as_deref().unwrap_or(""));
                                }
                                div {
                                    button type="submit" { "Save hook script" }
                                }
                            }
                            @if let Some(requested_at) = app.auto_build_requested_at {
                                small.subdue { "last automatic build requested: " (requested_at.utc_date()) }
                            }
                            @if let Some(auto_build_error) = &app.auto_build_error {
                                p.m-0 { strong { "auto-build error: " } (auto_build_error) }
                            }
                        }

                        @if workflows.is_empty() {
                            p.m-0 { "No workflows synced yet" }
                        } @else {
                            div.vstack.gap-4 {
                                @for workflow in &workflows {
                                    form.vstack.gap-2 action=(format!("/apps/{}/builds", app.id)) method="post" {
                                        input type="hidden" name="workflow_id" value=(workflow.asc_id.as_str());
                                        div {
                                            strong { (workflow.display_name()) }
                                            @if let Some(description) = &workflow.description {
                                                " " small.subdue { (description) }
                                            }
                                        }
                                        label {
                                            input type="checkbox" name="clean" value="1";
                                            " clean build"
                                        }
                                        div {
                                            @if workflow.can_start() {
                                                button type="submit" { "Start build" }
                                            } @else {
                                                button type="submit" disabled { "Start build" }
                                                " " small.subdue { "workflow disabled or locked" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    section.vstack.gap-4 {
                        h2.m-0 { "TestFlight" }
                        @if let Some(test_flight_build) = &test_flight_build {
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
                            p.m-0 { "No TestFlight build" }
                        }
                    }

                    section.vstack.gap-4 {
                        h2.m-0 { "Builds" }
                        @if builds.is_empty() {
                            p.m-0 { "No builds synced yet" }
                        } @else {
                            ol.vstack.gap-4 {
                                @for build in &builds {
                                    li.vstack.gap-2 {
                                        div.hstack.gap-4 {
                                            strong { (build.display_name()) }
                                            span { (build.status()) }
                                            a target="_blank" href=(format!(
                                                "https://appstoreconnect.apple.com/teams/{}/apps/{}/ci/builds/{}",
                                                asc_issuer_id,
                                                app.asc_id,
                                                build.asc_id
                                            )) { "open" }
                                        }
                                        small.subdue.hstack.gap-4 {
                                            span {
                                                "created: "
                                                @if let Some(created_date) = build.created_date {
                                                    (created_date.utc_date())
                                                } @else {
                                                    "unknown"
                                                }
                                            }
                                            span {
                                                "started: "
                                                @if let Some(started_date) = build.started_date {
                                                    (started_date.utc_date())
                                                } @else {
                                                    "unknown"
                                                }
                                            }
                                            span {
                                                "finished: "
                                                @if let Some(finished_date) = build.finished_date {
                                                    (finished_date.utc_date())
                                                } @else {
                                                    "unknown"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    p.m-0 { a href="/" { "back home" } }
                }
            },
        )
        .into_string(),
    ))
}
