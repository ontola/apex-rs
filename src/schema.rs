table! {
    documents {
        id -> Integer,
        iri -> VarChar,
    }
}

table! {
    resources {
        id -> Integer,
        document_id -> Integer,
        iri -> VarChar,
    }
}
joinable!(resources -> documents (document_id));
allow_tables_to_appear_in_same_query!(resources, documents);

table! {
    properties(resource_id, predicate, order)  {
        resource_id -> Integer,
        predicate -> VarChar,
        order -> Nullable<Integer>,
        value -> VarChar,
        datatype -> VarChar,
        language -> VarChar,
        prop_resource -> Nullable<Integer>,
    }
}

joinable!(properties -> resources (resource_id));

allow_tables_to_appear_in_same_query!(resources, properties);
allow_tables_to_appear_in_same_query!(documents, properties);
