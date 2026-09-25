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
fn document(body: &str) -> Document {
    parse(&format!(
        r#"<htmlp tokenizer="scalar" max-tokens="1k" reason="Shared context.">{body}</htmlp>"#
    ))
    .unwrap()
}
#[test]
fn budgets_are_integer_tokens() {
    for (s, n) in [
        ("0", 0),
        ("100", 100),
        ("1k", 1000),
        ("1.5k", 1500),
        ("0.001K", 1),
        ("18446744073709551615", u64::MAX),
    ] {
        assert_eq!(parse_limit(s).unwrap(), n);
    }
    for s in [
        "",
        "-1",
        "+1",
        "1.5",
        "1.0001k",
        ".5k",
        "1.k",
        "1e3",
        " 1k",
        "1k ",
        "18446744073709551616",
        "18446744073709552k",
    ] {
        assert!(parse_limit(s).is_err(), "{s}");
    }
}
#[test]
fn preserves_markdown_unicode_and_whitespace() {
    let d = document(
        "\r\n<section id=\"task\"># Task\r\né &amp; &lt; &#x1F600;<!-- hidden --></section>\n",
    );
    assert_eq!(d.to_string(), "\n# Task\né & < 😀\n");
    assert_eq!(
        d.get_element_by_id("task").unwrap().to_string(),
        "# Task\né & < 😀"
    );
    assert_eq!(d.sections().len(), 1);
    assert_eq!(d.sections()[0].position.line, 2);
}
#[test]
fn strict_syntax_rejects_browser_repair_and_xml_extensions() {
    let bad = [
        "<htmlp max-tokens='1k' max-item-tokens='100' reason='Shared.'></htmlp>",
        "",
        "<htmlp></htmlp>",
        "<!DOCTYPE html><htmlp max-tokens='1k' reason='x'></htmlp>",
        "<?xml version='1.0'?><htmlp max-tokens='1k' reason='x'></htmlp>",
        "<htmlp max-tokens='1k' reason='x'><section></htmlp>",
        "<htmlp max-tokens='1k' reason='x'><SECTION></SECTION></htmlp>",
        "<htmlp max-tokens='1k' reason='x'><div>x</div></htmlp>",
        "<htmlp max-tokens='1k' reason='x' other='x'></htmlp>",
        "<htmlp max-tokens='1k' reason='x' reason='y'></htmlp>",
        "<htmlp max-tokens='1k' reason='x'><![CDATA[x]]></htmlp>",
        "<htmlp max-tokens='1k' reason='x'>&nbsp;</htmlp>",
        "<htmlp max-tokens='1k' reason='x'>&#128;</htmlp>",
        "<htmlp max-tokens='1k' reason='x'>a & b</htmlp>",
        "<htmlp max-tokens='1k' reason='x'><section id='x'></section><section id='x'></section></htmlp>",
        "<htmlp max-tokens='1k' reason='x'><var id='x'> </var></htmlp>",
        "<htmlp max-tokens='1k' reason='x'><htmlp></htmlp></htmlp>",
        "<htmlp max-tokens='1k' reason='x' version='99'></htmlp>",
        "<htmlp max-tokens='1k' reason='x'>\0</htmlp>",
    ];
    for source in bad {
        assert!(parse(source).is_err(), "accepted: {source}");
    }
}
#[test]
fn every_declared_limit_needs_its_own_reason() {
    for source in [
        "<htmlp max-tokens='1k'></htmlp>",
        "<htmlp max-tokens='1k' reason='  '></htmlp>",
        "<htmlp max-tokens='1k' reason='Root.'><section max-tokens='1'></section></htmlp>",
        "<htmlp max-tokens='1k' reason='Root.'><section per-item='1'></section></htmlp>",
    ] {
        assert_eq!(parse(source).unwrap_err().code, "reason");
    }
    let d = Document::new(100, "", vec![]);
    assert!(
        lint(&d, &Scalars)
            .diagnostics
            .iter()
            .any(|e| e.code == "reason")
    );
}
#[test]
fn independent_nested_limits_keep_the_correct_reasons() {
    let d = document(
        "<section id='list' per-item='10' reason='Parent cap.'><section id='child' per-item='2' reason='Child cap.'><section id='grandchild'>abc</section></section></section>",
    );
    let r = lint(&d, &Scalars);
    let grand = r
        .measurements
        .iter()
        .find(|m| m.id.as_deref() == Some("grandchild"))
        .unwrap();
    assert_eq!(
        (grand.limit, grand.reason.as_deref()),
        (Some(2), Some("Child cap."))
    );
    let child = r
        .measurements
        .iter()
        .find(|m| m.id.as_deref() == Some("child"))
        .unwrap();
    assert_eq!(
        (child.limit, child.reason.as_deref()),
        (Some(10), Some("Parent cap."))
    );
    assert_eq!(r.diagnostics.len(), 1);
    assert!(r.diagnostics[0].message.contains("Child cap."));
}
#[test]
fn item_limits_do_not_apply_to_variables_or_plain_text() {
    let d = document("<section per-item='0' reason='No child content.'>abc{{v}}</section>");
    assert!(lint(&d, &Scalars).is_ok());
}
#[test]
fn static_and_runtime_budgets_are_both_enforced() {
    let mut d = document("a{{v}}");
    d.limits.max_tokens = Some(4);
    assert!(lint(&d, &Scalars).is_ok());
    let mut bindings = Bindings::new();
    assert!(render(&d, &bindings, &Scalars).is_err());
    bindings.insert("v".into(), "hey".into());
    assert_eq!(render(&d, &bindings, &Scalars).unwrap(), "ahey");
    bindings.insert("v".into(), "long".into());
    assert!(render(&d, &bindings, &Scalars).is_err());
    d.limits.max_tokens = Some(3);
    bindings.insert("v".into(), "".into());
    assert_eq!(render(&d, &bindings, &Scalars).unwrap(), "a");
    assert_eq!(d.to_string(), "a{{v}}");
}
#[test]
fn substitution_is_literal_and_cannot_change_the_tree() {
    let d = document("{{v}}");
    let value = "</htmlp><section max-tokens='0'>";
    let bindings = Bindings::from([("v".into(), value.into())]);
    assert_eq!(render(&d, &bindings, &Scalars).unwrap(), value);
}
#[test]
fn final_counts_are_not_assumed_additive() {
    struct Boundary;
    impl TokenCounter for Boundary {
        fn name(&self) -> &str {
            "scalar"
        }
        fn count(&self, s: &str) -> u64 {
            if s == "ab" { 3 } else { s.len() as u64 }
        }
    }
    let mut d = document("a{{v}}");
    d.limits.max_tokens = Some(2);
    assert!(lint(&d, &Boundary).is_ok());
    assert!(render(&d, &Bindings::from([("v".into(), "b".into())]), &Boundary).is_err());
    let d = document("<section>a</section><section>b</section>");
    assert_eq!(
        lint(&d, &Boundary).measurements.last().unwrap().tokens,
        Some(3)
    );
}
#[test]
fn constructors_and_ids_are_typed_and_validated() {
    let s = Section::new("system", vec![Node::text("Review.")]);
    let mut d = Document::new(100, "Shared.", vec![Node::Section(s.clone())]);
    d.tokenizer = "scalar".into();
    assert!(matches!(
        d.get_element_by_id("system"),
        Some(ElementRef::Section(_))
    ));
    assert!(lint(&d, &Scalars).is_ok());
    d.children.push(Node::Section(s));
    assert!(
        lint(&d, &Scalars)
            .diagnostics
            .iter()
            .any(|e| e.code == "id")
    );
}
#[test]
fn depth_and_source_size_are_bounded() {
    let body = format!("{}x{}", "<section>".repeat(63), "</section>".repeat(63));
    assert!(lint(&document(&body), &Scalars).is_ok());
    let body = format!("<section>{body}</section>");
    assert!(
        parse(&format!(
            "<htmlp max-tokens='1k' reason='Shared.'>{body}</htmlp>"
        ))
        .is_err()
    );
    assert_eq!(
        parse(&"x".repeat(4 * 1024 * 1024 + 1)).unwrap_err().code,
        "source-size"
    );
}
#[test]
fn tokenizer_identity_is_checked() {
    let d = parse("<htmlp max-tokens='1k' reason='Shared.'></htmlp>").unwrap();
    assert!(
        lint(&d, &Scalars)
            .diagnostics
            .iter()
            .any(|e| e.code == "tokenizer")
    );
}
#[test]
fn deterministic_malformed_inputs_do_not_panic() {
    let alphabet = [
        '<', '>', '/', '\'', '"', '&', ';', '\n', 'é', '😀', 'a', '=', '\0',
    ];
    let mut seed = 7u64;
    for len in 0..256 {
        let source: String = (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                alphabet[(seed >> 32) as usize % alphabet.len()]
            })
            .collect();
        let _ = parse(&source);
    }
}
#[cfg(feature = "tokens")]
#[test]
fn cl100k_vectors_and_ordinary_special_strings() {
    let c = Cl100k::new().unwrap();
    assert_eq!(c.count("hello world"), 2);
    assert_eq!(c.count(""), 0);
    assert_eq!(c.count("antidisestablishmentarianism"), 6);
    assert!(c.count("<|endoftext|>") > 1);
}
#[cfg(feature = "json")]
#[test]
fn json_roundtrips_the_typed_ast() {
    let d = document("<section id='x'>Text</section>");
    let json = serde_json::to_string(&d).unwrap();
    let copy: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(d, copy);
    assert!(json.contains("\"kind\":\"section\""));
}

