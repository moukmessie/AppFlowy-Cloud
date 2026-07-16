-- System administrators are represented by GoTrue app metadata so the role is
-- included in freshly issued JWTs and remains compatible with GoTrue's legacy
-- is_super_admin column.
--
-- Existing installations bootstrap the earliest GoTrue user. Fresh installs
-- with no users are handled when the first user is verified by appflowy-cloud.
DO $$
DECLARE
    first_user_id UUID;
BEGIN
    SELECT id
      INTO first_user_id
      FROM auth.users
     ORDER BY created_at ASC
     LIMIT 1;

    IF first_user_id IS NULL THEN
        RAISE NOTICE 'No users found in auth.users; skipping system admin backfill';
        RETURN;
    END IF;

    UPDATE auth.users
       SET raw_app_meta_data =
           CASE
               WHEN raw_app_meta_data IS NULL
                 OR jsonb_typeof(raw_app_meta_data) <> 'object'
               THEN jsonb_build_object('is_system_admin', true)
               ELSE raw_app_meta_data || jsonb_build_object('is_system_admin', true)
           END
     WHERE id = first_user_id
       AND NOT (
           COALESCE(is_super_admin, false)
           OR COALESCE((raw_app_meta_data->>'is_system_admin')::boolean, false)
       );
END
$$;

CREATE OR REPLACE FUNCTION public.af_is_system_admin(user_uuid UUID)
RETURNS BOOLEAN
LANGUAGE SQL
STABLE
SECURITY DEFINER
SET search_path = public, auth
AS $$
    SELECT COALESCE(
        (
            SELECT COALESCE(is_super_admin, false)
                OR COALESCE((raw_app_meta_data->>'is_system_admin')::boolean, false)
              FROM auth.users
             WHERE id = user_uuid
        ),
        false
    );
$$;

REVOKE ALL ON FUNCTION public.af_is_system_admin(UUID) FROM PUBLIC;
