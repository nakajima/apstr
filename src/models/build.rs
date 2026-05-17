use std::str::FromStr;

use seekwel::{BelongsTo, SqlField, model};
use serde::Deserialize;

use crate::models::app::App;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(jiff::Timestamp);

impl Timestamp {
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }

    pub fn is_past(self) -> bool {
        self.0 <= jiff::Timestamp::now()
    }

    pub fn is_within_days(self, days: i64) -> bool {
        self.0.duration_since(jiff::Timestamp::now()).as_secs() <= days * 86_400
    }

    pub fn is_within_last_hours(self, hours: i64) -> bool {
        jiff::Timestamp::now().duration_since(self.0).as_secs() < hours * 3_600
    }

    pub fn days_until_floor(self) -> i64 {
        let seconds = self.0.duration_since(jiff::Timestamp::now()).as_secs();
        seconds.max(0) / 86_400
    }

    pub fn utc_date(self) -> String {
        jiff::tz::TimeZone::UTC
            .to_datetime(self.0)
            .date()
            .to_string()
    }
}

impl Into<i64> for Timestamp {
    fn into(self) -> i64 {
        self.0.as_microsecond()
    }
}

impl FromStr for Timestamp {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl SqlField for Timestamp {
    const SQL_TYPE: &'static str = "text";

    fn to_sql_value(&self) -> rusqlite::types::Value {
        self.0.to_string().into()
    }

    fn from_sql_row(row: &rusqlite::Row, index: usize) -> rusqlite::Result<Self> {
        Ok(Self(
            row.get::<usize, String>(index)?
                .as_str()
                .parse()
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
        ))
    }
}

#[model]
pub struct Build {
    pub id: u64,
    pub app: BelongsTo<App>,
    pub asc_id: String,
    pub number: Option<u64>,
    pub created_date: Option<Timestamp>,
    pub started_date: Option<Timestamp>,
    pub finished_date: Option<Timestamp>,
    pub execution_progress: Option<String>,
    pub completion_status: Option<String>,
    pub start_reason: Option<String>,
    pub cancel_reason: Option<String>,
}
