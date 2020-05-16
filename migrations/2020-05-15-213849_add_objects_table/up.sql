-- Your SQL goes here
CREATE TABLE public.objects
(
    hash uuid NOT NULL,
    value text NOT NULL,
    PRIMARY KEY (hash),
    CONSTRAINT hash_uniq UNIQUE (hash)
)
    WITH (
        OIDS = FALSE
    );

ALTER TABLE public.objects
    OWNER to postgres;


ALTER TABLE public.properties
    ADD COLUMN object_id uuid;
ALTER TABLE public.properties
    ADD CONSTRAINT properties_object_id FOREIGN KEY (object_id)
        REFERENCES public.objects (hash) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
        DEFERRABLE INITIALLY DEFERRED;


