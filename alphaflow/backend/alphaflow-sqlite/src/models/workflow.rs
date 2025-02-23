use diesel::prelude::*;
use crate::schema::workflows;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

use crate::naive_dt_seconds::naive as dt_seconds;

#[derive(Queryable, Debug, Serialize, Deserialize)]
#[diesel(table_name = workflows)]
pub struct Workflow {
    pub id: Option<String>,
    pub name: String,
    pub active: bool,
    pub nodes: String,
    pub connections: String,
    pub settings: Option<String>,
    pub static_data: Option<String>,
    pub meta: Option<String>,
    pub owner_id: Option<String>,

    #[serde(with = "dt_seconds")]
    pub created_at: NaiveDateTime,

    #[serde(with = "dt_seconds")]
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = workflows)]
pub struct NewWorkflow<'a> {
    pub id: Option<&'a str>,
    pub name: &'a str,
    pub active: bool,
    pub nodes: &'a str,
    pub connections: &'a str,
    pub settings: Option<&'a str>,
    pub static_data: Option<&'a str>,
    pub meta: Option<&'a str>,
    pub owner_id: Option<&'a str>,
}