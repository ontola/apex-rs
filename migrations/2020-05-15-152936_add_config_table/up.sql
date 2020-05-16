-- Your SQL goes here
CREATE TABLE public._apex_config
(
    key text NOT NULL,
    value text NOT NULL,
    PRIMARY KEY (key),
    CONSTRAINT key_uniq UNIQUE (key)
)
    WITH (
        OIDS = FALSE
    );

ALTER TABLE public._apex_config
    OWNER to postgres;
