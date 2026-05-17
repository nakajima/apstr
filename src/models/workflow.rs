use seekwel::{BelongsTo, model};

use crate::models::app::App;

#[model]
pub struct Workflow {
    pub id: u64,
    pub app: BelongsTo<App>,
    pub asc_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_enabled: Option<bool>,
    pub is_locked_for_editing: Option<bool>,
}

impl Workflow {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.asc_id)
    }

    pub fn can_start(&self) -> bool {
        self.is_enabled.unwrap_or(true) && !self.is_locked_for_editing.unwrap_or(false)
    }
}
