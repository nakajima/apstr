use std::str::FromStr;

use seekwel::{BelongsTo, SqlField, model};
use serde::Deserialize;

use crate::models::app::App;

#[derive(Clone, Copy, Deserialize)]
pub struct Timestamp(jiff::Timestamp);

impl Timestamp {
    pub fn now() -> Timestamp {
        Self(jiff::Timestamp::now())
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
