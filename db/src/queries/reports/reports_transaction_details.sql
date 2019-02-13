SELECT e.name                                                                                                           AS event_name,
       tt.name                                                                                                          AS ticket_name,
       CAST(oi.quantity AS BIGINT)                                                                                      AS quantity,
       CAST(COALESCE(oi.refunded_quantity, 0) AS BIGINT)                                                                AS refunded_quantity,
       CAST(oi.quantity - COALESCE(oi.refunded_quantity, 0) AS BIGINT)                                                  AS actual_quantity,
       CAST(oi.unit_price_in_cents AS BIGINT)                                                                           AS unit_price_in_cents,
       CAST(COALESCE(oi_fees.company_fee_in_cents, 0) AS BIGINT)                                                        AS company_fee_in_cents,
       CAST(COALESCE(oi_fees.client_fee_in_cents, 0) AS BIGINT)                                                         AS client_fee_in_cents,
       CAST(COALESCE(oi_fees.client_fee_in_cents, 0) +
            COALESCE(oi_fees.company_fee_in_cents, 0) AS BIGINT)                                                        AS gross_fee_in_cents,
       CAST(
             (COALESCE(oi_fees.client_fee_in_cents, 0) + COALESCE(oi_fees.company_fee_in_cents, 0)) *
             (COALESCE(oi_fees.quantity, 0) - COALESCE(oi_fees.refunded_quantity, 0)) AS BIGINT)                        AS gross_fee_in_cents_total,

       CAST(COALESCE(oi_event_fees.company_fee_in_cents, 0) AS BIGINT)                                                  AS event_fee_company_in_cents,
       CAST(COALESCE(oi_event_fees.client_fee_in_cents, 0) AS BIGINT)                                                   AS event_fee_client_in_cents,
       CAST(COALESCE(oi_event_fees.client_fee_in_cents, 0) + COALESCE(oi_event_fees.company_fee_in_cents, 0) AS BIGINT) AS event_fee_gross_in_cents,
       CAST(
             (COALESCE(oi_event_fees.client_fee_in_cents, 0) + COALESCE(oi_event_fees.company_fee_in_cents, 0)) *
             (COALESCE(oi_event_fees.quantity, 0) - COALESCE(oi_event_fees.refunded_quantity, 0)) AS BIGINT)            AS event_fee_gross_in_cents_total,
       oi_fees.fee_schedule_range_id                                                                                    AS fee_range_id,
       orders.paid_at                                                                                                   AS transaction_date,
       orders.order_type,
       p.payment_method,
       p.payment_provider,
       h.redemption_code,
       orders.id                                                                                                        AS order_id,
       oi.event_id,
       orders.user_id,
       CAST(
             (oi.quantity - oi.refunded_quantity) * oi.unit_price_in_cents +
             (COALESCE(oi_fees.quantity, 0) - COALESCE(oi_fees.refunded_quantity, 0)) *
             COALESCE(oi_fees.unit_price_in_cents, 0) +
             (COALESCE(oi_event_fees.quantity, 0) - COALESCE(oi_event_fees.refunded_quantity, 0)) *
             COALESCE(oi_event_fees.unit_price_in_cents, 0)
         AS BIGINT)                                                                                                     AS gross,
       COALESCE(u.first_name, '')                                                                                       AS first_name,
       COALESCE(u.last_name, '')                                                                                        AS last_name,
       COALESCE(u.phone, '')                                                                                            AS phone,
       COALESCE(u.email, '')                                                                                            AS email,
       e.event_start                                                                                                    AS event_start

FROM orders
       LEFT JOIN order_items oi on (orders.id = oi.order_id AND oi.item_type = 'Tickets')
       LEFT JOIN order_items oi_fees on oi.id = oi_fees.parent_id
       LEFT JOIN order_items oi_event_fees ON (oi_event_fees.item_type = 'EventFees' AND orders.id = oi_event_fees.order_id)
       LEFT JOIN ticket_types tt ON (oi.ticket_type_id = tt.id)
       LEFT JOIN (SELECT order_id, ARRAY_TO_STRING(ARRAY_AGG(DISTINCT p.payment_method), ', ') AS payment_method, ARRAY_TO_STRING(ARRAY_AGG(DISTINCT p.provider), ', ') AS payment_provider FROM payments p GROUP BY p.payment_method, p.order_id) AS p on orders.id = p.order_id
       LEFT JOIN holds h on oi.hold_id = h.id
       LEFT JOIN events e on oi.event_id = e.id
       LEFT JOIN users u on orders.user_id = u.id
WHERE orders.status = 'Paid'
  AND ($1 IS NULL OR oi.event_id = $1)
  AND ($2 IS NULL OR e.organization_id = $2)
  AND ($3 IS NULL OR orders.paid_at >= $3)
  AND ($4 IS NULL OR orders.paid_at <= $4)
  AND (oi.item_type = 'Tickets');