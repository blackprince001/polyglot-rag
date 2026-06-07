-- Search query audit log (tenant-scoped).
CREATE TABLE search_queries (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  query_text TEXT NOT NULL,
  results_count INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  user_id TEXT,
  search_parameters JSONB
);

CREATE INDEX idx_search_queries_tenant_id ON search_queries(tenant_id);
