DELETE FROM gi_wishes_chronicled
WHERE uid = $1
    AND NOT official;

