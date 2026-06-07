-- Binary assets (e.g. embedded images) extracted from documents. The bytes
-- live in the configured file storage backend under `storage_key`; this table
-- records the metadata and links each asset to its owning file and tenant.
CREATE TABLE file_assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    asset_type VARCHAR(50) NOT NULL,
    storage_key VARCHAR(512) NOT NULL,
    content_type VARCHAR(255) NOT NULL,
    page_number INT,
    label TEXT,
    byte_size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_file_assets_file_tenant ON file_assets(file_id, tenant_id);
