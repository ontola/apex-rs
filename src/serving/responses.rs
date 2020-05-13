use crate::serving::response_type::ResponseType;
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::header;

pub(crate) fn set_default_headers<'a>(
    res: &'a mut HttpResponseBuilder,
    response_type: &'a ResponseType,
) -> &'a mut HttpResponseBuilder {
    res.set_header(header::CONTENT_TYPE, response_type.to_string())
        .set_header(header::VARY, "Accept, Accept-Encoding, Origin")
}
