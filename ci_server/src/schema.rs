//! Diesel table definitions for generic CI platform.
//!
//! Tables: ci_projects, ci_triggers, ci_builds, ci_build_steps,
//! ci_environments, ci_errors, ci_error_occurrences, ci_artifacts.
//! All tables include tenant_id for multi-tenancy via RLS.

diesel::table! {
    ci_projects (id) {
        id -> Int8,
        tenant_id -> Uuid,
        name -> Varchar,
        github_repo -> Varchar,
        default_branch -> Varchar,
        pipeline_config -> Nullable<Jsonb>,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ci_triggers (id) {
        id -> Int8,
        tenant_id -> Uuid,
        project_id -> Int8,
        event_type -> Varchar,
        branch_pattern -> Nullable<Varchar>,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ci_builds (id) {
        id -> Int8,
        tenant_id -> Uuid,
        project_id -> Int8,
        commit_sha -> Varchar,
        branch -> Varchar,
        pr_number -> Nullable<Int4>,
        author -> Nullable<Varchar>,
        message -> Nullable<Text>,
        fingerprint -> Varchar,
        trigger_event -> Varchar,
        status -> Varchar,
        started_at -> Nullable<Timestamptz>,
        finished_at -> Nullable<Timestamptz>,
        duration_ms -> Nullable<Int4>,
        summary -> Nullable<Jsonb>,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ci_build_steps (id) {
        id -> Int8,
        tenant_id -> Uuid,
        build_id -> Int8,
        name -> Varchar,
        sequence -> Int4,
        status -> Varchar,
        started_at -> Nullable<Timestamptz>,
        finished_at -> Nullable<Timestamptz>,
        duration_ms -> Nullable<Int4>,
        exit_code -> Nullable<Int4>,
        stdout -> Nullable<Text>,
        stderr -> Nullable<Text>,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ci_environments (id) {
        id -> Int8,
        tenant_id -> Uuid,
        project_id -> Int8,
        build_id -> Nullable<Int8>,
        pr_number -> Int4,
        branch -> Varchar,
        commit_sha -> Varchar,
        status -> Varchar,
        url -> Nullable<Varchar>,
        last_activity -> Nullable<Timestamptz>,
        idle_timeout_min -> Int4,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ci_errors (id) {
        id -> Int8,
        tenant_id -> Uuid,
        project_id -> Nullable<Int8>,
        fingerprint -> Varchar,
        category -> Varchar,
        severity -> Varchar,
        title -> Varchar,
        file_path -> Nullable<Varchar>,
        line_number -> Nullable<Int4>,
        first_seen_at -> Timestamptz,
        last_seen_at -> Timestamptz,
        occurrence_count -> Int4,
        status -> Varchar,
        assigned_to -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        raw_text -> Text,
        normalized_text -> Text,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ci_error_occurrences (id) {
        id -> Int8,
        tenant_id -> Uuid,
        error_id -> Int8,
        build_id -> Int8,
        step_name -> Varchar,
        raw_output -> Nullable<Text>,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ci_artifacts (id) {
        id -> Int8,
        tenant_id -> Uuid,
        build_id -> Int8,
        name -> Varchar,
        artifact_type -> Varchar,
        content -> Nullable<Text>,
        size_bytes -> Nullable<Int8>,
        active -> Bool,
        create_uid -> Nullable<Int8>,
        create_date -> Nullable<Timestamptz>,
        write_uid -> Nullable<Int8>,
        write_date -> Nullable<Timestamptz>,
    }
}

// Foreign key relationships
diesel::joinable!(ci_triggers -> ci_projects (project_id));
diesel::joinable!(ci_builds -> ci_projects (project_id));
diesel::joinable!(ci_build_steps -> ci_builds (build_id));
diesel::joinable!(ci_environments -> ci_projects (project_id));
diesel::joinable!(ci_environments -> ci_builds (build_id));
diesel::joinable!(ci_errors -> ci_projects (project_id));
diesel::joinable!(ci_error_occurrences -> ci_errors (error_id));
diesel::joinable!(ci_error_occurrences -> ci_builds (build_id));
diesel::joinable!(ci_artifacts -> ci_builds (build_id));

diesel::allow_tables_to_appear_in_same_query!(
    ci_projects,
    ci_triggers,
    ci_builds,
    ci_build_steps,
    ci_environments,
    ci_errors,
    ci_error_occurrences,
    ci_artifacts,
);
