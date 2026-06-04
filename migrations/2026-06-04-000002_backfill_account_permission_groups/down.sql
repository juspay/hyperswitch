-- Not reversible: `account_view` / `account_manage` are pre-existing groups, so a
-- backfilled membership can't be told apart from one a role already had. Stripping
-- them here would revoke access from roles that legitimately held them.
SELECT 1;
