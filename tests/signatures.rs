use htmlp::*;
struct Scalars;
impl TokenCounter for Scalars {
    fn name(&self) -> &str {
        "scalar"
    }
    fn count(&self, text: &str) -> u64 {
        text.chars().count() as u64
    }
}
const SOURCE: &str = "<htmlp tokenizer='scalar' max-tokens='1k' per-item='100' reason='Shared &amp; brief.'><section id='s' max-tokens='10' reason='Short.'>Hi</section>{{name}}</htmlp>";
fn checked(source: &str) -> Report {
    lint(&parse(source).unwrap(), &Scalars)
}

#[test]
fn golden_vector_and_normalized_attribute_values() {
    // Independently computed using Python hashlib + struct.pack('>Q', ...).
    let signed = sign_source(SOURCE).unwrap();
    let doc = parse(&signed).unwrap();
    assert_eq!(doc.limits.sig.as_deref(), Some("29580b676d56bf50"));
    let equivalent = signed
        .replace("max-tokens='1k'", "max-tokens=\"1000\"")
        .replace("&amp;", "&#38;");
    assert!(checked(&equivalent).is_ok());
    assert_eq!(sign_source(&signed).unwrap(), signed);
}

#[test]
fn budgets_and_reasons_require_explicit_resigning() {
    assert_eq!(checked(SOURCE).diagnostics.len(), 2);
    let signed = sign_source(SOURCE).unwrap();
    assert!(checked(&signed).is_ok());
    for edited in [
        signed.replace("max-tokens='1k'", "max-tokens='2k'"),
        signed.replace("per-item='100'", "per-item='200'"),
        signed.replace("Shared &amp; brief.", "New rationale."),
        signed.replace("max-tokens='10'", "max-tokens='20'"),
        signed.replace(" sig=\"29580b676d56bf50\"", ""),
        signed.replace("29580b676d56bf50", "wrong"),
    ] {
        let report = checked(&edited);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "signature" && d.message.contains("htmlp sign"))
        );
        assert!(checked(&sign_source(&edited).unwrap()).is_ok());
        assert!(
            render(
                &parse(&edited).unwrap(),
                &Bindings::from([("name".into(), "x".into())]),
                &Scalars
            )
            .is_err()
        );
    }
}

#[test]
fn content_edits_do_not_need_signing_but_still_obey_budgets() {
    let signed = sign_source(SOURCE).unwrap();
    assert!(checked(&signed.replace("Hi", "Hello")).is_ok());
    let report = checked(&signed.replace("Hi", "This is too long"));
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code, "budget");
}

#[test]
fn signing_preserves_source_including_multiline_tags_and_self_closing_sections() {
    let source = "<!-- keep -->\r\n<htmlp\r\n reason='A &amp; B.'\r\n max-tokens=\"1k\" sig='old'>\r\n# Hello 😀\r\n<section per-item='2' reason='Small.' />{{name}}\r\n<section sig='orphan' id='empty' />\r\n</htmlp>\r\n";
    let signed = sign_source(source).unwrap();
    let root_sig = parse(&signed).unwrap().limits.sig.unwrap();
    let section_sig = parse(&signed).unwrap().sections()[0]
        .limits
        .sig
        .clone()
        .unwrap();
    let expected = source
        .replace("sig='old'", &format!("sig='{root_sig}'"))
        .replace(
            "reason='Small.' />",
            &format!("reason='Small.' sig=\"{section_sig}\" />"),
        )
        .replace("sig='orphan'", "");
    assert_eq!(signed, expected);
    assert_eq!(sign_source(&signed).unwrap(), signed);
    assert!(lint(&parse(&signed).unwrap(), &Cl100kLike).is_ok());
    assert!(sign_source("<htmlp max-tokens='1k' />").is_err());
}
struct Cl100kLike;
impl TokenCounter for Cl100kLike {
    fn name(&self) -> &str {
        "cl100k_base"
    }
    fn count(&self, _: &str) -> u64 {
        0
    }
}

#[test]
fn in_memory_signing_and_orphan_signatures() {
    let mut section = Section::new("s", vec![]);
    section.limits.per_item = Some(10);
    section.limits.reason = Some("Brief.".into());
    let mut doc = Document::new(100, "Shared.", vec![Node::Section(section)]);
    doc.sign();
    assert!(lint(&doc, &Cl100kLike).is_ok());
    if let Node::Section(section) = &mut doc.children[0] {
        section.limits.per_item = None;
    }
    assert_eq!(lint(&doc, &Cl100kLike).diagnostics[0].code, "signature");
    doc.sign();
    assert!(doc.sections()[0].limits.sig.is_none());
    assert!(lint(&doc, &Cl100kLike).is_ok());
}
