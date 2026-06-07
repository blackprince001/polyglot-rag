-- Enable uuid extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Tenants: every piece of data belongs to exactly one tenant.
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- API keys: a service authenticates with a key that resolves to a tenant.
-- Only the SHA-256 hash of the raw key is stored; the raw key is shown once.
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL UNIQUE,
    key_prefix TEXT NOT NULL,
    name TEXT,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    last_used_at TIMESTAMP WITH TIME ZONE,
    revoked_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_tenant_id ON api_keys(tenant_id);

-- Files table
CREATE TABLE files (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_size BIGINT,
    file_type TEXT,
    file_hash TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_files_tenant_id ON files(tenant_id);

-- Content chunks table (tenant_id denormalized from the owning file)
CREATE TABLE content_chunks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    file_id UUID REFERENCES files(id) ON DELETE CASCADE,
    chunk_text TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    token_count INTEGER,
    page_number INTEGER,
    section_path TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_content_chunks_tenant_id ON content_chunks(tenant_id);
CREATE INDEX idx_content_chunks_file_id ON content_chunks(file_id);

-- Embeddings table (tenant_id denormalized for fast tenant-filtered vector search)
CREATE TABLE embeddings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    content_chunk_id UUID REFERENCES content_chunks(id) ON DELETE CASCADE,
    embedding VECTOR(1024),
    model_name TEXT NOT NULL,
    model_version TEXT,
    generated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    generation_parameters JSONB
);

-- HNSW index using cosine distance. Embeddings are normalized by the embedder,
-- so cosine and L2 rank identically; cosine keeps scores in a clean [0, 1] range.
CREATE INDEX idx_embeddings_hnsw ON embeddings USING hnsw (embedding vector_cosine_ops);
CREATE INDEX idx_embeddings_tenant_id ON embeddings(tenant_id);
CREATE INDEX idx_embeddings_content_chunk_id ON embeddings(content_chunk_id);
