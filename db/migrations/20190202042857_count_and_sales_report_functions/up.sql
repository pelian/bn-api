CREATE OR REPLACE FUNCTION ticket_sales_per_ticket_pricing(start TIMESTAMP, "end" TIMESTAMP, group_by_ticket_type BOOLEAN, group_by_event_id BOOLEAN)
  RETURNS TABLE
          (
            organization_id                  UUID,
            event_id                         UUID,
            ticket_type_id                   UUID,
            ticket_pricing_id                UUID,
            ticket_name                      TEXT,
            ticket_status                    TEXT,
            event_name                       TEXT,
            ticket_pricing_name              TEXT,
            box_office_sales_in_cents        BIGINT,
            online_sales_in_cents            BIGINT,
            box_office_sale_count            BIGINT,
            online_sale_count                BIGINT,
            comp_sale_count                  BIGINT,
            total_box_office_fees_in_cents   BIGINT,
            total_online_fees_in_cents       BIGINT,
            company_box_office_fees_in_cents BIGINT,
            client_box_office_fees_in_cents  BIGINT,
            company_online_fees_in_cents     BIGINT,
            client_online_fees_in_cents      BIGINT
          )
AS
$body$
SELECT e.organization_id                                                                                                                                                                                                                                       AS organization_id,
       e.id                                                                                                                                                                                                                                                    AS event_id,
       tt.id                                                                                                                                                                                                                                                   AS ticket_type_id,
       tp.id                                                                                                                                                                                                                                                   AS ticket_pricing_id,
       tt.name                                                                                                                                                                                                                                                 AS ticket_name,
       tt.status                                                                                                                                                                                                                                               AS ticket_status,
       e.name                                                                                                                                                                                                                                                  AS event_name,
       tp.name                                                                                                                                                                                                                                                 AS ticket_pricing_name,
       -- Actual Values
       CAST(COALESCE(SUM(oi.unit_price_in_cents * (oi.quantity - oi.refunded_quantity)) FILTER (WHERE p.is_box_office IS TRUE AND o.status = 'Paid'), 0) AS BIGINT)                                                                                            AS box_office_sales_in_cents,
       CAST(COALESCE(SUM(oi.unit_price_in_cents * (oi.quantity - oi.refunded_quantity)) FILTER (WHERE p.is_box_office IS FALSE AND o.status = 'Paid'), 0) AS BIGINT)                                                                                           AS online_sales_in_cents,


       --Total Sold Count
       CAST(COALESCE(SUM(oi.quantity - oi.refunded_quantity) FILTER (WHERE p.is_box_office IS TRUE AND o.status = 'Paid' AND (h.hold_type IS NULL OR h.hold_type != 'Comp')), 0) AS BIGINT)                                                                                           AS box_office_sale_count,
       CAST(COALESCE(SUM(oi.quantity - oi.refunded_quantity) FILTER (WHERE p.is_box_office IS FALSE AND o.status = 'Paid' AND (h.hold_type IS NULL OR h.hold_type != 'Comp')), 0) AS BIGINT)                                                                                          AS online_sale_count,
       CAST(COALESCE(SUM(oi.quantity - oi.refunded_quantity) FILTER (WHERE h.hold_type = 'Comp' AND o.status = 'Paid'), 0) AS BIGINT)                                                                                                                                                 AS comp_sale_count,


       CAST(COALESCE(SUM((oi_t_fees.unit_price_in_cents * (oi_t_fees.quantity - oi_t_fees.refunded_quantity)) + (oi_e_fees.unit_price_in_cents * (oi_e_fees.quantity - oi_e_fees.refunded_quantity))) FILTER (WHERE p.is_box_office IS TRUE), 0) AS BIGINT)    AS total_box_office_fees_in_cents,
       CAST(COALESCE(SUM((oi_t_fees.unit_price_in_cents * (oi_t_fees.quantity - oi_t_fees.refunded_quantity)) + (oi_e_fees.unit_price_in_cents * (oi_e_fees.quantity - oi_e_fees.refunded_quantity))) FILTER (WHERE p.is_box_office IS FALSE), 0) AS BIGINT)   AS total_online_fees_in_cents,
       CAST(COALESCE(SUM((oi_t_fees.company_fee_in_cents * (oi_t_fees.quantity - oi_t_fees.refunded_quantity)) + (oi_e_fees.company_fee_in_cents * (oi_e_fees.quantity - oi_e_fees.refunded_quantity))) FILTER (WHERE p.is_box_office IS TRUE),
                     0) AS BIGINT)                                                                                                                                                                                                                             AS company_box_office_fees_in_cents,
       CAST(COALESCE(SUM((oi_t_fees.client_fee_in_cents * (oi_t_fees.quantity - oi_t_fees.refunded_quantity)) + (oi_e_fees.client_fee_in_cents * (oi_e_fees.quantity - oi_e_fees.refunded_quantity))) FILTER (WHERE p.is_box_office IS TRUE), 0) AS BIGINT)    AS client_box_office_fees_in_cents,
       CAST(COALESCE(SUM((oi_t_fees.company_fee_in_cents * (oi_t_fees.quantity - oi_t_fees.refunded_quantity)) + (oi_e_fees.company_fee_in_cents * (oi_e_fees.quantity - oi_e_fees.refunded_quantity))) FILTER (WHERE p.is_box_office IS FALSE), 0) AS BIGINT) AS company_online_fees_in_cents,
       CAST(COALESCE(SUM((oi_t_fees.client_fee_in_cents * (oi_t_fees.quantity - oi_t_fees.refunded_quantity)) + (oi_e_fees.client_fee_in_cents * (oi_e_fees.quantity - oi_e_fees.refunded_quantity))) FILTER (WHERE p.is_box_office IS FALSE), 0) AS BIGINT)   AS client_online_fees_in_cents
