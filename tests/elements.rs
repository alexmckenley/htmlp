use htmlp::*;

struct Count;
impl TokenCounter for Count {
    fn name(&self) -> &str {
        "cl100k_base"
    }
    fn count(&self, text: &str) -> u64 {
        text.chars().count() as u64
    }
}
fn normalized(mut doc: Document) -> Document {
    fn nodes(children: &mut [Node]) {
        for node in children {
            match node {
                Node::Element(e) => {
                    e.position = Position::default();
                    nodes(&mut e.children);
                }
                Node::Variable(v) => v.position = Position::default(),
                Node::Text { .. } => {}
            }
        }
    }
    doc.position = Position::default();
    nodes(&mut doc.children);
    doc.sign();
    doc
}
#[test]
fn markup_templates_and_explicit_nodes_share_one_model() {
    let markup = normalized(parse("<htmlp max-tokens='100' reason='Shared.'><greeting id='welcome' max-tokens='50' reason='Brief.'>Hello, {{name}} &amp; {{name}}!</greeting></htmlp>").unwrap());
    let template = normalized(Document::new(
        100,
        "Shared.",
        vec![
            Element::new("greeting")
                .id("welcome")
                .max_tokens(50, "Brief.")
                .template("Hello, {{name}} & {{name}}!")
                .unwrap()
                .into(),
        ],
    ));
    let explicit = normalized(Document::new(
        100,
        "Shared.",
        vec![
            Element::new("greeting")
                .id("welcome")
                .max_tokens(50, "Brief.")
                .text("Hello, ")
                .variable("name")
                .text(" & ")
                .variable("name")
                .text("!")
                .into(),
        ],
    ));
    assert_eq!(markup, template);
    assert_eq!(template, explicit);
    let bindings = Bindings::from([("name".into(), "<x>{{other}}".into())]);
    for doc in [markup, template, explicit] {
        assert!(lint(&doc, &Count).measurements.iter().all(|m| m.deferred));
        let output = doc.render(&bindings, &Count).unwrap();
        assert_eq!(output.text(), "Hello, <x>{{other}} & <x>{{other}}!");
        assert_eq!(output.element("welcome"), Some(output.text()));
        assert_eq!(output.elements_by_name("greeting"), [output.text()]);
        assert!(doc.render(&Bindings::new(), &Count).is_err());
        assert!(
            doc.render(&Bindings::from([("name".into(), "x".repeat(51))]), &Count)
                .is_err()
        );
    }
}
#[test]
fn anonymous_custom_elements_are_selectable_and_have_unambiguous_provenance() {
    let mut doc = Document::new(
        100,
        "Shared.",
        vec![
            Element::new("skills")
                .per_item(5, "Short items.")
                .element(Element::new("skill").text("one"))
                .element(Element::new("skill"))
                .element(Element::new("skill").text("two"))
                .into(),
        ],
    );
    doc.sign();
    assert_eq!(doc.elements_by_name("skill").len(), 3);
    let output = doc.render(&Bindings::new(), &Count).unwrap();
    assert_eq!(output.elements_by_name("skill"), ["one", "", "two"]);
    assert_eq!(output.elements().len(), 4);
    assert_eq!(output.elements()[2].origin.path, [0, 1]);
    assert_eq!(output.elements()[2].range, 3..3);
    assert_eq!(output.spans()[1].elements[1].path, [0, 2]);
    let children: Vec<_> = output
        .report()
        .measurements
        .iter()
        .filter(|m| m.name == "skill")
        .collect();
    assert_eq!(children.len(), 3);
    assert!(
        children
            .iter()
            .all(|m| m.limit == Some(5) && m.id.is_none())
    );
    assert_eq!(children[2].path, [0, 2]);
    assert_eq!(output.measurement().tokens, 6);
}
#[test]
fn names_are_generic_but_syntax_and_attributes_remain_strict() {
    let source = "<htmlp max-tokens='100' reason='Shared.'><system-prompt /><tool-definition id='shell' /><var /></htmlp>";
    let signed = sign_source(source).unwrap();
    let doc = parse(&signed).unwrap();
    assert!(lint(&doc, &Count).is_ok());
    assert_eq!(
        doc.elements()
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["system-prompt", "tool-definition", "var"]
    );
    for name in [
        "",
        "htmlp",
        "System",
        "two words",
        "ns:tag",
        "-tag",
        "1tag",
        "tag_name",
    ] {
        let mut doc = Document::new(100, "Shared.", vec![Element::new(name).into()]);
        doc.sign();
        assert!(
            lint(&doc, &Count)
                .diagnostics
                .iter()
                .any(|d| d.code == "element"),
            "{name}"
        );
    }
    assert!(
        parse("<htmlp max-tokens='100' reason='Shared.'><skill category='system' /></htmlp>")
            .is_err()
    );
    assert!(
        parse(
            "<htmlp max-tokens='100' reason='Shared.'><skill id='same' /><rule id='same' /></htmlp>"
        )
        .is_err()
    );
    assert!(parse("<htmlp max-tokens='100' reason='Shared.'><skill></rule></htmlp>").is_err());
}
#[test]
fn text_is_literal_and_template_errors_have_positions() {
    let mut doc = Document::new(
        100,
        "Shared.",
        vec![
            Element::new("literal").text("{{name}} &amp; <tag>").into(),
            Element::new("template")
                .template(" &amp; <tag> {{name}}")
                .unwrap()
                .into(),
        ],
    );
    doc.sign();
    let output = doc
        .render(&Bindings::from([("name".into(), "ok".into())]), &Count)
        .unwrap();
    assert_eq!(output.text(), "{{name}} &amp; <tag> &amp; <tag> ok");
    for invalid in ["{{}}", "{{ name }}", "{{name", "{{name max-tokens=2}}"] {
        assert!(Element::new("greeting").template(invalid).is_err());
    }
    let error = parse_template("é\n{{bad name}}").unwrap_err();
    assert_eq!(
        (
            error.position.line,
            error.position.column,
            error.position.offset
        ),
        (2, 1, 3)
    );
}
