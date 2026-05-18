use std::{collections::BTreeMap, process::Stdio, time::Duration};

use seekwel::PersistedModel;
use tokio::{process::Command, time};

use crate::models::{
    app::App,
    build::{Build, Timestamp},
    hook_run::HookRun,
    test_flight_build::TestFlightBuild,
    workflow::Workflow,
};

const HOOK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_CAPTURED_OUTPUT_BYTES: usize = 64 * 1024;

pub fn spawn_build_started(app: &App, build: &Build, workflow: &Workflow) {
    let mut env = hook_env(app, "build_started", "Build started");
    insert_build_env(&mut env, app, build);
    insert_workflow_env(&mut env, workflow);
    spawn_hook(app, env);
}

pub fn spawn_build_auto_started(app: &App, build: &Build, workflow: &Workflow) {
    let mut env = hook_env(app, "build_auto_started", "Automatic build started");
    insert_build_env(&mut env, app, build);
    insert_workflow_env(&mut env, workflow);
    spawn_hook(app, env);
}

pub fn spawn_build_completed(app: &App, build: &Build) {
    let mut env = hook_env(app, "build_completed", "Build completed");
    insert_build_env(&mut env, app, build);
    spawn_hook(app, env);
}

pub fn spawn_testflight_expired(app: &App, build: &TestFlightBuild) {
    let mut env = hook_env(app, "testflight_expired", "TestFlight expired");
    insert_testflight_env(&mut env, build);
    spawn_hook(app, env);
}

pub fn spawn_testflight_expiring(app: &App, build: &TestFlightBuild) {
    let mut env = hook_env(app, "testflight_expiring", "TestFlight expiring");
    insert_testflight_env(&mut env, build);
    spawn_hook(app, env);
}

fn hook_env(app: &App, event: &str, label: &str) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("APSTR_EVENT".to_string(), event.to_string());
    env.insert("APSTR_EVENT_LABEL".to_string(), label.to_string());
    env.insert("APSTR_APP_ID".to_string(), app.id.to_string());
    env.insert("APSTR_APP_NAME".to_string(), app.name.clone());
    env.insert(
        "APSTR_BUNDLE_IDENTIFIER".to_string(),
        app.bundle_identifier.clone(),
    );
    env.insert("APSTR_ASC_APP_ID".to_string(), app.asc_id.clone());

    if let Some(url) = app_url(app) {
        env.insert("APSTR_APP_URL".to_string(), url);
    }
    if let Some(url) = asc_app_url(app) {
        env.insert("APSTR_ASC_APP_URL".to_string(), url);
    }
    if let Some(url) = testflight_url(app) {
        env.insert("APSTR_TESTFLIGHT_URL".to_string(), url);
    }

    env
}

fn insert_build_env(env: &mut BTreeMap<String, String>, app: &App, build: &Build) {
    env.insert("APSTR_BUILD_ID".to_string(), build.asc_id.clone());
    env.insert("APSTR_BUILD_STATUS".to_string(), build.status().to_string());
    if let Some(number) = build.number {
        env.insert("APSTR_BUILD_NUMBER".to_string(), number.to_string());
    }
    if let Some(url) = asc_build_url(app, build) {
        env.insert("APSTR_ASC_BUILD_URL".to_string(), url);
    }
}

fn insert_workflow_env(env: &mut BTreeMap<String, String>, workflow: &Workflow) {
    env.insert("APSTR_WORKFLOW_ID".to_string(), workflow.asc_id.clone());
    env.insert(
        "APSTR_WORKFLOW_NAME".to_string(),
        workflow.display_name().to_string(),
    );
}

fn insert_testflight_env(env: &mut BTreeMap<String, String>, build: &TestFlightBuild) {
    if let Some(version) = &build.version {
        env.insert("APSTR_TESTFLIGHT_VERSION".to_string(), version.clone());
    }
    env.insert(
        "APSTR_TESTFLIGHT_EXPIRATION_STATUS".to_string(),
        build.expiration_status(),
    );
    if let Some(expiration_date) = build.expiration_date {
        env.insert(
            "APSTR_TESTFLIGHT_EXPIRATION_DATE".to_string(),
            expiration_date.utc_date(),
        );
    }
}

fn spawn_hook(app: &App, env: BTreeMap<String, String>) {
    let Some(script) = app
        .hook_script
        .as_deref()
        .map(str::trim)
        .filter(|script| !script.is_empty())
    else {
        return;
    };

    let script = script.to_string();
    let event = env.get("APSTR_EVENT").cloned().unwrap_or_default();
    let event_label = env.get("APSTR_EVENT_LABEL").cloned().unwrap_or_default();
    let app_id = app.id;
    let hook_run_id = create_hook_run(app, &script, &event, &event_label);

    tokio::spawn(async move {
        let result = run_hook(&script, &event, app_id, env).await;
        if let Some(hook_run_id) = hook_run_id {
            update_hook_run(hook_run_id, result);
        }
    });
}

