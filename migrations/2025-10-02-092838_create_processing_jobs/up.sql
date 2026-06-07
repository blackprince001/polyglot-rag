-- Create processing_jobs table for async job management
CREATE TABLE processing_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    job_type VARCHAR(150) NOT NULL,
    job_data JSONB,
    status VARCHAR(150) NOT NULL DEFAULT 'pending',
    progress REAL NOT NULL DEFAULT 0.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    result_summary JSONB
);

-- Create indexes for better query performance
CREATE INDEX idx_processing_jobs_tenant_id ON processing_jobs(tenant_id);
CREATE INDEX idx_processing_jobs_file_id ON processing_jobs(file_id);
CREATE INDEX idx_processing_jobs_status ON processing_jobs(status);
CREATE INDEX idx_processing_jobs_created_at ON processing_jobs(created_at);
CREATE INDEX idx_processing_jobs_active ON processing_jobs(status) WHERE status IN ('pending', 'processing');

-- Add constraints
ALTER TABLE processing_jobs ADD CONSTRAINT chk_progress_range CHECK (progress >= 0.0 AND progress <= 1.0);
-- ALTER TABLE processing_jobs ADD CONSTRAINT chk_job_type CHECK (job_type IN ('file_processing', 'url_extraction', 'youtube_extraction'));
-- ALTER TABLE processing_jobs ADD CONSTRAINT chk_status CHECK (status IN ('pending', 'processing', 'completed') OR status LIKE 'failed:%');
ALTER TABLE processing_jobs ADD CONSTRAINT chk_timestamps CHECK (
    (started_at IS NULL OR started_at >= created_at) AND
    (completed_at IS NULL OR (started_at IS NOT NULL AND completed_at >= started_at))
);
