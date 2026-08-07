/// Granit-parser against YAML test suite integration test
use granit_parser::Parser;
use std::path::Path;

struct EventSink {
    _count: u64,
}

impl<'a> granit_parser::EventReceiver<'a> for EventSink {
    fn on_event(&mut self, _ev: granit_parser::Event<'a>) {
        self._count += 1;
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

    let mut entries: Vec<_> = std::fs::read_dir(suite_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let content = std::fs::read_to_string(&path).unwrap();

        let mut sink = EventSink { _count: 0 };
        let mut parser = Parser::new_from_str(&content);
        total += 1;

        match parser.load(&mut sink, true) {
            Ok(_) => success += 1,
            Err(e) => {
                if errors.len() < 10 {
                    errors.push(format!(
                        "{}: {}",
                        path.file_stem().unwrap().to_str().unwrap(),
                        e
                    ));
                }
            }
        }
    }

    eprintln!("granit-parser YAML test suite results:");
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

    assert!(success > 0, "granit-parser should parse some YAML files");
}
