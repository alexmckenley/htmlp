use htmlp::*;
struct Count;
impl TokenCounter for Count {
    fn name(&self) -> &str {
        "scalar"
    }
    fn count(&self, s: &str) -> u64 {
        s.chars().count() as u64
    }
}
fn doc(source: &str) -> Document {
    let mut doc = parse(source).unwrap();
    doc.sign();
    doc
}
#[test]
fn rendered_spans_partition_unicode_text_and_keep_section_ancestry() {
    let document = doc(
        "<htmlp tokenizer='scalar' max-tokens='100' reason='Shared.'>😀<section id='outer'>A<section id='inner'>{{v}}</section>Z</section><section id='empty' /></htmlp>",
    );
    let output = render_checked(
        &document,
        &Bindings::from([("v".into(), "é{{literal}}".into())]),
        &Count,
    )
    .unwrap();
    assert_eq!(output.text(), "😀Aé{{literal}}Z");
    assert_eq!(output.section("inner"), Some("é{{literal}}"));
    assert_eq!(output.section("outer"), Some("Aé{{literal}}Z"));
    assert_eq!(output.section("empty"), Some(""));
    assert_eq!(output.section("absent"), None);
    assert_eq!(
        output.measurement().tokens,
        output.text().chars().count() as u64
    );
    assert_eq!(output.measurement().tokenizer, "scalar");
    let mut end = 0;
    for span in output.spans() {
        assert_eq!(span.range.start, end);
        assert!(output.text().get(span.range.clone()).is_some());
        end = span.range.end;
    }
    assert_eq!(end, output.text().len());
    let variable = output
        .spans()
        .iter()
        .find(|s| s.variable.is_some())
        .unwrap();
    assert_eq!(
        variable
            .sections
            .iter()
            .map(|s| s.id.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("outer"), Some("inner")]
    );
    assert!(output.report().measurements.iter().all(|m| !m.deferred));
}
#[test]
fn checked_selection_cannot_bypass_parent_or_sibling_limits() {
    let document = doc(
        "<htmlp tokenizer='scalar' max-tokens='3' reason='Small.'><section id='ok'>a</section><section>{{v}}</section></htmlp>",
    );
    assert!(
        render_checked(
            &document,
            &Bindings::from([("v".into(), "long".into())]),
            &Count
        )
        .is_err()
    );
    assert!(render_checked(&document, &Bindings::new(), &Count).is_err());
    let mut changed = document.clone();
    changed.limits.max_tokens = Some(100);
    assert!(
        render_checked(
            &changed,
            &Bindings::from([("v".into(), "x".into())]),
            &Count
        )
        .is_err()
    );
}
#[test]
fn whole_measurement_does_not_sum_overlapping_sections_or_token_boundaries() {
    struct Boundary;
    impl TokenCounter for Boundary {
        fn name(&self) -> &str {
            "scalar"
        }
        fn count(&self, s: &str) -> u64 {
            if s == "ab" { 3 } else { s.len() as u64 }
        }
    }
    let document = doc(
        "<htmlp tokenizer='scalar' max-tokens='4' reason='Small.'><section>a</section><section>b</section></htmlp>",
    );
    let output = render_checked(&document, &Bindings::new(), &Boundary).unwrap();
    assert_eq!(output.measurement().tokens, 3);
    assert_eq!(
        output
            .report()
            .measurements
            .iter()
            .filter_map(|m| m.tokens)
            .sum::<u64>(),
        5
    );
}
