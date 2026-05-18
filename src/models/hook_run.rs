use seekwel::{BelongsTo, model};

use crate::models::{app::App, build::Timestamp};

#[model]
pub struct HookRun {
    pub id: u64,
    pub app: BelongsTo<App>,
    pub event: String,
    pub event_label: String,
    pub command: String,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub exit_code: Option<i64>,
    pub timed_out: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error: Option<String>,
}

impl HookRun {
    pub fn status(&self) -> String {
        if self.timed_out {
            return "timed out".to_string();
        }

        if self.error.is_some() {
            return "error".to_string();
        }

        match self.exit_code {
            Some(0) => "succeeded".to_string(),
            Some(code) => format!("exit {code}"),
            None => "running".to_string(),
        }
    }
}
