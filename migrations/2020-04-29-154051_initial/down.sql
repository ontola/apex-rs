-- This file should undo anything in `up.sql`

DROP INDEX properties_pred_dt_resource;
DROP INDEX properties_resource_id;
DROP TABLE properties;

DROP TABLE datatypes;

DROP TABLE predicates;

DROP TABLE languages;

DROP INDEX resources_document_id;
DROP TABLE resources;

DROP INDEX id_index;
DROP TABLE documents;
