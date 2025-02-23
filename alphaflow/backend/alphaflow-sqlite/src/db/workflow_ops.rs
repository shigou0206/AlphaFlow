// src/db/workflow_ops.rs

use diesel::prelude::*;
use diesel::result::QueryResult;
use crate::models::workflow::{Workflow, NewWorkflow};
use crate::schema::workflows;

pub fn create_workflow(conn: &mut SqliteConnection, new_wf: &NewWorkflow) -> QueryResult<Workflow> {
    // INSERT INTO workflows
    diesel::insert_into(workflows::table)
        .values(new_wf)
        .execute(conn)?;

    // fetch newly inserted
    workflows::table
        .order(workflows::id.desc())
        .first(conn)
}

pub fn get_workflow_by_id(conn: &mut SqliteConnection, wf_id: &str) -> QueryResult<Workflow> {
    // SELECT * FROM workflows WHERE id == Some(wf_id)
    workflows::table
        .filter(workflows::id.eq(Some(wf_id.to_string())))
        .first(conn)
}

pub fn list_workflows(conn: &mut SqliteConnection) -> QueryResult<Vec<Workflow>> {
    // SELECT * FROM workflows ORDER BY created_at desc
    workflows::table
        .order(workflows::created_at.desc())
        .load(conn)
}

pub fn update_workflow_name(conn: &mut SqliteConnection, wf_id: &str, new_name: &str) -> QueryResult<usize> {
    // UPDATE workflows SET name = new_name WHERE id == Some(wf_id)
    diesel::update(workflows::table.filter(workflows::id.eq(Some(wf_id.to_string()))))
        .set(workflows::name.eq(new_name))
        .execute(conn)
}

pub fn delete_workflow(conn: &mut SqliteConnection, wf_id: &str) -> QueryResult<usize> {
    // DELETE FROM workflows WHERE id == Some(wf_id)
    diesel::delete(workflows::table.filter(workflows::id.eq(Some(wf_id.to_string()))))
        .execute(conn)
}