#[test]
fn numeric_references_require_unsigned_digits() {
    for entity in ["&#+65;", "&#x+41;", "&#;", "&#x;"] {
        assert!(
            parse(&format!(
                "<htmlp max-tokens='1k' reason='Shared.'>{entity}</htmlp>"
            ))
            .is_err()
        );
    }
}
#[test]
fn unicode_positions_use_scalars_and_byte_offsets() {
    let source = "<htmlp max-tokens='1k' reason='Shared.'>é😀<section id='s'></section></htmlp>";
    let section = parse(source).unwrap().sections()[0].position;
    let offset = source.find("<section").unwrap();
    assert_eq!(section.offset, offset);
    assert_eq!(section.column, source[..offset].chars().count() + 1);
}

#[test]
fn variables_only_accept_ids_and_static_counts_are_deferred() {
    for source in [
        "{{}}",
        "{{ v }}",
        "{{v",
        "{{v max-tokens=1k}}",
        "<var id='v' />",
    ] {
        assert!(
            parse(&format!(
                "<htmlp max-tokens='1k' reason='Shared.'>{source}</htmlp>"
            ))
            .is_err()
        );
    }
    let d = document("<section id='dynamic'>{{v}}</section><section id='static'>abc</section>");
    let report = lint(&d, &Scalars);
    assert!(report.is_ok());
    assert_eq!(report.measurements.iter().filter(|m| m.deferred).count(), 2);
    assert!(
        report
            .measurements
            .iter()
            .filter(|m| m.deferred)
            .all(|m| m.tokens.is_none())
    );
    assert_eq!(
        report
            .measurements
            .iter()
            .find(|m| m.id.as_deref() == Some("static"))
            .unwrap()
            .tokens,
        Some(3)
    );
}
#[test]
fn self_closing_elements_keep_tree_structure_and_validation() {
    let d = document("<section id='empty' />{{v}}<section id='after'>End.</section>");
    assert_eq!(d.to_string(), "{{v}}End.");
    assert_eq!(d.sections().len(), 2);
    assert!(parse("<htmlp max-tokens='1k' reason='Shared.' />").is_ok());
    assert!(parse("<htmlp max-tokens='1k' reason='Shared.'><var /></htmlp>").is_err());
    assert!(
        parse("<htmlp max-tokens='1k' reason='Shared.'>{{v}}<section id='v' /></htmlp>").is_err()
    );
}
#[test]
fn per_item_is_enforced_after_binding() {
    let d =
        document("<section per-item='2' reason='Brief items.'><section>{{v}}</section></section>");
    assert!(lint(&d, &Scalars).is_ok());
    assert!(render(&d, &Bindings::from([("v".into(), "abc".into())]), &Scalars).is_err());
}
#[cfg(feature = "json")]
#[test]
fn variable_json_rejects_limits() {
    let d = document("{{v}}");
    for key in ["max_tokens", "per_item", "limits", "reason"] {
        let mut value = serde_json::to_value(&d).unwrap();
        value["children"][0][key] = serde_json::json!(10);
        assert!(serde_json::from_value::<Document>(value).is_err(), "{key}");
    }
}

#[test]
fn repeated_variables_escape_literals_and_do_not_reparse_values() {
    let d = document("{{name}} / {{name}} / &#123;&#123;name}} / <!-- {{ignored}} -->");
    assert!(lint(&d, &Scalars).is_ok());
    let result = render(
        &d,
        &Bindings::from([("name".into(), "{{other}}".into())]),
        &Scalars,
    )
    .unwrap();
    assert_eq!(result, "{{other}} / {{other}} / {{name}} / ");
    assert!(
        parse("<htmlp max-tokens='1k' reason='Shared.'><section id='v'>{{v}}</section></htmlp>")
            .is_err()
    );
    assert!(
        parse("<htmlp max-tokens='1k' reason='Shared.'>{{v}}<section id='v' /></htmlp>").is_err()
    );
}
