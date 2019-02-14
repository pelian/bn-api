CREATE OR REPLACE FUNCTION event_ids_belong_to_organization(UUID, UUID[]) RETURNS BOOLEAN AS $$
BEGIN
    RETURN (
        SELECT NOT EXISTS (
            SELECT *
            FROM organizations o
            LEFT JOIN events e ON e.organization_id = o.id
            WHERE e.id = ANY($2)
            AND o.id <> $1
        )
    );
END $$ LANGUAGE 'plpgsql';

ALTER TABLE organization_users
    ADD event_ids uuid[] NOT NULL DEFAULT '{}';
ALTER TABLE organization_users
    ADD CONSTRAINT event_ids_belong_to_organization_users CHECK(event_ids_belong_to_organization(organization_id, event_ids));
ALTER TABLE organization_invites
    ADD event_ids uuid[] NOT NULL DEFAULT '{}';
ALTER TABLE organization_invites
    ADD CONSTRAINT organization_invites_event_ids_belong_to_organization_invites CHECK(event_ids_belong_to_organization(organization_id, event_ids));
