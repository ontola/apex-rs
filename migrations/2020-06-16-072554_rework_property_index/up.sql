-- Your SQL goes here

DROP INDEX properties_pred_dt_resource;
DROP INDEX properties_resource_id;

CREATE INDEX properties_rdf_object
    ON public.properties USING btree
        (object_id ASC NULLS LAST, datatype_id ASC NULLS LAST, language_id ASC NULLS LAST);

CREATE INDEX properties_resources
    ON public.properties USING btree
        (resource_id ASC NULLS LAST);

CREATE INDEX properties_predicates
    ON public.properties USING btree
        (predicate_id ASC NULLS LAST);
