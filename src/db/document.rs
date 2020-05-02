use crate::db::db_context::DbContext;
use crate::db::models::*;
use crate::db::schema;
use crate::hashtuple::{HashModel, LookupTable};
use diesel::insert_into;
use diesel::prelude::*;

pub(crate) fn get_doc_count(db_conn: &PgConnection) {
    use diesel::dsl;
    use schema::documents::dsl::*;
    use schema::properties::dsl::*;
    use schema::resources::dsl::*;

    let doc_count: i64 = documents.select(dsl::count_star()).first(db_conn).unwrap();
    let resource_count: i64 = resources.select(dsl::count_star()).first(db_conn).unwrap();
    let property_count: i64 = properties.select(dsl::count_star()).first(db_conn).unwrap();

    println!(
        "we got {:?} documents, {:?} resources and {:?} properties",
        doc_count, resource_count, property_count
    );
}

pub fn doc_by_id<'a>(
    ctx: &DbContext,
    lookup_table: &'a mut LookupTable,
    id: i64,
) -> Option<HashModel> {
    let doc = get_document(&ctx.get_conn(), id);
    let first = doc.first();

    if doc.is_empty() {
        debug!(target: "app", "Doc is empty");
        return None;
    }

    let mut props: HashModel = vec![];
    let (_, resources) = first.unwrap();
    for (resource, properties) in resources {
        for p in properties {
            let predicate = ctx
                .property_map
                .iter()
                .find(|(_, v)| **v == p.predicate_id)
                .unwrap()
                .0;
            let datatype = ctx
                .datatype_map
                .iter()
                .find(|(_, v)| **v == p.datatype_id)
                .unwrap()
                .0;
            props.push([
                lookup_table.ensure_value(&resource.iri),
                lookup_table.ensure_value(predicate),
                lookup_table.ensure_value(&p.value),
                lookup_table.ensure_value(datatype),
                lookup_table.ensure_value(&String::from(EMPTY_STRING)), // p.language.clone()
                lookup_table.ensure_value(&String::from(EMPTY_STRING)),
            ]);
        }
    }

    Some(props)
}

const EMPTY_STRING: &str = "";

pub(crate) fn reset_document<'a>(
    ctx: &DbContext,
    lookup_table: &'a mut LookupTable,
    id: i64,
) -> HashModel {
    match doc_by_id(&ctx, lookup_table, id) {
        None => {
            let doc = &Document {
                id,
                iri: format!("https://id.openraadsinformatie.nl/{}", id),
            };
            insert_into(schema::documents::table)
                .values(doc)
                .execute(&ctx.get_conn())
                .expect("Error while inserting into documents");

            vec![]
        }
        Some(model) => {
            delete_document_data(&ctx.get_conn(), id);
            model
        }
    }
}

fn delete_document_data(db_conn: &PgConnection, doc_id: i64) {
    use schema::properties;
    use schema::resources::dsl::*;

    let resource_ids = resources
        .select(id)
        .filter(document_id.eq(doc_id))
        .get_results::<i64>(db_conn)
        .expect("Could not fetch resource ids for document");

    let props = properties::dsl::resource_id.eq_any(&resource_ids);
    diesel::delete(properties::table)
        .filter(props)
        .execute(db_conn)
        .expect("Couldn't delete existing properties");

    diesel::delete(resources)
        .filter(id.eq_any(&resource_ids))
        .execute(db_conn)
        .expect("Couldn't delete existing resources");
}

fn get_document(
    db_conn: &PgConnection,
    doc_id: i64,
) -> Vec<(Document, Vec<(Resource, Vec<Property>)>)> {
    use schema::documents::dsl::*;

    let docs: Vec<Document> = documents
        .filter(id.eq(doc_id))
        .load::<Document>(db_conn)
        .unwrap();

    let doc_resources: Vec<Resource> = Resource::belonging_to(&docs)
        .load::<Resource>(db_conn)
        .unwrap();

    let doc_properties: Vec<Property> =
        match Property::belonging_to(&doc_resources).load::<Property>(db_conn) {
            Ok(res) => res,
            Err(e) => {
                println!("{:?}", e);
                vec![]
            }
        };

    let grouped_properties: Vec<Vec<Property>> = doc_properties.grouped_by(&doc_resources);

    let resources_and_properties = doc_resources
        .into_iter()
        .zip(grouped_properties)
        .grouped_by(&docs);

    docs.into_iter().zip(resources_and_properties).collect()
}
