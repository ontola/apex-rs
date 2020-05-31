use actix_web::{get, HttpResponse};

#[get("/")]
pub(crate) async fn welcome() -> HttpResponse {
    let html = r#"
    <p>You're running Apex RS!</p>
    <p>Check out <a href="/random">/random</a></p>
    <p>And for Triple Pattern Fragment queries, check out <a href="/tpf">/tpf</a></p>
    "#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(String::from(html))
}
