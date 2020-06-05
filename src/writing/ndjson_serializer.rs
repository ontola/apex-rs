
pub(crate) struct Tuple {
    resource: Vec<String>,
}

pub fn serialize<'a>(tuples: Tuple) -> &'a &str {
  let message = "[\"http://localhost:8080/test\", \"http://schema.org/birthDate\", \"2000-06-08\", \"http://www.w3.org/2001/XMLSchema#date\", \"\", \"http://purl.org/linked-delta/replace\"]\n";
}
