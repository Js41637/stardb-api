DELETE FROM gi_wishes_character
WHERE uid = $1
    AND NOT official;

