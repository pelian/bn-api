SELECT * FROM event_fees_per_event($1, $2, $3) AS r
WHERE ($4 IS NULL OR r.event_id = $4)
  AND ($5 IS NULL OR r.organization_id = $5);