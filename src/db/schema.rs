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
    properties (id) {
        id -> Int8,
        resource_id -> Int8,
        predicate_id -> Int4,
        order -> Int4,
        prop_resource -> Nullable<Int8>,
        datatype_id -> Int4,
        language_id -> Nullable<Int4>,
        value -> Varchar,
    }
}

table! {
    resources (id) {
        id -> Int8,
        document_id -> Int8,
        iri -> Varchar,
    }
}

joinable!(properties -> resources (resource_id));
joinable!(resources -> documents (document_id));

allow_tables_to_appear_in_same_query!(
    datatypes, documents, languages, predicates, properties, resources,
);
