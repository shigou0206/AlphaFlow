// src/db/shared_workflow_ops.rs

use diesel::prelude::*;
use diesel::result::QueryResult;
use crate::models::shared_workflow::{SharedWorkflow, NewSharedWorkflow};
use crate::schema::shared_workflows;

pub fn create_shared_workflow(conn: &mut SqliteConnection, new_sw: &NewSharedWorkflow) -> QueryResult<SharedWorkflow> {
    diesel::insert_into(shared_workflows::table)
        .values(new_sw)
        .execute(conn)?;

    shared_workflows::table
        .order(shared_workflows::id.desc())
        .first(conn)
}

pub fn get_shared_by_id(conn: &mut SqliteConnection, shared_id: &str) -> QueryResult<SharedWorkflow> {
    shared_workflows::table
        .filter(shared_workflows::id.eq(Some(shared_id.to_string())))
        .first(conn)
}

pub fn list_shared_for_user(conn: &mut SqliteConnection, user_id: &str) -> QueryResult<Vec<SharedWorkflow>> {
    shared_workflows::table
        .filter(shared_workflows::user_id.eq(user_id.to_string()))
        .order(shared_workflows::created_at.desc())
        .load(conn)
}

pub fn update_permission(conn: &mut SqliteConnection, shared_id: &str, new_permission: &str) -> QueryResult<usize> {
    diesel::update(shared_workflows::table.filter(shared_workflows::id.eq(Some(shared_id.to_string()))))
        .set(shared_workflows::permission.eq(new_permission))
        .execute(conn)
}

pub fn delete_shared_workflow(conn: &mut SqliteConnection, shared_id: &str) -> QueryResult<usize> {
    diesel::delete(shared_workflows::table.filter(shared_workflows::id.eq(Some(shared_id.to_string()))))
        .execute(conn)
}