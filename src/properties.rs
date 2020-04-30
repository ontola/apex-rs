use crate::db_context::DbContext;
use crate::hashtuple::{HashModel, LookupTable};
use crate::models::{Datatype, Predicate};
use diesel::{insert_into, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use std::collections::HashMap;

pub(crate) fn insert_properties(
    ctx: &mut DbContext,
    lookup_table: &LookupTable,
    model: &HashModel,
    resource_id_map: HashMap<String, i64>,
) {
    use crate::schema::properties::dsl;

    let mut properties = vec![];

    for h in model {
        let resource_id = *resource_id_map
            .get(lookup_table.get_by_hash(h[0]))
            .expect("Inserting property not inserted in resources");

        let predicate = lookup_table.get_by_hash(h[1]);
        if !ctx.property_map.contains_key(predicate) {
            insert_and_update(ctx.db_conn, &mut ctx.property_map, predicate);
        }
        let datatype = lookup_table.get_by_hash(h[3]);
        if !ctx.datatype_map.contains_key(datatype) {
            insert_and_update_datatype(ctx.db_conn, &mut ctx.datatype_map, datatype);
        }

        let pred_id: i32 = *ctx.property_map.get_mut(predicate).unwrap();

        properties.push((
            dsl::resource_id.eq(resource_id),
            dsl::predicate_id.eq(pred_id),
            //            dsl::order.eq(None),
            dsl::value.eq(lookup_table.get_by_hash(h[2])),
            dsl::datatype_id.eq(*(&mut ctx.datatype_map)
                .get(lookup_table.get_by_hash(h[3]))
                .unwrap_or_else(|| {
                    panic!(
                        "Data type not found in map ({})",
                        lookup_table.get_by_hash(h[3])
                    )
                })),
            //            dsl::language_id.eq(Some(0)),
            //            dsl::prop_resource.eq(None),
        ));
    }

    insert_into(crate::schema::properties::table)
        .values(&properties)
        .execute(ctx.db_conn)
        .expect("Error while inserting into resources");
}

fn insert_and_update(
    db_conn: &PgConnection,
    map: &mut HashMap<String, i32>,
    insert_value: &str,
) -> i32 {
    use crate::schema::predicates::dsl::*;

    let target = value.eq(insert_value);
    let p = insert_into(predicates)
        .values(vec![(&target)])
        .get_result::<Predicate>(db_conn)
        .unwrap_or_else(|_| {
            predicates
                .filter(&target)
                .get_result(db_conn)
                .unwrap_or_else(|_| panic!("Predicate not found {}", insert_value))
        });
    map.entry(p.value).or_insert(p.id);

    p.id
}

fn insert_and_update_datatype(
    db_conn: &PgConnection,
    map: &mut HashMap<String, i32>,
    insert_value: &str,
) -> i32 {
    use crate::schema::datatypes::dsl::*;

    let target = value.eq(insert_value);
    let p = insert_into(datatypes)
        .values(vec![(&target)])
        .get_result::<Datatype>(db_conn)
        .unwrap_or_else(|_| {
            datatypes
                .filter(&target)
                .get_result(db_conn)
                .unwrap_or_else(|_| panic!("Datatype not found {}", insert_value))
        });
    map.entry(p.value).or_insert(p.id);

    p.id
}
