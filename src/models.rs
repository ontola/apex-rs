use crate::db::cache_control::CacheControl;
use crate::importing::parsing::DocumentSet;

#[derive(Clone)]
pub(crate) struct Document {
    pub iri: String,
    pub status: i16,
    pub cache_control: CacheControl,
    pub data: DocumentSet,
}
