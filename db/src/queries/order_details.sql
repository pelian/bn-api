SELECT
    ticket_instance_id,
    order_item_id,
    description,
    ticket_price_in_cents,
    fees_price_in_cents,
    (ticket_price_in_cents + fees_price_in_cents) as total_price_in_cents,
    status,
    status IN ('Purchased', 'Redeemed') as refundable
FROM (
    SELECT
        t.ticket_instance_id,
        t.order_item_id as order_item_id,
        CASE
            WHEN oi.item_type = 'EventFees' THEN 'Event Fees - ' || e.name
            ELSE e.name || ' - ' || tt.name
        END AS description,
        CASE
            WHEN oi.quantity = oi.refunded_quantity or rt.ticket_refunded_at is not null THEN 0
            WHEN oi.item_type = 'EventFees' THEN 0
            ELSE oi.unit_price_in_cents
        END as ticket_price_in_cents,
        CASE
            WHEN oi.quantity = oi.refunded_quantity or rt.fee_refunded_at is not null THEN 0
            WHEN fi.unit_price_in_cents is null THEN oi.company_fee_in_cents + oi.client_fee_in_cents
            ELSE fi.unit_price_in_cents
        END as fees_price_in_cents,
        CASE
            WHEN oi.quantity = oi.refunded_quantity or rt.ticket_refunded_at is not null THEN 'Refunded'
            WHEN w.user_id <> o.user_id THEN 'Transferred'
            WHEN ti.status is null THEN 'Purchased'
            ELSE ti.status
        END as status
    FROM
       (
        select distinct ticket_instance_id, order_item_id from (
            select null as ticket_instance_id, oi.id as order_item_id from order_items oi where oi.order_id = $1 and oi.item_type = 'EventFees'
            union select rt.ticket_instance_id, rt.order_item_id from refunded_tickets rt join order_items oi on rt.order_item_id = oi.id where oi.order_id = $1
            union select ti.id as ticket_instance_id, ti.order_item_id from ticket_instances ti join order_items oi on ti.order_item_id = oi.id where oi.order_id = $1 and oi.item_type = 'Tickets'
        ) t
       ) t
       INNER JOIN order_items oi ON t.order_item_id = oi.id
       LEFT JOIN ticket_instances ti ON ti.id = t.ticket_instance_id
       INNER JOIN events e ON oi.event_id = e.id
       INNER JOIN organizations orgs ON e.organization_id = orgs.id
       LEFT JOIN ticket_pricing tp ON oi.ticket_pricing_id = tp.id
       INNER JOIN orders o on oi.order_id = o.id
       LEFT JOIN users u on u.id = $3
       LEFT JOIN organization_users ou ON orgs.id = ou.organization_id AND ou.user_id = u.id
       LEFT JOIN ticket_types tt ON tp.ticket_type_id = tt.id
       LEFT JOIN holds h ON oi.hold_id = h.id
       LEFT JOIN wallets w on ti.wallet_id = w.id
       LEFT JOIN refunded_tickets rt on rt.ticket_instance_id = ti.id
       LEFT JOIN order_items fi on fi.parent_id = oi.id and fi.item_type = 'PerUnitFees'
       WHERE e.organization_id = ANY($2)
       AND o.status in ('Paid', 'PartiallyPaid')
       AND (
         o.user_id = $3
         OR o.on_behalf_of_user_id = $3
         OR 'Admin' = ANY(u.role)
         OR (
             (
               NOT ('Promoter' = ANY(ou.role))
               AND NOT ('PromoterReadOnly' = ANY(ou.role))
             )
             OR e.id = ANY(ou.event_ids)
         )
       )
    ORDER BY oi.event_id, oi.item_type DESC, t.ticket_instance_id
) details
