/// Test saphyr-parser against YAML test suite
#[cfg(test)]
mod yaml_suite_saphyr_tests {
    use saphyr_parser::{Parser, EventReceiver, Event};
    use std::fs;
    use std::path::Path;

    struct EventSink<'a> {
        events: Vec<Event<'a>>,
    }

    impl<'a> EventReceiver<'a> for EventSink<'a> {
        fn on_event(&mut self, ev: Event<'a>) {
            self.events.push(ev);
        }
    }

    #[test]
    fn test_yaml_test_suite() {
        let suite_dir = Path::new("Reference/yaml-test-suite/src");

        if !suite_dir.exists() {
            eprintln!("YAML test suite not found at {:?}, skipping", suite_dir);
            return;
        }

        let mut total = 0;
        let mut success = 0;
        let mut errors = Vec::new();

        // Read all yaml files in the directory
        let mut entries: Vec<_> = fs::read_dir(suite_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml"))
            .collect();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let path = entry.path();
            let content = fs::read_to_string(&path).unwrap();

            let mut sink = EventSink { events: Vec::new() };
            let mut parser = Parser::new_from_str(&content);
            total += 1;

            match parser.load(&mut sink, true) {
                Ok(_) => success += 1,
                Err(e) => {
                    if errors.len() < 10 {
                        errors.push(format!("{}: {}", path.file_stem().unwrap().to_str().unwrap(), e));
                    }
                }
            }
        }

        eprintln!("saphyr-parser YAML test suite results:");
        eprintln!("  Total: {}", total);
        eprintln!("  Success: {}", success);
        eprintln!("  Failed: {}", total - success);
        eprintln!("  Rate: {:.1}%", success as f64 / total as f64 * 100.0);

        if !errors.is_empty() {
            eprintln!("\nSample errors:");
            for e in &errors {
                eprintln!("  {}", e);
            }
        }

        assert!(success > 0, "saphyr-parser should parse some YAML files");
    }
}