FROM order_items oi
       LEFT JOIN orders o on oi.order_id = o.id
       LEFT JOIN events e on oi.event_id = e.id
       LEFT JOIN holds h ON oi.hold_id = h.id
       LEFT JOIN order_items oi_t_fees ON oi_t_fees.parent_id = oi.id AND oi_t_fees.item_type = 'PerUnitFees'
       LEFT JOIN order_items oi_e_fees ON oi_e_fees.order_id = oi.order_id AND oi_e_fees.item_type = 'EventFees'
       LEFT JOIN (SELECT order_id, CAST(ARRAY_TO_STRING(ARRAY_AGG(DISTINCT p.payment_method), ', ') LIKE '%External' AS BOOLEAN) AS is_box_office FROM payments p GROUP BY p.payment_method, p.order_id) AS p on o.id = p.order_id
       LEFT JOIN (SELECT tt.id, tt.name, tt.status FROM ticket_types tt WHERE $4 IS NOT TRUE) AS tt ON tt.id = oi.ticket_type_id
       LEFT JOIN (SELECT tp.id, tp.name FROM ticket_pricing tp WHERE $3 IS NOT TRUE) AS tp ON oi.ticket_pricing_id = tp.id
WHERE oi.ticket_type_id IS NOT NULL
  AND ($1 IS NULL OR o.paid_at >= $1)
  AND ($2 IS NULL OR o.paid_at <= $2)
GROUP BY e.id, tt.id, tt.name, tt.status, tp.id, tp.name;
$body$
  LANGUAGE SQL;


CREATE OR REPLACE FUNCTION ticket_count_per_ticket_type(event_id UUID, organization_id UUID, group_by_event_id BOOLEAN, group_by_organization_id BOOLEAN)
  RETURNS TABLE
          (
            organization_id                      UUID,
            event_id                             UUID,
            ticket_type_id                       UUID,
            ticket_name                          TEXT,
            ticket_status                        TEXT,
            event_name                           TEXT,
            organization_name                    TEXT,
            allocation_count_including_nullified BIGINT,
            allocation_count                     BIGINT,
            unallocated_count                    BIGINT,
            reserved_count                       BIGINT,
            redeemed_count                       BIGINT,
            purchased_count                      BIGINT,
            nullified_count                      BIGINT,
            available_for_purchase_count         BIGINT,
            total_refunded_count                 BIGINT,
            comp_count                           BIGINT,
            comp_available_count                 BIGINT,
            comp_redeemed_count                  BIGINT,
            comp_purchased_count                 BIGINT,
            comp_reserved_count                  BIGINT,
            comp_nullified_count                 BIGINT,
            hold_count                           BIGINT,
            hold_available_count                 BIGINT,
            hold_redeemed_count                  BIGINT,
            hold_purchased_count                 BIGINT,
            hold_reserved_count                  BIGINT,
            hold_nullified_count                 BIGINT
          )
