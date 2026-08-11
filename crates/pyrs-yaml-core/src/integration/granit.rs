//! Granit-parser integration tests

struct EventSink<'a> {
    events: Vec<granit_parser::Event<'a>>,
}

impl<'a> granit_parser::EventReceiver<'a> for EventSink<'a> {
    fn on_event(&mut self, ev: granit_parser::Event<'a>) {
        self.events.push(ev);
    }
}

#[test]
fn test_basic_parse() {
    let yaml = "name: John\nage: 30";
    let mut sink = EventSink { events: Vec::new() };
    let mut parser = granit_parser::Parser::new_from_str(yaml);
    parser.load(&mut sink, true).unwrap();
    assert!(!sink.events.is_empty());
}

#[test]
fn test_complex_yaml() {
    let yaml = r#"
# Comment
defaults: &defaults
  timeout: 30

production:
  <<: *defaults
  host: prod.com
"#;
    let mut sink = EventSink { events: Vec::new() };
    let mut parser = granit_parser::Parser::new_from_str(yaml);
    parser.load(&mut sink, true).unwrap();
    assert!(!sink.events.is_empty());
}
