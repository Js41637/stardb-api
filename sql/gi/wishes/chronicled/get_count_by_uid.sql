SELECT
    count(*)
FROM
    gi_wishes_chronicled
WHERE
    uid = $1;

