// src/db/user_ops.rs

use diesel::prelude::*;
use diesel::result::QueryResult;
use crate::models::user::{User, NewUser};
use crate::schema::users;

pub fn create_user(conn: &mut SqliteConnection, new_u: &NewUser) -> QueryResult<User> {
    // INSERT INTO users (...)
    diesel::insert_into(users::table)
        .values(new_u)
        .execute(conn)?;

    // ORDER BY id desc, fetch newly created
    users::table
        .order(users::id.desc())
        .first(conn)
}

pub fn get_user_by_id(conn: &mut SqliteConnection, user_id: &str) -> QueryResult<User> {
    // SELECT * FROM users WHERE id == Some(user_id)
    users::table
        .filter(users::id.eq(Some(user_id.to_string())))
        .first(conn)
}

pub fn get_user_by_email(conn: &mut SqliteConnection, email: &str) -> QueryResult<User> {
    // SELECT * FROM users WHERE email == email
    users::table
        .filter(users::email.eq(email))
        .first(conn)
}

pub fn list_users(conn: &mut SqliteConnection) -> QueryResult<Vec<User>> {
    // SELECT * FROM users ORDER BY created_at desc
    users::table
        .order(users::created_at.desc())
        .load(conn)
}

pub fn update_user_role(conn: &mut SqliteConnection, user_id: &str, new_role: &str) -> QueryResult<usize> {
    // UPDATE users SET role = new_role WHERE id == Some(user_id)
    diesel::update(users::table.filter(users::id.eq(Some(user_id.to_string()))))
        .set(users::role.eq(new_role))
        .execute(conn)
}

pub fn delete_user(conn: &mut SqliteConnection, user_id: &str) -> QueryResult<usize> {
    // DELETE FROM users WHERE id == Some(user_id)
    diesel::delete(users::table.filter(users::id.eq(Some(user_id.to_string()))))
        .execute(conn)
}

pub fn login_user(conn: &mut SqliteConnection, user_email: &str, password: &str) -> QueryResult<User> {
    use crate::schema::users::dsl::*;
    // SELECT * FROM users WHERE email=? and password_hash=?
    // limit 1
    users
        .filter(email.eq(user_email))
        .filter(password_hash.eq(password))
        .first::<User>(conn)
}