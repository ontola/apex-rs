use crate::hashtuple::{HashModel, LookupTable};
use crate::models::{NewResource, Resource};
use diesel::{insert_into, PgConnection, RunQueryDsl};
use std::collections::HashSet;

pub(crate) fn insert_resources(
    db_conn: &PgConnection,
    lookup_table: &LookupTable,
    model: &HashModel,
    id: i64,
) -> Vec<Resource> {
    let mut resource_iris = HashSet::new();
    for hex in model {
        resource_iris.insert(hex[0]);
    }
    let mut new_resources = vec![];
    for r_iri in resource_iris {
        new_resources.push(NewResource {
            document_id: id,
            iri: lookup_table.get_by_hash(r_iri).clone(),
        });
    }

    insert_into(crate::schema::resources::table)
        .values(&new_resources)
        .get_results(db_conn)
        .expect("Error while inserting into resources")
}
