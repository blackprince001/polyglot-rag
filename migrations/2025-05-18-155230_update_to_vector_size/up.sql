-- No-op: the embedding column dimension (1024) and its vector index are now
-- defined directly in the create_items migration. Kept as a registered
-- migration so existing migration history stays consistent.
SELECT 1;
