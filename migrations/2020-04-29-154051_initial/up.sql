CREATE TABLE documents
(
    id BIGSERIAL PRIMARY KEY,
    iri character varying(2000) NOT NULL,
    CONSTRAINT documents_id_unique UNIQUE (id)
);

-- ALTER TABLE public.documents
--     OWNER to postgres;

CREATE INDEX id_index
    ON documents USING hash
    (id);

CREATE TABLE resources
(
    id BIGSERIAL PRIMARY KEY,
    document_id BIGINT NOT NULL,
    iri character varying(2000) NOT NULL,
    CONSTRAINT resources_id_unique UNIQUE (id),
    CONSTRAINT fk_resources_document_id_id FOREIGN KEY (document_id)
        REFERENCES documents (id) MATCH SIMPLE
        ON UPDATE RESTRICT
        ON DELETE CASCADE
);

-- ALTER TABLE public.resources
--     OWNER to postgres;

CREATE INDEX resources_document_id
    ON resources USING btree
    (document_id ASC NULLS LAST);


CREATE TABLE datatypes
(
    id SERIAL PRIMARY KEY,
    value character varying(1024) NOT NULL,
    CONSTRAINT datatypes_value_uniq UNIQUE (value)
);

-- ALTER TABLE public.datatypes
--     OWNER to postgres;


CREATE TABLE predicates
(
    id SERIAL PRIMARY KEY,
    value character varying(2048) NOT NULL,
    CONSTRAINT predicates_value_uniq UNIQUE (value)
);

-- ALTER TABLE public.predicates
--     OWNER to postgres;


CREATE TABLE languages
(
    id SERIAL PRIMARY KEY,
    value character varying(255) NOT NULL
);

-- ALTER TABLE public.languages
--     OWNER to postgres;


CREATE TABLE public.properties
(
    id bigserial NOT NULL,
    resource_id bigint NOT NULL,
    predicate_id integer NOT NULL,
    "order" integer NOT NULL DEFAULT 0,
    prop_resource bigint,
    datatype_id integer NOT NULL,
    language_id integer NOT NULL,
    value text NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT properties_datatype_id FOREIGN KEY (datatype_id)
        REFERENCES public.datatypes (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
        DEFERRABLE INITIALLY DEFERRED
        NOT VALID,
    CONSTRAINT properties_language_id FOREIGN KEY (language_id)
        REFERENCES public.languages (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
        DEFERRABLE INITIALLY DEFERRED
        NOT VALID,
    CONSTRAINT properties_resource_id FOREIGN KEY (resource_id)
        REFERENCES public.resources (id) MATCH SIMPLE
        ON UPDATE RESTRICT
        ON DELETE CASCADE
        DEFERRABLE INITIALLY DEFERRED
        NOT VALID
);

-- ALTER TABLE public.properties
--     OWNER to postgres;

CREATE INDEX properties_pred_dt_resource
    ON properties USING btree
    (predicate_id ASC NULLS LAST, datatype_id ASC NULLS LAST, resource_id ASC NULLS LAST);


CREATE INDEX properties_resource_id
    ON properties USING btree
    (resource_id ASC NULLS LAST);

