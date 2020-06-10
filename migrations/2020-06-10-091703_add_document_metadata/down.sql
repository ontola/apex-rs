-- This file should undo anything in `up.sql`
ALTER TABLE public.documents DROP COLUMN created_at;

ALTER TABLE public.documents DROP COLUMN updated_at;

ALTER TABLE public.documents DROP COLUMN cache_control;