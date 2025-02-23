// @generated automatically by Diesel CLI.

diesel::table! {
    executions (id) {
        id -> Nullable<Text>,
        workflow_id -> Text,
        finished -> Bool,
        mode -> Text,
        started_at -> Timestamp,
        stopped_at -> Nullable<Timestamp>,
        data -> Nullable<Text>,
        started_by_user_id -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    shared_workflows (id) {
        id -> Nullable<Text>,
        workflow_id -> Text,
        user_id -> Text,
        permission -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Nullable<Text>,
        email -> Text,
        password_hash -> Text,
        role -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    workflows (id) {
        id -> Nullable<Text>,
        name -> Text,
        active -> Bool,
        nodes -> Text,
        connections -> Text,
        settings -> Nullable<Text>,
        static_data -> Nullable<Text>,
        meta -> Nullable<Text>,
        owner_id -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(executions -> users (started_by_user_id));
diesel::joinable!(executions -> workflows (workflow_id));
diesel::joinable!(shared_workflows -> users (user_id));
diesel::joinable!(shared_workflows -> workflows (workflow_id));
diesel::joinable!(workflows -> users (owner_id));

diesel::allow_tables_to_appear_in_same_query!(
    executions,
    shared_workflows,
    users,
    workflows,
);
