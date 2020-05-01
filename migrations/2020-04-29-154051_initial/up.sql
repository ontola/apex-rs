-- Your SQL goes here

CREATE TABLE documents
(
    id BIGSERIAL PRIMARY KEY,
    iri character varying(2000) NOT NULL,
    CONSTRAINT documents_id_unique UNIQUE (id)
)
WITH (
    OIDS = FALSE
);

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
)
WITH (
    OIDS = FALSE
);

CREATE INDEX resources_document_id
    ON resources USING btree
    (document_id ASC NULLS LAST);


CREATE TABLE datatypes
(
    id SERIAL PRIMARY KEY,
    value character varying(1024) NOT NULL,
    CONSTRAINT datatypes_value_uniq UNIQUE (value)
)
WITH (
    OIDS = FALSE
);


CREATE TABLE predicates
(
    id SERIAL PRIMARY KEY,
    value character varying(2048) NOT NULL,
    CONSTRAINT predicates_value_uniq UNIQUE (value)
)
WITH (
    OIDS = FALSE
);


CREATE TABLE languages
(
    id SERIAL PRIMARY KEY,
    value character varying(255) NOT NULL
)
WITH (
    OIDS = FALSE
);


CREATE TABLE properties
(
    id BIGSERIAL PRIMARY KEY,
    resource_id BIGINT,
    predicate_id integer NOT NULL,
    "order" integer DEFAULT 0,
    prop_resource BIGINT,
    datatype_id integer NOT NULL,
    language_id integer,
    value character varying(10000000) NOT NULL,
    CONSTRAINT fk_properties_resource_id_id FOREIGN KEY (resource_id)
        REFERENCES resources (id) MATCH SIMPLE
        ON UPDATE RESTRICT
        ON DELETE CASCADE
)
WITH (
    OIDS = FALSE
);

CREATE INDEX properties_pred_dt_resource
    ON properties USING btree
    (predicate_id ASC NULLS LAST, datatype_id ASC NULLS LAST, resource_id ASC NULLS LAST);


CREATE INDEX properties_resource_id
    ON properties USING btree
    (resource_id ASC NULLS LAST);

