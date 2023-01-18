-- if user_id is null, find all reservations within during for the resource
-- if resource_id is null, find all reservations within during for the user
-- if both are null, find all reservation within during
-- if both set, find all reservation within during for the resource and user
CREATE OR REPLACE FUNCTION rsvp.query(
    uid text, 
    rid text, 
    _start timestamp with time zone,
    _end timestamp with time zone,
    status rsvp.reservation_status DEFAULT 'pending',
    is_desc bool DEFAULT FALSE
) RETURNS TABLE (LIKE rsvp.reservations) AS $$ 
DECLARE
    _during TSTZRANGE;
    _sql text;
BEGIN
    -- if start or end is null, use infinity
    _during := TSTZRANGE(
        COALESCE(_start, '-infinity'),
        COALESCE(_end, 'infinity'),
        '[)'
    );

    -- format the query based on parameters
    _sql := format('SELECT * FROM rsvp.reservations WHERE %L @> timespan AND status = %L AND %s ORDER BY lower(timespan) %s',
        _during,
        status,
        CASE
            WHEN uid IS NULL AND rid IS NULL THEN 'TRUE'
            WHEN uid IS NULL THEN 'resource_id = ' || quote_literal(rid)
            WHEN rid IS NULL THEN 'user_id = ' || quote_literal(uid)
            ELSE 'user_id = ' || quote_literal(uid) || ' AND resource_id = ' || quote_literal(rid)
        END,
        CASE 
            WHEN is_desc THEN 'DESC'
            ELSE 'ASC'
        END
    );
    -- log the sql
    RAISE NOTICE '%', _sql;

    -- excute the query
    RETURN QUERY EXECUTE _sql; 
END;
$$ LANGUAGE plpgsql;

