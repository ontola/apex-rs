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
    datatypes {
        id -> Integer,
        value -> VarChar,
    }
}

table! {
    predicates {
        id -> Integer,
        value -> VarChar,
    }
}

table! {
    languages {
        id -> Integer,
        value -> VarChar,
    }
}

table! {
    properties(resource_id, predicate_id, order)  {
        resource_id -> Integer,
        predicate_id -> Integer,
        order -> Nullable<Integer>,
        value -> VarChar,
        datatype_id -> Integer,
        language_id -> Nullable<Integer>,
        prop_resource -> Nullable<Integer>,
    }
}

joinable!(properties -> predicates (predicate_id));
joinable!(properties -> datatypes (datatype_id));
joinable!(properties -> resources (resource_id));

allow_tables_to_appear_in_same_query!(properties, resources);
allow_tables_to_appear_in_same_query!(properties, documents);
allow_tables_to_appear_in_same_query!(properties, datatypes);
allow_tables_to_appear_in_same_query!(properties, predicates);
allow_tables_to_appear_in_same_query!(properties, languages);
