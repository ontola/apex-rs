-- Your SQL goes here
CREATE INDEX iri_id
    ON public.documents USING btree
        (iri, id)
;

-- TODO replace with hash + INCLUDE id when we use 10+