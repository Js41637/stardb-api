DELETE FROM gi_wishes_beginner
WHERE uid = $1
    AND NOT official;

