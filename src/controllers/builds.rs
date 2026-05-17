use anyhow::Context;
use axum::{
    Form,
    extract::Path,
    response::{IntoResponse, Redirect},
};
use seekwel::PersistedModel;
use serde::Deserialize;

use crate::{
    error::AppResult,
    library::{app_store_connect::AppStoreConnectClient, hook_runner},
    models::{
        app::App,
        build::{Build, BuildColumns, Timestamp},
    },
};

#[derive(Debug, Deserialize)]
pub struct StartBuildParams {
    workflow_id: String,
    clean: Option<String>,
}

pub async fn create(
    Path(id): Path<u64>,
    Form(params): Form<StartBuildParams>,
) -> AppResult<impl IntoResponse> {
    let app = App::find(id).with_context(|| format!("loading app {id}"))?;
    let workflow = app
        .workflows()
        .with_context(|| format!("loading workflows for app {id}"))?
        .into_iter()
        .find(|workflow| workflow.asc_id == params.workflow_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "workflow {} does not belong to app {}",
                params.workflow_id,
                app.id
            )
        })?;

    if !workflow.can_start() {
        return Err(
            anyhow::anyhow!("workflow {} cannot be started", workflow.display_name()).into(),
        );
    }

    let client = AppStoreConnectClient::from_env().context("building App Store Connect client")?;
    let asc_build = client
        .start_build(&workflow.asc_id, params.clean.is_some())
        .await
        .with_context(|| format!("starting workflow {} for app {id}", workflow.asc_id))?;

    let build = Build::builder()
        .app(app.clone())
        .asc_id(asc_build.id)
        .number(asc_build.number)
        .created_date(
            asc_build
                .created_date
                .as_ref()
                .map(|v| v.parse::<Timestamp>())
                .transpose()?,
        )
        .started_date(
            asc_build
                .started_date
                .as_ref()
                .map(|v| v.parse::<Timestamp>())
                .transpose()?,
        )
        .finished_date(
            asc_build
                .finished_date
                .as_ref()
                .map(|v| v.parse::<Timestamp>())
                .transpose()?,
        )
        .execution_progress(asc_build.execution_progress)
        .completion_status(asc_build.completion_status)
        .start_reason(asc_build.start_reason)
        .cancel_reason(asc_build.cancel_reason)
        .create_or_update_by([BuildColumns::AscId])
        .context("saving started build")?;

    hook_runner::spawn_build_started(&app, &build, &workflow);

    Ok(Redirect::to(&format!("/apps/{id}")).into_response())
}
