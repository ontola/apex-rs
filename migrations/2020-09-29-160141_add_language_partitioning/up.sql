-- Your SQL goes here
TRUNCATE TABLE public.documents RESTART IDENTITY CASCADE;

ALTER TABLE public.documents
    ADD COLUMN language character(4) NOT NULL;
