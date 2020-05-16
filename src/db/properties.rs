use crate::db::db_context::DbContext;
use crate::db::models::{Datatype, Object, Predicate};
use crate::db::schema;
use crate::db::uu128::Uu128;
use crate::hashtuple::{HashModel, LookupTable};
use diesel::{insert_into, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

const MAX_PROPERTY_INSERT_SIZE: usize = 60_000 / 8;

pub(crate) fn insert_properties(
    ctx: &mut DbContext,
    model: &HashModel,
    resource_id_map: HashMap<String, i64>,
) {
    use schema::properties::dsl;

    let mut properties = vec![];

    if model.len() > 65000 {
        error!(
            "Giant model detected (properties: {}, id: {})",
            model.len(),
            ctx.lookup_table.get_by_hash(model[0].subject)
        );
        dump_model_to_screen(&ctx.lookup_table, &model);
    }

    let mut values = HashSet::new();

    for h in model {
        let resource_id = *resource_id_map
            .get(ctx.lookup_table.get_by_hash(h.subject))
            .expect("Inserting property not inserted in resources");

        let predicate = ctx.lookup_table.get_by_hash(h.predicate);
        if !ctx.property_map.contains_key(predicate) {
            insert_and_update_predicate(&ctx.get_conn(), &mut ctx.property_map, predicate);
        }

        values.insert(Object {
            hash: Uu128::from(h.value),
            value: String::from(ctx.lookup_table.get_by_hash(h.value)),
        });

        let datatype = ctx.lookup_table.get_by_hash(h.datatype);
        if !ctx.datatype_map.contains_key(datatype) {
            insert_and_update_datatype(&ctx.get_conn(), &mut ctx.datatype_map, datatype);
        }
        let datatype_id = *(&mut ctx.datatype_map)
            .get(ctx.lookup_table.get_by_hash(h.datatype))
            .unwrap_or_else(|| panic!("Data type not found in map ({})", h.datatype));

        let language = ctx.lookup_table.get_by_hash(h.language);
        if !ctx.language_map.contains_key(language) {
            insert_and_update_language(&ctx.get_conn(), &mut ctx.language_map, language);
        }
        let language_id = *(&mut ctx.language_map)
            .get(ctx.lookup_table.get_by_hash(h.language))
            .unwrap_or_else(|| panic!("Language not found in map ({})", h.language));

        let pred_id: i32 = *ctx.property_map.get_mut(predicate).unwrap();

        properties.push((
            dsl::resource_id.eq(resource_id),
            dsl::predicate_id.eq(pred_id),
            //            dsl::order.eq(None),
            dsl::value.eq(""),
            dsl::object_id.eq(Uu128::from(h.value)),
            dsl::datatype_id.eq(datatype_id),
            dsl::language_id.eq(language_id),
            //            dsl::prop_resource.eq(None),
        ));
    }

    values
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(MAX_PROPERTY_INSERT_SIZE)
        .for_each(|chunk| {
            insert_into(schema::objects::table)
                .values(chunk)
                .on_conflict_do_nothing()
                .execute(&ctx.get_conn())
                .expect("Error while inserting into objects");
        });

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
