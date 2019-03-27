DROP INDEX index_redemption_code_event_id;

UPDATE codes
SET max_uses = 9223372036854775807 where max_uses = 0; -- previously no limit

ALTER TABLE codes
  DROP CONSTRAINT discount_absolute_xor_percentage,
  DROP COLUMN discount_as_percentage,
  ADD CHECK (max_uses > 0);

ALTER TABLE order_items ADD CONSTRAINT order_items_code_id_max_uses_valid CHECK(order_items_code_id_max_uses_valid(order_id, code_id)); -- This constraint considers a single code use to be a row in order_items where it should be multiplied by quantity