

ALTER TABLE ticket_pricing
    ALTER COLUMN start_date SET NOT NULL;



ALTER TABLE ticket_types
    DROP CONSTRAINT check_ticket_types_start_date_parent_id;

ALTER TABLE ticket_types
    DROP COLUMN parent_id;

ALTER TABLE ticket_types
    ALTER COLUMN start_date SET NOT NULL;