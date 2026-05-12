use anyhow::Context;
use axum::{
    Form,
    extract::Path,
    response::{IntoResponse, Redirect},
};
use seekwel::PersistedModel;
use serde::Deserialize;

use crate::{
    error::AppResult, library::app_store_connect::AppStoreConnectClient, models::app::App,
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
    let client = AppStoreConnectClient::from_env().context("building App Store Connect client")?;
    let overview = client
        .overview_for_bundle_id(&app.bundle_identifier)
        .await
        .with_context(|| format!("loading App Store Connect overview for app {id}"))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no App Store Connect app found for bundle identifier {}",
                app.bundle_identifier
            )
        })?;

    if !overview
        .workflows
        .iter()
        .any(|workflow| workflow.id == params.workflow_id)
    {
        return Err(anyhow::anyhow!(
            "workflow {} does not belong to app {}",
            params.workflow_id,
            app.id
        )
        .into());
    }

    client
        .start_build(&params.workflow_id, params.clean.is_some())
        .await
        .with_context(|| format!("starting workflow {} for app {id}", params.workflow_id))?;

    Ok(Redirect::to(&format!("/apps/{id}")).into_response())
}
