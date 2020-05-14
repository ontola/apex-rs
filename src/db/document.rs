use crate::db::db_context::DbContext;
use crate::db::models::*;
use crate::db::schema;
use crate::hashtuple::{HashModel, LookupTable, Statement};
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

pub fn doc_by_iri<'a>(
    ctx: &DbContext,
    lookup_table: &'a mut LookupTable,
    iri: &str,
) -> Option<(i64, HashModel)> {
    let docs = get_document(&ctx.get_conn(), iri);
    let first = docs.first();

    if docs.is_empty() {
        debug!(target: "apex", "Doc with iri '{}' is empty", iri);
        return None;
    }

    let mut props: HashModel = vec![];
    let (doc, resources) = first.unwrap();
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
            props.push(Statement::new(
                lookup_table.ensure_value(&resource.iri),
                lookup_table.ensure_value(predicate),
                lookup_table.ensure_value(&p.value),
                lookup_table.ensure_value(datatype),
                lookup_table.ensure_value(&String::from(EMPTY_STRING)), // p.language.clone()
                lookup_table.ensure_value(&String::from(EMPTY_STRING)),
            ));
        }
    }

    Some((doc.id, props))
}

const RANDOM_DOC_ID: &str = "SELECT *
FROM  (
    SELECT DISTINCT 1 + trunc(random() * 5100000)::integer AS id
    FROM   generate_series(1, 1100) g
    ) r
JOIN documents USING (id)
LIMIT  1;";

pub fn random_doc<'a>(
    ctx: &DbContext,
    lookup_table: &'a mut LookupTable,
) -> Option<(i64, HashModel)> {
    let random_iri = diesel::sql_query(RANDOM_DOC_ID)
        .get_result::<Document>(&ctx.get_conn())
        .unwrap()
        .iri;

    doc_by_iri(ctx, lookup_table, &random_iri)
}

const EMPTY_STRING: &str = "";

pub(crate) fn reset_document<'a>(
    ctx: &DbContext,
    lookup_table: &'a mut LookupTable,
    iri: &str,
) -> (i64, HashModel) {
    match doc_by_iri(&ctx, lookup_table, iri) {
        None => {
            let doc = &NewDocument {
                iri: String::from(iri),
            };
            let doc = diesel::insert_into(schema::documents::table)
                .values(doc)
                .get_result::<Document>(&ctx.get_conn())
                .expect("Error while inserting into documents");

            (doc.id, vec![])
        }
        Some(model) => {
            delete_document_data(&ctx.get_conn(), iri);
            model
        }
    }
}

fn delete_document_data(db_conn: &PgConnection, doc_iri: &str) -> i64 {
    use schema::documents;
    use schema::properties;
    use schema::resources::dsl::*;

    let doc_id = documents::table
        .filter(documents::iri.eq(doc_iri))
        .select(documents::id)
        .get_result::<i64>(db_conn)
        .unwrap_or_else(|_| panic!("Document with iri '{}' does not exist", doc_iri));

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

    doc_id
}

fn get_document(
    db_conn: &PgConnection,
    doc_iri: &str,
) -> Vec<(Document, Vec<(Resource, Vec<Property>)>)> {
    use schema::documents::dsl::*;

    let docs: Vec<Document> = documents
        .filter(iri.eq(doc_iri))
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
