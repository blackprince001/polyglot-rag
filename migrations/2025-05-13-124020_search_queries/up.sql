-- Your SQL goes here
CREATE TABLE search_queries (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  query_text TEXT, 
  query_embedding VECTOR(1024),
  results_returned INTEGER,
  searched_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
