// src/db/execution_ops.rs

use diesel::prelude::*;
use diesel::result::QueryResult;
use crate::models::execution::{Execution, NewExecution};
use crate::schema::executions;

pub fn create_execution(conn: &mut SqliteConnection, new_exec: &NewExecution) -> QueryResult<Execution> {
    diesel::insert_into(executions::table)
        .values(new_exec)
        .execute(conn)?;

    executions::table
        .order(executions::id.desc())
        .first(conn)
}

pub fn get_execution_by_id(conn: &mut SqliteConnection, exec_id: &str) -> QueryResult<Execution> {
    executions::table
        .filter(executions::id.eq(Some(exec_id.to_string())))
        .first(conn)
}

pub fn list_executions_by_workflow(conn: &mut SqliteConnection, wf_id: &str) -> QueryResult<Vec<Execution>> {
    executions::table
        .filter(executions::workflow_id.eq(wf_id.to_string()))
        .order(executions::created_at.desc())
        .load(conn)
}

pub fn update_execution_mode(conn: &mut SqliteConnection, exec_id: &str, new_mode: &str) -> QueryResult<usize> {
    // e.g. update "manual" -> "trigger"
    diesel::update(executions::table.filter(executions::id.eq(Some(exec_id.to_string()))))
        .set(executions::mode.eq(new_mode))
        .execute(conn)
}

pub fn delete_execution(conn: &mut SqliteConnection, exec_id: &str) -> QueryResult<usize> {
    diesel::delete(executions::table.filter(executions::id.eq(Some(exec_id.to_string()))))
        .execute(conn)
}