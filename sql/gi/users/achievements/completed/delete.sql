DELETE FROM gi_users_achievements_completed
WHERE username = $1
    AND id = $2;

