use crate::schema::*;

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable, Insertable)]
pub struct Document {
    pub id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
#[belongs_to(Document)]
pub struct Resource {
    pub id: i32,
    pub document_id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Associations, Insertable)]
#[table_name = "resources"]
#[belongs_to(Document)]
pub struct NewResource {
    pub document_id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
pub struct Predicate {
    pub id: i32,
    pub value: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
pub struct Datatype {
    pub id: i32,
    pub value: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
pub struct Language {
    pub id: i32,
    pub value: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
#[table_name = "properties"]
#[primary_key(resource_id, predicate_id, order)]
#[belongs_to(Resource)]
pub struct Property {
    //    pub id: uuid::Uuid,
    pub resource_id: i32,
    pub predicate_id: i32,
    pub order: Option<i32>,
    pub value: String,
    pub datatype_id: i32,
    pub language_id: Option<i32>,
    pub prop_resource: Option<i32>,
}

#[derive(Eq, PartialEq, Debug, Associations, Insertable)]
#[table_name = "properties"]
pub struct NewProperty {
    pub resource_id: i32,
    pub predicate_id: i32,
    pub order: Option<i32>,
    pub value: String,
    pub datatype_id: i32,
    pub language_id: Option<i32>,
    pub prop_resource: Option<i32>,
}
