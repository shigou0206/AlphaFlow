use diesel::prelude::*;
use crate::schema::executions;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

use crate::naive_dt_seconds::{naive as dt_seconds, naive_opt as dt_seconds_opt};

#[derive(Queryable, Debug, Serialize, Deserialize)]
#[diesel(table_name = executions)]
pub struct Execution {
    pub id: Option<String>,
    pub workflow_id: String,
    pub finished: bool,
    pub mode: String,

    #[serde(with = "dt_seconds")]
    pub started_at: NaiveDateTime,

    #[serde(with = "dt_seconds_opt")]
    pub stopped_at: Option<NaiveDateTime>,

    pub data: Option<String>,
    pub started_by_user_id: Option<String>,

    #[serde(with = "dt_seconds")]
    pub created_at: NaiveDateTime,

    #[serde(with = "dt_seconds")]
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = executions)]
pub struct NewExecution<'a> {
    pub id: Option<&'a str>,
    pub workflow_id: &'a str,
    pub finished: bool,
    pub mode: &'a str,

    pub started_at: NaiveDateTime,
    pub stopped_at: Option<NaiveDateTime>,
    pub data: Option<&'a str>,
    pub started_by_user_id: Option<&'a str>,
}