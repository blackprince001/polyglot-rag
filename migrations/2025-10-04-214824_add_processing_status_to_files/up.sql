-- Add processing_status column to files table
ALTER TABLE files ADD COLUMN processing_status VARCHAR(20) DEFAULT 'pending';

-- Update existing records to have 'pending' status
UPDATE files SET processing_status = 'pending' WHERE processing_status IS NULL;

-- Make the column NOT NULL after setting default values
ALTER TABLE files ALTER COLUMN processing_status SET NOT NULL;

-- Add index for efficient querying by processing status
CREATE INDEX idx_files_processing_status ON files(processing_status);
