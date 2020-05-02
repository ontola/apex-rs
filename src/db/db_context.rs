use crate::db::models::{Datatype, Predicate};
use crate::db::schema;
use diesel::prelude::*;
use diesel::PgConnection;
use std::collections::HashMap;

pub(crate) type IRIMapping = HashMap<String, i32>;

pub(crate) struct DbContext<'a> {
    pub db_conn: &'a PgConnection,
    pub property_map: IRIMapping,
    pub datatype_map: IRIMapping,
}

pub(crate) fn create_context(db_conn: &diesel::PgConnection) -> DbContext {
    DbContext {
        db_conn,
        property_map: get_predicates(&db_conn),
        datatype_map: get_datatypes(&db_conn),
    }
}

/**
 * Retrieve a map of data type IRIs to their ids from the db.
 */
fn get_datatypes(db_conn: &PgConnection) -> IRIMapping {
    use schema::datatypes::dsl::*;

    let mut map = HashMap::new();
    let props = datatypes
        .limit(100_000)
        .load::<Datatype>(db_conn)
        .expect("Could not fetch datatypes");

    for p in props {
        map.entry(p.value.clone()).or_insert(p.id);
    }

    map
}

/**
 * Retrieve a map of predicate IRIs to their ids from the db.
 */
fn get_predicates(db_conn: &PgConnection) -> IRIMapping {
    use schema::predicates::dsl::*;

    let mut map = HashMap::new();
    let props = predicates
        .limit(100_000)
        .load::<Predicate>(db_conn)
        .expect("Could not fetch properties");

    for p in props {
        map.entry(p.value.clone()).or_insert(p.id);
    }

    map
}
