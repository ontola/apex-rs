use crate::serving::response_type::ResponseType;
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::header;

pub(crate) fn set_default_headers<'a>(
    res: &'a mut HttpResponseBuilder,
    response_type: &'a ResponseType,
) -> &'a mut HttpResponseBuilder {
    set_default_headers_str(res, response_type.to_string().as_ref())
}

pub(crate) fn set_default_headers_str<'a>(
    res: &'a mut HttpResponseBuilder,
    response_type: &str,
) -> &'a mut HttpResponseBuilder {
    res.set_header(header::SERVER, "Apex/1")
        .set_header(header::CONTENT_TYPE, response_type)
        .set_header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .set_header(header::ACCESS_CONTROL_ALLOW_METHODS, "POST, GET, OPTIONS")
        .set_header(header::ACCESS_CONTROL_MAX_AGE, 86400u32.to_string())
        .set_header(
            header::VARY,
            "Accept, Accept-Encoding, Authorization, Origin",
        )
}
