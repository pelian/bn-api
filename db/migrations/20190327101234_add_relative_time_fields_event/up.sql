ALTER TABLE ticket_types
    ADD parent_id UUID NULL REFERENCES ticket_types (id);

ALTER TABLE ticket_types
    ALTER COLUMN start_date DROP NOT NULL;

ALTER TABLE ticket_types
    ADD CONSTRAINT check_ticket_types_start_date_parent_id CHECK (start_date IS NOT NULL OR parent_id IS NOT NULL);


ALTER TABLE ticket_pricing
    ALTER COLUMN start_date DROP NOT NULL;

