// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    content_chunks (id) {
        id -> Uuid,
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

    files (id) {
        id -> Uuid,
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
        query_text -> Text,
        results_count -> Int4,
        created_at -> Timestamptz,
        user_id -> Nullable<Text>,
        search_parameters -> Nullable<Jsonb>,
    }
}

diesel::joinable!(content_chunks -> files (file_id));
diesel::joinable!(embeddings -> content_chunks (content_chunk_id));

diesel::allow_tables_to_appear_in_same_query!(
    content_chunks,
    embeddings,
    files,
    processing_jobs,
    search_queries,
);
