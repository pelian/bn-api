ALTER TABLE organization_users
    DROP CONSTRAINT event_ids_belong_to_organization_users;
ALTER TABLE organization_users
    DROP COLUMN event_ids;
ALTER TABLE organization_invites
    DROP CONSTRAINT organization_invites_event_ids_belong_to_organization_invites;
ALTER TABLE organization_invites
    DROP COLUMN event_ids;

DROP FUNCTION event_ids_belong_to_organization;
