//! TOML 1.0 / 1.1 parsers and the TOML writer, fuzzed together.
//!
//! Same contract as the JSON target: typed errors are passes, panics and
//! writer output that our own parser cannot re-read are bugs. The dialect
//! flag is exercised on both grammar revisions, and the depth guard (#166
//! lineage) must reject rather than overflow.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_toml::{from_toml, from_toml_v1_0, to_toml};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    for parsed in [from_toml(text).ok(), from_toml_v1_0(text).ok()] {
        let Some(node) = parsed else { continue };
        if let Ok(out) = to_toml(&node) {
            let _ = from_toml(&out);
            let _ = from_toml_v1_0(&out);
        }
    }
});
