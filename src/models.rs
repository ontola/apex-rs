use crate::schema::documents;
use crate::schema::properties;
use crate::schema::resources;

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable, Insertable)]
pub struct Document {
    pub id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable, Insertable)]
#[belongs_to(Document)]
pub struct Resource {
    pub id: i32,
    pub document_id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable, Insertable)]
#[table_name = "properties"]
#[belongs_to(Resource)]
pub struct Property {
    pub id: i32,
    pub resource_id: i32,
    pub predicate: String,
    pub order: i32,
    pub value: String,
    pub datatype: String,
    pub language: String,
    pub prop_resource: i32,
}
