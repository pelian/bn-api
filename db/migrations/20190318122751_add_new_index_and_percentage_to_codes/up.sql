ALTER TABLE codes
  DROP CONSTRAINT codes_max_uses_check,
  ADD COLUMN discount_as_percentage BIGINT NULL,
  ADD CONSTRAINT discount_absolute_xor_percentage CHECK(code_type <> 'Discount' OR ((discount_as_percentage IS NOT NULL AND discount_in_cents IS NULL) OR (discount_as_percentage IS NULL AND discount_in_cents IS NOT NULL)));

CREATE INDEX index_redemption_code_event_id ON codes (redemption_code, event_id);

ALTER TABLE order_items DROP CONSTRAINT order_items_code_id_max_uses_valid;
