SELECT * FROM ticket_sales_per_ticket_pricing($1, $2, $3 ,$4, $5) AS r
WHERE ($6 IS NULL OR r.event_id = $6)
  AND ($7 IS NULL OR r.organization_id = $7);