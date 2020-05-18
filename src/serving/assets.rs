use crate::serving::responses::set_default_headers_str;
use actix_web::http::header;
use actix_web::{get, HttpResponse, Responder};
use std::time::{Duration, SystemTime};

static FAVICON_BASE64: &str = "AAABAAEAIBsAAAEAIAAUDgAAFgAAACgAAAAgAAAANgAAAAEAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABQAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAACAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAA/wAAAP8AAAD/AAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwAAAP8AAAD/AAAA/wAAAP8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEgAAAP8AAAD/AAAAAAAAAAIAAAD/AAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAD/AAAAAQAAAAAAAAD/AAAA/wAAAA4AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAAAAAAAP8AAAAAAAAAAAAAAAAAAAAJAAAA/wAAAP8AAAAAAAAAAAAAAP8AAAD/AAAABgAAAAAAAAAAAAAAAAAAAP8AAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAkAAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEgAAAP8AAAD/AAAADwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAAOAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAD/AAAAAAAAAAAAAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAcAAAD/AAAA/wAAAP8AAAD/AAAABQAAAAAAAAAAAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAD/AAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAEAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAIAAAD/AAAA/wAAAAAAAAAAAAAAAAAAAAAAAAD/AAAA/wAAAAEAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAMAAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAAD/AAAAAAAAAAAAAAAAAAAA/wAAAAAAAAD/AAAA/wAAAAMAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFAAAA/wAAAP8AAAAAAAAA/wAAAAAAAAAAAAAAAAAAAP8AAAALAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAD/AAAA/wAAAAoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAAAAP8AAAD/AAAAAAAAAAAAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAALAAAAAAAAAAAAAAAMAAAA/wAAAP8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAD/AAAACgAAAAAAAAAAAAAAEAAAAP8AAAAAAAAAAAAAAAAAAAADAAAA/wAAAAAAAAAEAAAA/wAAAP8AAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAD/AAAA/wAAAAMAAAAAAAAA/wAAAAEAAAAAAAAAAAAAAP8AAAACAAAA/wAAAP8AAAABAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAABAAAA/wAAAP8AAAAEAAAA/wAAAAAAAAAOAAAA/wAAAP8AAAAFAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAAAAAAAAAAABwAAAP8AAAD/AAAACQAAAP8AAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAD/AAAADQAAAP8AAAD/AAAABgAAAAAAAAAAAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAkAAAD/AAAA/wAAAAgAAAAAAAAA/wAAAAIAAAD/AAAA/wAAAAEAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAIAAAD/AAAA/wAAAAUAAAD/AAAAAAAAAAAAAAACAAAA/wAAAAAAAAADAAAA/wAAAP8AAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAD/AAAA/wAAAAIAAAAAAAAA/wAAAAEAAAAAAAAAAAAAAAAAAAD/AAAADAAAAAAAAAAAAAAACgAAAP8AAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAA/wAAAAgAAAAAAAAAAAAAABEAAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAAAAAAAAAAAAP8AAAD/AAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAA/wAAAP8AAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA8AAAD/AAAAAAAAAAAAAAAAAAAA/wAAAAAAAAD/AAAA/wAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFAAAA/wAAAP8AAAAAAAAA/wAAAAAAAAAAAAAAAAAAAP8AAAALAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAABAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAABAAAA/wAAAP8AAAAAAAAAAAAAAAAAAAAAAAAA/wAAAP8AAAAAAAAAAAAAAAAAAAD/AAAAAAAAAAAAAAADAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwAAAP8AAAAAAAAAAAAAAP8AAAAAAAAAAAAAAAAAAAAAAAAABQAAAP8AAAD/AAAA/wAAAP8AAAADAAAAAAAAAAAAAAAAAAAAAAAAAP8AAAAAAAAAAAAAAP8AAAABAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAoAAAAAAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEwAAAP8AAAD/AAAAEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAAPAAAA/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAAAAAAD/AAAAAAAAAAAAAAAAAAAACgAAAP8AAAD/AAAAAAAAAAAAAAD/AAAA/wAAAAgAAAAAAAAAAAAAAAAAAAD/AAAAAAAAAP8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAARAAAA/wAAAP8AAAAAAAAAAwAAAP8AAAD/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/wAAAP8AAAACAAAAAAAAAP8AAAD/AAAADQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAAA/wAAAP8AAAD/AAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgAAAP8AAAD/AAAA/wAAAP8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAA/wAAAP8AAAD/AAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD/AAD//h/4f/5n5n/9eZ6/+35+3/t8Pt/3c87v90/y7+8//Pfef/572X/+m6d//uWff/75P3/+/J9//vmnf/7l2X/+m95//nvvH/j390/y7/dzzu/7fD7f+35+3/15nr/+Z+Z//h/4f/8AAP8=";
static ICO_TYPE: &str = "image/x-icon";

#[get("/favicon.ico")]
pub(crate) async fn favicon() -> impl Responder {
    let mut res = HttpResponse::Ok();
    let expiration = SystemTime::now() + Duration::from_secs(60 * 60 * 24);

    set_default_headers_str(&mut res, ICO_TYPE)
        .set(header::CacheControl(vec![
            header::CacheDirective::MaxAge(86400u32),
            header::CacheDirective::Public,
        ]))
        .set(header::Expires(expiration.into()))
        .body(base64::decode(FAVICON_BASE64).expect("Wrong favicon encoding"))
}
