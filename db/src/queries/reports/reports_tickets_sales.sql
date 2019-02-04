SELECT * FROM ticket_sales_per_ticket_pricing($1, $2, $3 ,$4) AS r
WHERE ($5 IS NULL OR r.event_id = $5)
  AND ($6 IS NULL OR r.organization_id = $6);