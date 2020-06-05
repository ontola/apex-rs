pub struct Tuple {
    subject: String,
    predicate: String,
    value: String,
    datatype: String,
    language: String,
    graph: String,
}

impl Tuple {
    pub fn new(
        subject: &str,
        predicate: &str,
        value: &str,
        datatype: &str,
        language: &str,
        graph: &str,
    ) -> Tuple {
        Tuple {
          subject: String::from(subject),
          predicate: String::from(predicate),
          value: String::from(value),
          datatype: String::from(datatype),
          language: String::from(language),
          graph: String::from(graph),
        }
    }
}

/// Converts a tuple into an NDJSON HexTuple with escaped quotes, ready for redis.
pub fn serialize_hextuple_redis(tuple: Tuple) -> String {
    let message = format!("[\"{}\", \"{}\", \"{}\", \"{}\", \"{}\", \"{}\"]\n",
        tuple.subject,
        tuple.predicate,
        tuple.value,
        tuple.datatype,
        tuple.language,
        tuple.graph
    );
    return message;
}
