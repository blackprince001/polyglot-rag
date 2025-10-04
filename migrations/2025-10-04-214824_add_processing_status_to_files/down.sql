-- Remove the processing_status column and its index
DROP INDEX IF EXISTS idx_files_processing_status;
ALTER TABLE files DROP COLUMN IF EXISTS processing_status;
