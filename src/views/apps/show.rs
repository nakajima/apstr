use anyhow::Context;
use axum::response::Html;
use maud::html;
use seekwel_forms::{FormMethod, form_for};

use crate::{
    error::AppResult,
    helpers::button_to::{ButtonToOptions, button_to},
    models::app::{App, AppColumns},
    views::layout::page,
};

pub async fn show(app: &App) -> AppResult<Html<String>> {
    let asc_issuer_id = std::env::var("ASC_ISSUER_ID").context("loading ASC_ISSUER_ID")?;
    let test_flight_build = app.current_test_flight_build()?;
    let workflows = app.workflows_for_build_start()?;
    let builds = app.recent_builds(10)?;
    let hook_runs = app.recent_hook_runs(10)?;

    Ok(Html(
        page(
            &format!("apps / {}", app.name),
            html! {
                main.vstack.gap-8 {
                    header.vstack.gap-2 {
                        div.hstack.gap-4 {
                            code { (app.bundle_identifier) }
                            (
                                button_to(
                                    "delete",
                                    format!("/apps/{}", app.id).as_str(),
                                    ButtonToOptions::default()
                                        .with_method(FormMethod::Delete)
                                        .confirm("Sure you want to delete this app?")
                                )
                            )

                            (
                                form_for(app).method("PATCH").fields(|f| {
                                    html! {
                                        @if app.archived {
                                            (f.hidden_field(AppColumns::Archived).attr("value", "0"))
                                            (f.submit("unarchive").class("link"))
                                        } @else {
                                            (f.hidden_field(AppColumns::Archived).attr("value", "1"))
                                            (f.submit("archive").class("link"))
                                        }
                                        
                                    }
                                })
                                // button_to(
                                //     ,
                                //     format!("/apps/{}?app[archived]={}", app.id, if app.archived { "0" } else { "1" }).as_str(),
                                //     ButtonToOptions::default()
                                //         .with_method(FormMethod::Patch)
                                // )
                            )
                        }
                    }

                    @if let Some(sync_error) = &app.sync_error {
                        section.vstack.gap-2 {
                            h2.m-0 { "Sync error" }
                            p.m-0 { (sync_error) }
                        }
                    }

                    section.vstack.gap-2 {
                        h2.m-0 { "Hook script" }

                        div.hstack.gap-4.space-evenly {
                            (form_for(app).class("grow vstack gap-2").fields(|f| {
                                html! {
                                    (f.label(AppColumns::HookScript, "Script Source"))
                                    (f.textarea(AppColumns::HookScript).class("w-full flex-1").attr("rows", "10").attr("placeholder", "Enter a shell script here."))
                                    (f.field_errors(AppColumns::HookScript))
                                    div {
                                        (f.submit("Save"))
                                    }
                                }
                            }))

                            @if hook_runs.is_empty() {
                                p.grow.m-0 { "No hook runs yet" }
                            } @else {
                                ol.grow.vstack.gap-4 {
                                    @for run in &hook_runs {
                                        li.vstack.gap-2 {
                                            div.hstack.gap-4 {
                                                strong { (run.event_label.as_str()) }
                                                span { (run.status()) }
                                                small.subdue { (run.started_at.utc_datetime()) }
                                            }
                                            small.subdue {
                                                "event: " (run.event.as_str())
                                                @if let Some(exit_code) = run.exit_code {
                                                    " exit: " (exit_code)
                                                }
                                                @if let Some(finished_at) = run.finished_at {
                                                    " finished: " (finished_at.utc_datetime())
                                                }
                                            }
                                            small.subdue { "command: " (run.command.as_str()) }
                                            @if let Some(error) = &run.error {
                                                div { strong { "error: " } (error) }
                                            }
                                            @if let Some(stdout) = &run.stdout {
                                                div.vstack.gap-2 {
                                                    small.subdue { "stdout" }
                                                    pre { (stdout) }
                                                }
                                            }
                                            @if let Some(stderr) = &run.stderr {
                                                div.vstack.gap-2 {
                                                    small.subdue { "stderr" }
                                                    pre { (stderr) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    section.vstack.gap-2 {
                        h2.m-0 { "Xcode Cloud" }

                        div.vstack.gap-2.mb-8 {
                            (form_for(app).class("vstack gap-2").fields(|f| {
                                html! {
                                    (f.label(AppColumns::AutoBuildEnabled, html! {
                                        (f.checkbox(AppColumns::AutoBuildEnabled))
                                        " Automatically prevent TestFlight expiration"
                                    }))
                                    (f.field_errors(AppColumns::AutoBuildEnabled))
                                    div.hstack.gap-2 {
                                        (f.submit("Save"))

                                        @if let Some(requested_at) = app.auto_build_requested_at {
                                            small.subdue { "last automatic build requested: " (requested_at.utc_date()) }
                                        }
                                        @if let Some(auto_build_error) = &app.auto_build_error {
                                            p.m-0 { strong { "auto-build error: " } (auto_build_error) }
                                        }
                                    }
                                }
                            }))
                        }

                        h3 { "Workflows"}

                        @if workflows.is_empty() {
                            p.m-0 { "No workflows synced yet" }
                        } @else {
                            div.vstack.gap-4 {
                                @for workflow in &workflows {
                                    form.vstack action=(format!("/apps/{}/builds", app.id)) method="post" {
                                        input type="hidden" name="workflow_id" value=(workflow.asc_id.as_str());
                                       
                                        div.hstack.gap-2 {
                                            strong { (workflow.display_name()) }
                                            @if let Some(description) = &workflow.description {
                                                " " small.subdue { (description) }
                                            }

                                            @if workflow.can_start() {
                                                button type="submit" { "Start build" }
                                            } @else {
                                                button type="submit" disabled { "Start build" }
                                                " " small.subdue { "workflow disabled or locked" }
                                            }

                                            label.inline {
                                                input type="checkbox" name="clean" value="1";
                                                " Clean build"
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
                            (test_flight_build.expiration_status())
                            @if let Some(expiration_date) = test_flight_build.expiration_date {
                                " (" (expiration_date.utc_date()) ")"
                            }
                            dl.vstack.gap-2 {
                                div {
                                    dt.inline.subdue { "Version " }
                                    dd.inline {
                                        @if let Some(version) = &test_flight_build.version {
                                            (version)
                                        } @else {
                                            "unknown"
                                        }
                                    }
                                }

                                div {
                                    dt.inline.subdue { "State " }
                                    dd.inline {
                                        @if let Some(state) = &test_flight_build.processing_state {
                                            (state)
                                        } @else {
                                            "unknown"
                                        }
                                    }
                                }

                                div {
                                    dt.inline.subdue { "Uploaded at " }
                                    dd.inline {
                                        @if let Some(uploaded_date) = test_flight_build.uploaded_date {
                                            (uploaded_date.utc_date())
                                        } @else {
                                            "unknown"
                                        }
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
