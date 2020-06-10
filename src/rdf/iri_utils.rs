pub fn stem_iri<T>(iri: T) -> String
where
    T: Into<String>,
{
    let iri = iri.into();
    if iri.starts_with("_:") {
        return iri;
    }

    let mut i = url::Url::parse(&iri).unwrap();
    if i.fragment().is_some() {
        i.set_fragment(None);
        i.to_string()
    } else {
        i.to_string()
    }
}
