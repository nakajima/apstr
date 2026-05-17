use seekwel::{BelongsTo, model};

use crate::models::{app::App, build::Timestamp};

#[model]
pub struct TestFlightBuild {
    pub id: u64,
    pub app: BelongsTo<App>,
    pub asc_id: String,
    pub version: Option<String>,
    pub uploaded_date: Option<Timestamp>,
    pub expiration_date: Option<Timestamp>,
    pub expired: Option<bool>,
    pub processing_state: Option<String>,
}

impl TestFlightBuild {
    pub fn is_valid(&self) -> bool {
        self.processing_state.as_deref() == Some("VALID")
    }

    pub fn is_expired(&self) -> bool {
        self.expired.unwrap_or(false)
            || self
                .expiration_date
                .is_some_and(|expiration_date| expiration_date.is_past())
    }

    pub fn expires_within_days(&self, days: i64) -> bool {
        self.expiration_date
            .is_some_and(|expiration_date| expiration_date.is_within_days(days))
    }

    pub fn expiration_status(&self) -> String {
        if self.is_expired() {
            return "Expired".to_string();
        }

        let Some(expiration_date) = self.expiration_date else {
            return "Expiration unknown".to_string();
        };

        match expiration_date.days_until_floor() {
            0 => "Expires TODAY".to_string(),
            1 => "Expires TOMORROW".to_string(),
            days => format!("Expires in {days} days"),
        }
    }
}
