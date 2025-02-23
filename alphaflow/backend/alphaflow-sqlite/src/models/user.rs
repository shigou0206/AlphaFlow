use diesel::prelude::*;
use crate::schema::users;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

use crate::naive_dt_seconds::{naive as dt_seconds};

#[derive(Queryable, Debug, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct User {
    // id -> Nullable<Text>
    pub id: Option<String>,
    pub email: String,
    pub password_hash: String,
    pub role: String,

    #[serde(with = "dt_seconds")]
    pub created_at: NaiveDateTime,

    #[serde(with = "dt_seconds")]
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub id: Option<&'a str>,
    pub email: &'a str,
    pub password_hash: &'a str,
    pub role: &'a str,
}