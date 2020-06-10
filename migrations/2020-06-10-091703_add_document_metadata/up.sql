-- Your SQL goes here

ALTER TABLE public.documents
    ADD COLUMN created_at timestamp with time zone NOT NULL DEFAULT NOW();

ALTER TABLE public.documents
    ADD COLUMN updated_at timestamp with time zone NOT NULL DEFAULT NOW();

ALTER TABLE public.documents
    ADD COLUMN cache_control smallint NOT NULL DEFAULT 0;
