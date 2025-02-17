use diesel::prelude::*;
use diesel::insert_into;

use crate::schema::users;
use crate::models::user::{NewUser, User};

pub fn create_user(conn: &mut SqliteConnection, new_user: NewUser) -> QueryResult<User> {
    insert_into(users::table)
        .values(&new_user)
        .execute(conn)?;
    users::table.order(users::id.desc()).first(conn)
}

pub fn get_user_by_id(conn: &mut SqliteConnection, user_id: i32) -> QueryResult<User> {
    use crate::schema::users::dsl::*;
    users.filter(id.eq(user_id)).first(conn)
}
