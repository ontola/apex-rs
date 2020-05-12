use crate::db::db_context::DbContext;
use crate::db::models::{Datatype, Predicate};
use crate::db::schema;
use crate::hashtuple::{HashModel, LookupTable};
use diesel::{insert_into, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, Table};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

const MAX_PROPERTY_INSERT_SIZE: usize = 60_000 / 8;

pub(crate) fn insert_properties(
    ctx: &mut DbContext,
    lookup_table: &LookupTable,
    model: &HashModel,
    resource_id_map: HashMap<String, i64>,
) {
    use schema::properties::dsl;

    let mut properties = vec![];

    if model.len() > 65000 {
        error!(
            "Giant model detected (properties: {}, id: {})",
            model.len(),
            lookup_table.get_by_hash(model[0].subject)
        );
        dump_model_to_screen(&lookup_table, &model);
    }

    for h in model {
        let resource_id = *resource_id_map
            .get(lookup_table.get_by_hash(h.subject))
            .expect("Inserting property not inserted in resources");

        let predicate = lookup_table.get_by_hash(h.predicate);
        if !ctx.property_map.contains_key(predicate) {
            insert_and_update_predicate(&ctx.get_conn(), &mut ctx.property_map, predicate);
        }

        let datatype = lookup_table.get_by_hash(h.datatype);
        if !ctx.datatype_map.contains_key(datatype) {
            insert_and_update_datatype(&ctx.get_conn(), &mut ctx.datatype_map, datatype);
        }

        let language = lookup_table.get_by_hash(h.language);
        if !ctx.language_map.contains_key(language) {
            insert_and_update_language(&ctx.get_conn(), &mut ctx.language_map, language);
        }

        let pred_id: i32 = *ctx.property_map.get_mut(predicate).unwrap();

        properties.push((
            dsl::resource_id.eq(resource_id),
            dsl::predicate_id.eq(pred_id),
            //            dsl::order.eq(None),
            dsl::value.eq(lookup_table.get_by_hash(h.value)),
            dsl::datatype_id.eq(*(&mut ctx.datatype_map)
                .get(lookup_table.get_by_hash(h.datatype))
                .unwrap_or_else(|| {
                    panic!(
                        "Data type not found in map ({})",
                        lookup_table.get_by_hash(h.datatype)
                    )
                })),
            dsl::language_id.eq(*(&mut ctx.language_map)
                .get(lookup_table.get_by_hash(h.language))
                .unwrap_or_else(|| {
                    panic!(
                        "Language not found in map ({})",
                        lookup_table.get_by_hash(h.language)
                    )
                })),
            //            dsl::prop_resource.eq(None),
        ));
    }

    properties
        .chunks(MAX_PROPERTY_INSERT_SIZE)
        .for_each(|chunk| {
            insert_into(schema::properties::table)
                .values(chunk)
                .execute(&ctx.get_conn())
                .expect("Error while inserting into resources");
        });
}

fn insert_and_update_predicate(
    db_conn: &PgConnection,
    map: &mut HashMap<String, i32>,
    insert_value: &str,
) -> i32 {
    use schema::predicates::dsl::*;

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
    use schema::datatypes::dsl::*;

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

fn insert_and_update_language(
    db_conn: &PgConnection,
    map: &mut HashMap<String, i32>,
    insert_value: &str,
) -> i32 {
    use schema::languages::dsl::*;

    let target = value.eq(insert_value);
    let p = insert_into(languages)
        .values(vec![(&target)])
        .get_result::<Datatype>(db_conn)
        .unwrap_or_else(|_| {
            languages
                .filter(&target)
                .get_result(db_conn)
                .unwrap_or_else(|_| panic!("Datatype not found {}", insert_value))
        });
    map.entry(p.value).or_insert(p.id);

    p.id
}

fn dump_model_to_screen(lookup_table: &LookupTable, model: &HashModel) {
    let mut output: String = String::from("");
    thread::sleep(Duration::new(5, 0));

    for hashtuple in model {
        output += format!(
            "({}, {}, {}, {}, {}, {})\n",
            lookup_table.get_by_hash(hashtuple.subject),
            lookup_table.get_by_hash(hashtuple.predicate),
            lookup_table.get_by_hash(hashtuple.value),
            lookup_table.get_by_hash(hashtuple.datatype),
            lookup_table.get_by_hash(hashtuple.language),
            lookup_table.get_by_hash(hashtuple.graph),
        )
        .as_ref();
    }

    error!("{}\n", output);
}
