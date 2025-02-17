use diesel::prelude::*;
use crate::schema::users;
use serde::{Serialize, Deserialize};

#[derive(Queryable, Serialize, Deserialize, Debug)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub hashed_password: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub hashed_password: &'a str,
}
