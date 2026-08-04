//! PyO3-specific extensions for Segment.

use pyo3::prelude::*;
use std::borrow::Cow;

use crate::editing::Segment;

/// Extension trait for Segment to support Python parsing.
pub trait SegmentExt {
    /// Parse a Python object into a Segment.
    fn from_py(obj: &Bound<'_, PyAny>) -> PyResult<Segment<'static>>;
}

impl SegmentExt for Segment<'_> {
    fn from_py(obj: &Bound<'_, PyAny>) -> PyResult<Segment<'static>> {
        if let Ok(s) = obj.extract::<String>() {
            Ok(Segment::Key(Cow::Owned(s)))
        } else if let Ok(i) = obj.extract::<i64>() {
            Ok(Segment::Index(i))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "segment must be str or int",
            ))
        }
    }
}
