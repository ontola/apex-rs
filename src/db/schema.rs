table! {
    _apex_config (key) {
        key -> Text,
        value -> Text,
    }
}

table! {
    datatypes (id) {
        id -> Int4,
        value -> Varchar,
    }
}

table! {
    documents (id) {
        id -> Int8,
        iri -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        cache_control -> Int2,
    }
}

table! {
    languages (id) {
        id -> Int4,
        value -> Varchar,
    }
}

table! {
    predicates (id) {
        id -> Int4,
        value -> Varchar,
    }
}

table! {
    objects (hash) {
        hash -> Uuid,
        value -> Varchar,
    }
}

table! {
    properties (id) {
        id -> Int8,
        resource_id -> Int8,
        predicate_id -> Int4,
        order -> Int4,
        prop_resource -> Nullable<Int8>,
        datatype_id -> Int4,
        language_id -> Nullable<Int4>,
        value -> Varchar,
        object_id -> Nullable<Uuid>,
    }
}

table! {
    resources (id) {
        id -> Int8,
        document_id -> Int8,
        iri -> Varchar,
    }
}

joinable!(properties -> objects (object_id));
joinable!(properties -> resources (resource_id));
joinable!(resources -> documents (document_id));

allow_tables_to_appear_in_same_query!(
    datatypes, documents, languages, predicates, properties, resources, objects
);
