// @generated automatically by Diesel CLI.

diesel::table! {
    edges (id) {
        id -> Nullable<Integer>,
        workflow_id -> Integer,
        from_node_id -> Integer,
        to_node_id -> Integer,
        condition -> Nullable<Text>,
    }
}

diesel::table! {
    execution_logs (id) {
        id -> Nullable<Integer>,
        workflow_id -> Integer,
        node_id -> Nullable<Integer>,
        status -> Text,
        start_time -> Timestamp,
        end_time -> Nullable<Timestamp>,
        log -> Nullable<Text>,
        retry_count -> Integer,
    }
}

diesel::table! {
    nodes (id) {
        id -> Nullable<Integer>,
        workflow_id -> Integer,
        #[sql_name = "type"]
        type_ -> Text,
        config -> Nullable<Text>,
        position_x -> Nullable<Float>,
        position_y -> Nullable<Float>,
        version -> Integer,
    }
}

diesel::table! {
    users (id) {
        id -> Nullable<Integer>,
        username -> Text,
        email -> Text,
        hashed_password -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    workflows (id) {
        id -> Nullable<Integer>,
        name -> Text,
        description -> Nullable<Text>,
        status -> Text,
        version -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        owner_id -> Nullable<Integer>,
    }
}

diesel::joinable!(edges -> workflows (workflow_id));
diesel::joinable!(execution_logs -> nodes (node_id));
diesel::joinable!(execution_logs -> workflows (workflow_id));
diesel::joinable!(nodes -> workflows (workflow_id));
diesel::joinable!(workflows -> users (owner_id));

diesel::allow_tables_to_appear_in_same_query!(
    edges,
    execution_logs,
    nodes,
    users,
    workflows,
);
