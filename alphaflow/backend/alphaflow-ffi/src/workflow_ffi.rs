use libc::c_char;
use crate::ffi_util::{to_c_string, from_c_str};
use data_service::{
    establish_connection_pool,
    db::workflow_ops,
    models::workflow::NewWorkflow,
};
use serde_json;

#[no_mangle]
pub extern "C" fn create_workflow_ffi(
    wf_id: *const c_char,
    name: *const c_char,
    active: bool,
    nodes_json: *const c_char,
    connections_json: *const c_char,
) -> *mut c_char {
    let wf_id_str = from_c_str(wf_id);
    let name_str = from_c_str(name);
    let nodes_str = from_c_str(nodes_json);
    let conns_str = from_c_str(connections_json);

    let pool = establish_connection_pool("sqlite://alpha.db");
    let mut conn = pool.get().unwrap();

    let new_wf = NewWorkflow {
        id: if wf_id_str.is_empty() { None } else { Some(&wf_id_str) },
        name: &name_str,
        active,
        nodes: &nodes_str,
        connections: &conns_str,
    
        settings: None,
    
        // 新增以下3个字段即可
        meta: None,
        owner_id: None,
        static_data: None,
    };

    match workflow_ops::create_workflow(&mut conn, &new_wf) {
        Ok(wf) => {
            let out_json = serde_json::to_string(&wf).unwrap();
            to_c_string(out_json)
        }
        Err(e) => {
            let err = format!(r#"{{"error":"{:?}"}}"#, e);
            to_c_string(err)
        }
    }
}

#[no_mangle]
pub extern "C" fn get_workflow_by_id_ffi(wf_id: *const c_char) -> *mut c_char {
    let wf_id_str = from_c_str(wf_id);

    let pool = establish_connection_pool("sqlite://alpha.db");
    let mut conn = pool.get().unwrap();

    match workflow_ops::get_workflow_by_id(&mut conn, &wf_id_str) {
        Ok(wf) => {
            let out_json = serde_json::to_string(&wf).unwrap();
            to_c_string(out_json)
        },
        Err(e) => {
            let err = format!(r#"{{"error":"{:?}"}}"#, e);
            to_c_string(err)
        }
    }
}