use actix_web::client::ClientRequest;

pub trait HeaderCopy {
    fn copy_header_from(
        self,
        header: &str,
        from: &actix_web::HttpRequest,
        default: Option<&str>,
    ) -> Self;
}

impl HeaderCopy for ClientRequest {
    fn copy_header_from(
        self,
        header: &str,
        from: &actix_web::HttpRequest,
        default: Option<&str>,
    ) -> Self {
        let value = from.headers().get(header);

        match value {
            Some(value) => match value.to_str() {
                Ok(value) => self.header(header, value),
                Err(_) => self,
            },
            None => match default {
                Some(value) => self.header(header, value),
                None => self,
            },
        }
    }
}
