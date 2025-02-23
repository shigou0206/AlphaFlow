use diesel::prelude::*;
use crate::schema::shared_workflows;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

use crate::naive_dt_seconds::naive as dt_seconds;

#[derive(Queryable, Debug, Serialize, Deserialize)]
#[diesel(table_name = shared_workflows)]
pub struct SharedWorkflow {
    pub id: Option<String>,
    pub workflow_id: String,
    pub user_id: String,
    pub permission: String,

    #[serde(with = "dt_seconds")]
    pub created_at: NaiveDateTime,

    #[serde(with = "dt_seconds")]
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = shared_workflows)]
pub struct NewSharedWorkflow<'a> {
    pub id: Option<&'a str>,
    pub workflow_id: &'a str,
    pub user_id: &'a str,
    pub permission: &'a str,
}