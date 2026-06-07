// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    tenants (id) {
        id -> Uuid,
        name -> Text,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    api_keys (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        key_hash -> Text,
        key_prefix -> Text,
        name -> Nullable<Text>,
        scopes -> Array<Text>,
        last_used_at -> Nullable<Timestamptz>,
        revoked_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    content_chunks (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        file_id -> Uuid,
        chunk_text -> Text,
        chunk_index -> Int4,
        token_count -> Nullable<Int4>,
        page_number -> Nullable<Int4>,
        section_path -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    embeddings (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        content_chunk_id -> Nullable<Uuid>,
        embedding -> Nullable<Vector>,
        model_name -> Text,
        model_version -> Nullable<Text>,
        generated_at -> Nullable<Timestamptz>,
        generation_parameters -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    file_assets (id) {
        id -> Uuid,
        file_id -> Uuid,
        tenant_id -> Uuid,
        #[max_length = 50]
        asset_type -> Varchar,
        #[max_length = 512]
        storage_key -> Varchar,
        #[max_length = 255]
        content_type -> Varchar,
        page_number -> Nullable<Int4>,
        label -> Nullable<Text>,
        byte_size -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    files (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        file_path -> Text,
        file_name -> Text,
        file_size -> Nullable<Int8>,
        file_type -> Nullable<Text>,
        file_hash -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        metadata -> Nullable<Jsonb>,
        processing_status -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    processing_jobs (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        file_id -> Uuid,
        job_type -> Varchar,
        job_data -> Nullable<Jsonb>,
        status -> Varchar,
        progress -> Float4,
        created_at -> Timestamptz,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        error_message -> Nullable<Text>,
        result_summary -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    search_queries (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        query_text -> Text,
        results_count -> Int4,
        created_at -> Timestamptz,
        user_id -> Nullable<Text>,
        search_parameters -> Nullable<Jsonb>,
    }
}

diesel::joinable!(api_keys -> tenants (tenant_id));
diesel::joinable!(content_chunks -> files (file_id));
diesel::joinable!(file_assets -> files (file_id));
diesel::joinable!(file_assets -> tenants (tenant_id));
diesel::joinable!(content_chunks -> tenants (tenant_id));
diesel::joinable!(embeddings -> content_chunks (content_chunk_id));
diesel::joinable!(embeddings -> tenants (tenant_id));
diesel::joinable!(files -> tenants (tenant_id));
diesel::joinable!(processing_jobs -> tenants (tenant_id));
diesel::joinable!(search_queries -> tenants (tenant_id));

diesel::allow_tables_to_appear_in_same_query!(
    api_keys,
    content_chunks,
    embeddings,
    file_assets,
    files,
    processing_jobs,
    search_queries,
    tenants,
);
