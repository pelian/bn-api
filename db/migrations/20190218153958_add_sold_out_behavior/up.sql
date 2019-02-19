ALTER TABLE ticket_types
    ADD sold_out_behavior TEXT NOT NULL DEFAULT ('ShowSoldOut');
