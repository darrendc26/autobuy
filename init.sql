CREATE TABLE intents (
  id BIGSERIAL PRIMARY KEY,
  trigger_price NUMERIC(18, 8) NOT NULL,
  status TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT now()
);