AS
$body$
SELECT o.id                                                                                                                                                                                              AS organization_id,
       e.id                                                                                                                                                                                              AS event_id,
       tt.id                                                                                                                                                                                             AS ticket_type_id,
       tt.name                                                                                                                                                                                           AS ticket_name,
       tt.status                                                                                                                                                                                         AS ticket_status,
       e.name                                                                                                                                                                                            AS event_name,
       o.name                                                                                                                                                                                            AS organization_name,

       -- Total Ticket Count
       CAST(COALESCE(COUNT(ti.id), 0) AS BIGINT)                                                                                                                                                         AS allocation_count_including_nullified,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.status != 'Nullified'), 0) AS BIGINT)                                                                                                                 AS allocation_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.status = 'Available' OR (ti.status = 'Reserved' AND ti.reserved_until < NOW())), 0) AS BIGINT)                                                        AS unallocated_count,

       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.status = 'Reserved' AND ti.reserved_until > NOW()), 0) AS BIGINT)                                                                                     AS reserved_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.status = 'Redeemed'), 0) AS BIGINT)                                                                                                                   AS redeemed_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.status = 'Purchased'), 0) AS BIGINT)                                                                                                                  AS purchased_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.status = 'Nullified'), 0) AS BIGINT)                                                                                                                  AS nullified_count,
       -- Not in a hold and not purchased / reserved / redeemed etc
       -- What can a generic user purchase.
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NULL AND ti.status = 'Available' OR (ti.status = 'Reserved' AND ti.reserved_until < NOW())), 0) AS BIGINT)                                 AS available_for_purchase_count,
       --Refunded
       CAST(COUNT(rt.id) AS BIGINT)                                                                                                                                                                      AS total_refunded_count,
       -------------------- COMPS --------------------
       -- Comp counts
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp'), 0) AS BIGINT)                                                                                          AS comp_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND (ti.status = 'Available' OR (ti.status = 'Reserved' AND ti.reserved_until < NOW()))), 0) AS BIGINT)  AS comp_available_count,
       -- comp_count - comp_available_count = the sum of these
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status = 'Redeemed'), 0) AS BIGINT)                                                               AS comp_redeemed_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status = 'Purchased'), 0) AS BIGINT)                                                              AS comp_purchased_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status = 'Reserved' AND ti.reserved_until > NOW()), 0) AS BIGINT)                                 AS comp_reserved_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status = 'Nullified'), 0) AS BIGINT)                                                              AS comp_nullified_count,
       ------------------ END COMPS ------------------

       -------------------- HOLDS --------------------
       -- Hold Counts
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp'), 0) AS BIGINT)                                                                                         AS hold_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND (ti.status = 'Available' OR (ti.status = 'Reserved' AND ti.reserved_until < NOW()))), 0) AS BIGINT) AS hold_available_count,
       -- hold_count - hold_available_count = the sum of these
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status = 'Redeemed'), 0) AS BIGINT)                                                              AS hold_redeemed_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status = 'Purchased'), 0) AS BIGINT)                                                             AS hold_purchased_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status = 'Reserved' AND ti.reserved_until > NOW()), 0) AS BIGINT)                                AS hold_reserved_count,
       CAST(COALESCE(COUNT(ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status = 'Nullified'), 0) AS BIGINT)                                                             AS hold_nullified_count
       ------------------ END HOLDS -------------------
FROM ticket_instances ti
       LEFT JOIN holds h ON (h.id = ti.hold_id)
       LEFT JOIN refunded_tickets rt ON (rt.ticket_instance_id = ti.id)
       LEFT JOIN assets a ON (a.id = ti.asset_id)
       LEFT JOIN (SELECT tt.id, tt.name, tt.status FROM ticket_types tt WHERE $3 IS NOT TRUE AND $4 IS NOT TRUE) AS tt ON tt.id = a.ticket_type_id
       LEFT JOIN ticket_types tt2 ON a.ticket_type_id = tt2.id
       LEFT JOIN (SELECT e.id, e.organization_id, e.name FROM events e WHERE $4 IS NOT TRUE) AS e ON (e.id = tt2.event_id)
       LEFT JOIN events e2 ON (e2.id = tt2.event_id)
       LEFT JOIN organizations o ON o.id = e2.organization_id
WHERE ($1 IS NULL OR e.id = $1)
  AND ($2 IS NULL OR e.organization_id = $2)
GROUP BY e.id, e.name,o.id, o.name,tt.id, tt.name, tt.status;
$body$
  LANGUAGE SQL;