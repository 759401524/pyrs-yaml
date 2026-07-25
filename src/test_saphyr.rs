/// Test saphyr-parser integration
#[cfg(test)]
mod saphyr_tests {
    use saphyr_parser::{Event, EventReceiver, Parser};

    struct EventSink<'a> {
        events: Vec<Event<'a>>,
    }

    impl<'a> EventReceiver<'a> for EventSink<'a> {
        fn on_event(&mut self, ev: Event<'a>) {
            self.events.push(ev);
        }
    }

    #[test]
    fn test_basic_parse() {
        let yaml = "name: John\nage: 30";
        let mut sink = EventSink { events: Vec::new() };
        let mut parser = Parser::new_from_str(yaml);
        parser.load(&mut sink, true).unwrap();

        println!("Events:");
        for event in &sink.events {
            println!("  {:?}", event);
        }

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
        let mut parser = Parser::new_from_str(yaml);
        parser.load(&mut sink, true).unwrap();

        println!("Complex YAML events:");
        for event in &sink.events {
            println!("  {:?}", event);
        }

        assert!(!sink.events.is_empty());
    }
}
