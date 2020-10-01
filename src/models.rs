use crate::db::cache_control::CacheControl;
use crate::hashtuple::HashModel;

#[derive(Clone)]
pub(crate) struct Document {
    pub iri: String,
    pub status: i16,
    pub cache_control: CacheControl,
    pub language: Option<String>,
    pub data: HashModel,
}
