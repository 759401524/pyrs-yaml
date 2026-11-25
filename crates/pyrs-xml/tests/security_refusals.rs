//! Security refusals: exact codes and positions, with no partial tree.
//!
//! DTD/entity fixtures are never handed to an oracle that would expand them.
//! The billion-laughs case additionally asserts that the scanner refuses at
//! the `<!` boundary — no token vector is built, so no node can be
//! materialised from the bomb.

use std::fs;
use std::path::{Path, PathBuf};

use pyrs_xml::scanner;
use pyrs_xml::{XmlError, XmlOptions, from_xml, from_xml_with_options};

fn fixture(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/security")
        .join(name);
    fs::read_to_string(path).expect("security fixture")
}

fn expect_code(name: &str, code: &str) -> XmlError {
    let source = fixture(name);
    let error = from_xml(&source).expect_err("a refusal fixture must be refused");
    assert_eq!(error.code(), code, "fixture {name}");
    error
}

#[test]
fn doctype_is_refused_at_the_markup_boundary() {
    let error = expect_code("dtd_doctype.xml", "E-XML-001");
    match error {
        XmlError::DtdDeclared { line, column } => assert_eq!((line, column), (1, 1)),
        other => panic!("expected DtdDeclared, got {other:?}"),
    }
}

#[test]
fn internal_and_external_entity_declarations_are_refused() {
    // Both start with a DOCTYPE, so the first `<!` is already the refusal:
    // no internal subset is read, no SYSTEM/PUBLIC id is interpreted.
    for name in ["internal_entity.xml", "external_entity.xml"] {
        let error = expect_code(name, "E-XML-001");
        assert!(error.is_parse_error());
    }
    let standalone = expect_code("standalone_entity.xml", "E-XML-002");
    match standalone {
        XmlError::EntityDeclaration { line, column } => assert_eq!((line, column), (1, 1)),
        other => panic!("expected EntityDeclaration, got {other:?}"),
    }
}

#[test]
fn billion_laughs_fails_before_any_node_is_materialised() {
    let source = fixture("billion_laughs.xml");
    // Ten nested declarations: a parse-then-filter design would expand these
    // into 10^10 copies before seeing the DOCTYPE. Here the scanner refuses
    // at byte 0 and produces no token vector at all.
    assert_eq!(source.matches("<!ENTITY").count(), 11);
    let error = from_xml(&source).expect_err("bomb must be refused");
    assert_eq!(error.code(), "E-XML-001");
    assert!(scanner::scan(&source).is_err(), "no tokens may be built");
    assert!(source.contains("&lol10;"), "fixture is a real bomb shape");
}

#[test]
fn undeclared_entity_names_the_reference_and_its_position() {
    let error = expect_code("undeclared_entity.xml", "E-XML-003");
    match error {
        XmlError::UndeclaredEntity { name, line, column } => {
            assert_eq!(name, "xx");
            assert_eq!((line, column), (1, 7));
        }
        other => panic!("expected UndeclaredEntity, got {other:?}"),
    }
}

#[test]
fn unsupported_bang_forms_are_refused() {
    let error = expect_code("unsupported_bang.xml", "E-XML-004");
    assert!(error.to_string().contains("1"));
}

#[test]
fn undeclared_prefix_is_refused() {
    let error = expect_code("undeclared_prefix.xml", "E-XML-013");
    match error {
        XmlError::UndeclaredPrefix {
            prefix,
            line,
            column,
        } => {
            assert_eq!(prefix, "p");
            assert_eq!((line, column), (1, 1));
        }
        other => panic!("expected UndeclaredPrefix, got {other:?}"),
    }
}

#[test]
fn duplicate_attribute_is_refused() {
    let error = expect_code("duplicate_attr.xml", "E-XML-009");
    match error {
        XmlError::DuplicateAttribute { name, line, column } => {
            assert_eq!(name, "a");
            assert_eq!((line, column), (1, 13));
        }
        other => panic!("expected DuplicateAttribute, got {other:?}"),
    }
}

#[test]
fn a_depth_bomb_hits_the_cap_before_allocation_grows() {
    let source = fixture("depth_bomb.xml");
    let error = from_xml(&source).expect_err("300 levels exceed the default 256");
    assert_eq!(error.code(), "E-XML-011");
    assert_eq!(error, XmlError::DepthLimitExceeded { limit: 256 });
    // Raising the cap reads the same document; the refusal is a configured
    // limit, not a parser limitation.
    let options = XmlOptions {
        max_depth: 400,
        ..XmlOptions::default()
    };
    assert!(from_xml_with_options(&source, &options).is_ok());
}