fn create_hook_run(app: &App, script: &str, event: &str, event_label: &str) -> Option<u64> {
    match HookRun::builder()
        .app(app.clone())
        .event(event)
        .event_label(event_label)
        .command(script)
        .started_at(Timestamp::now())
        .timed_out(false)
        .create()
    {
        Ok(run) => Some(run.id),
        Err(error) => {
            tracing::warn!(app_id = app.id, event, %error, "failed to create hook run");
            None
        }
    }
}

async fn run_hook(
    script: &str,
    event: &str,
    app_id: u64,
    env: BTreeMap<String, String>,
) -> HookResult {
    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg(script)
        .envs(env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    match time::timeout(HOOK_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => {
            if output.status.success() {
                tracing::debug!(app_id, event, "hook script completed");
            } else {
                tracing::warn!(
                    app_id,
                    event,
                    status = %output.status,
                    stdout = %String::from_utf8_lossy(&output.stdout),
                    stderr = %String::from_utf8_lossy(&output.stderr),
                    "hook script failed"
                );
            }

            HookResult {
                exit_code: output.status.code().map(i64::from),
                stdout: Some(captured_output(&output.stdout)),
                stderr: Some(captured_output(&output.stderr)),
                error: if output.status.code().is_none() {
                    Some(output.status.to_string())
                } else {
                    None
                },
                timed_out: false,
            }
        }
        Ok(Err(error)) => {
            tracing::warn!(app_id, event, %error, "failed to run hook script");
            HookResult {
                exit_code: None,
                stdout: None,
                stderr: None,
                error: Some(error.to_string()),
                timed_out: false,
            }
        }
        Err(_) => {
            tracing::warn!(app_id, event, timeout = ?HOOK_TIMEOUT, "hook script timed out");
            HookResult {
                exit_code: None,
                stdout: None,
                stderr: None,
                error: Some(format!(
                    "timed out after {} seconds",
                    HOOK_TIMEOUT.as_secs()
                )),
                timed_out: true,
            }
        }
    }
}

fn update_hook_run(id: u64, result: HookResult) {
    let update = || -> anyhow::Result<()> {
        let mut run = HookRun::find(id)?;
        run.finished_at = Some(Timestamp::now());
        run.exit_code = result.exit_code;
        run.timed_out = result.timed_out;
        run.stdout = result.stdout;
        run.stderr = result.stderr;
        run.error = result.error;
        run.save()?;
        Ok(())
    };

    if let Err(error) = update() {
        tracing::warn!(hook_run_id = id, ?error, "failed to update hook run");
    }
}

struct HookResult {
    exit_code: Option<i64>,
    stdout: Option<String>,
    stderr: Option<String>,
    error: Option<String>,
    timed_out: bool,
}

fn captured_output(bytes: &[u8]) -> String {
    let truncated = bytes.len() > MAX_CAPTURED_OUTPUT_BYTES;
    let bytes = if truncated {
        &bytes[..MAX_CAPTURED_OUTPUT_BYTES]
    } else {
        bytes
    };
    let mut output = String::from_utf8_lossy(bytes).into_owned();
    if truncated {
        output.push_str("\n[truncated]");
    }
    output
}

fn app_url(app: &App) -> Option<String> {
    std::env::var("APSTR_BASE_URL")
        .ok()
        .map(|base_url| format!("{}/apps/{}", base_url.trim_end_matches('/'), app.id))
}

fn asc_app_url(app: &App) -> Option<String> {
    asc_team_id().map(|team_id| {
        format!(
            "https://appstoreconnect.apple.com/teams/{team_id}/apps/{}",
            app.asc_id
        )
    })
}

fn asc_build_url(app: &App, build: &Build) -> Option<String> {
    asc_team_id().map(|team_id| {
        format!(
            "https://appstoreconnect.apple.com/teams/{team_id}/apps/{}/ci/builds/{}",
            app.asc_id, build.asc_id
        )
    })
}

fn testflight_url(app: &App) -> Option<String> {
    asc_team_id().map(|team_id| {
        format!(
            "https://appstoreconnect.apple.com/teams/{team_id}/apps/{}/testflight",
            app.asc_id
        )
    })
}

fn asc_team_id() -> Option<String> {
    std::env::var("ASC_ISSUER_ID").ok()
}
