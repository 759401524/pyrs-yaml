"""
NumPy array serialization tests — dump ndarray to YAML lists.

Round-trip guarantee: safe_dump → safe_load preserves values and shape.
"""

import pytest

import pyrs_yaml

# Skip entire module if numpy is not installed
numpy = pytest.importorskip("numpy")


class TestNumpyScalar:
    """Test scalar (0-D) array serialization."""

    def test_int32_scalar(self):
        arr = numpy.array(42, dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == 42

    def test_float64_scalar(self):
        arr = numpy.array(3.14, dtype="float64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert abs(data - 3.14) < 1e-10

    def test_bool_scalar(self):
        """0-D bool: reshaped to 1-D and serialized as a single-item list.
        numpy-rs 0-D bool arrays have a known dtype-matching quirk that may
        serialize as an integer (0/1) instead of a boolean string."""
        arr = numpy.array(True, dtype="bool")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        # The round-trip value is either True (ideal) or 1 (numpy-rs quirk)
        assert data is True or data == 1


class TestNumpy1D:
    """Test 1-D array serialization for all supported dtypes."""

    @pytest.mark.parametrize(
        "dtype, values, expect",
        [
            ("int8", [1, 2, 3], [1, 2, 3]),
            ("int16", [-100, 200], [-100, 200]),
            ("int32", [42, -42, 0], [42, -42, 0]),
            ("int64", [1000000, -1000000], [1000000, -1000000]),
            ("uint8", [0, 127, 255], [0, 127, 255]),
            ("uint16", [100, 200], [100, 200]),
            ("uint32", [1000, 2000], [1000, 2000]),
            ("uint64", [10000, 20000], [10000, 20000]),
            ("float32", [1.5, -2.5], [1.5, -2.5]),
            ("float64", [1.5, -2.5], [1.5, -2.5]),
        ],
    )
    def test_integer_float(self, dtype, values, expect):
        arr = numpy.array(values, dtype=dtype)
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == expect

    def test_bool(self):
        arr = numpy.array([True, False, True])
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == [True, False, True]

    def test_nan(self):
        """NaN values should be serialized as the literal string 'NaN'."""
        arr = numpy.array([1.0, float("nan"), 2.0])
        yaml_str = pyrs_yaml.safe_dump(arr)
        assert "NaN" in yaml_str

    def test_negative_roundtrip(self):
        """Negative numbers must round-trip as integers, not strings."""
        arr = numpy.array([-100, -42, -255], dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == [-100, -42, -255]
        # Verify types are int, not str
        assert all(isinstance(x, int) for x in data)

    def test_mixed_positive_negative(self):
        arr = numpy.array([0, -1, 1, -2, 2], dtype="int16")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == [0, -1, 1, -2, 2]


class TestNumpy2D:
    """Test 2-D array serialization."""

    def test_int32_matrix(self):
        arr = numpy.array([[1, 2], [3, 4]], dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == [[1, 2], [3, 4]]

    def test_float64_matrix(self):
        """Float matrix round-trip — use numpy for comparison (pytest.approx doesn't
        support nested lists)."""
        arr = numpy.array([[1.1, 2.2], [3.3, 4.4]], dtype="float64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        expected = [[1.1, 2.2], [3.3, 4.4]]
        numpy.testing.assert_array_almost_equal(data, expected)

    def test_negative_matrix(self):
        arr = numpy.array([[-1, -2], [-3, -4]], dtype="int64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == [[-1, -2], [-3, -4]]

    def test_float_negative_matrix(self):
        arr = numpy.array([[-1.5, 2.5], [3.5, -4.5]], dtype="float32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        numpy.testing.assert_array_almost_equal(data, [[-1.5, 2.5], [3.5, -4.5]])


class TestNumpy3D:
    """Test 3-D array serialization."""

    def test_int64_cube(self):
        arr = numpy.array([[[1, 2], [3, 4]], [[5, 6], [7, 8]]], dtype="int64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]

    def test_3d_with_negatives(self):
        arr = numpy.array([[[1, -2], [3, -4]], [[5, -6], [7, -8]]], dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == [[[1, -2], [3, -4]], [[5, -6], [7, -8]]]


class TestNumpy4D:
    """Test 4-D array serialization (edge case for multi-dimensional nesting)."""

    def test_4d_int(self):
        arr = numpy.arange(1, 17).reshape(2, 2, 2, 2).astype("int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert numpy.array(data).tolist() == arr.tolist()


class TestNumpyComplex:
    """Test complex dtype serialization."""

    def test_complex64(self):
        """Complex64 is serialized as '(re+imj)' format. Note: integer parts
        drop trailing '.0' (Rust format! behavior)."""
        arr = numpy.array([1.0 + 2.0j, 3.0 + 4.0j], dtype="complex64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        # Rust format!("({}+{}j)", 1.0f32, 2.0f32) → "(1+2j)" (drops .0 for whole numbers)
        assert "(1+2j)" in yaml_str
        assert "(3+4j)" in yaml_str

    def test_complex128(self):
        arr = numpy.array([1.0 + 2.0j], dtype="complex128")
        yaml_str = pyrs_yaml.safe_dump(arr)
        assert "(1+2j)" in yaml_str

    def test_complex_roundtrip_dump_only(self):
        """Complex numbers serialize correctly but YAML has no native complex type;
        safe_load returns them as strings, not Python complex."""
        arr = numpy.array([1.0 + 2.0j], dtype="complex64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        # YAML stores complex as string representation; load returns it as str
        assert data == ["(1+2j)"]


class TestNumpyEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_1d(self):
        """Empty 1-D array: serializes to the explicit empty sequence and round-trips back."""
        arr = numpy.array([], dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        # Empty collections emit [] (not an empty document that loads as null).
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == []

    def test_empty_2d(self):
        arr = numpy.empty((0, 3), dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == []

    def test_empty_3d(self):
        arr = numpy.empty((2, 0, 3), dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data == []

    def test_single_element(self):
        """A 1-D single-element array keeps its dimension.

        It used to answer `42` on one numpy and `[42]` on another: Python's number protocol
        converts a one-element array (a warning until numpy 2.2, an error from 2.3 on), and that
        conversion ran ahead of the array route. See `TestNumpyBufferRoute`.
        """
        arr = numpy.array([42], dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        assert yaml_str == "- 42\n"
        assert pyrs_yaml.safe_load(yaml_str) == [42]

    def test_unsupported_dtype_raises(self):
        """Unsupported dtypes (e.g., string) should raise YamlTypeError."""
        arr = numpy.array(["hello", "world"])
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.safe_dump(arr)

    def test_non_ndarray_raises(self):
        """Non-ndarray objects should raise YamlTypeError."""
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.safe_dump(object())

    def test_from_dict_with_ndarray(self):
        """Arrays inside dicts should be serialized correctly."""
        data = {"matrix": numpy.array([[1, 2], [3, 4]], dtype="int32"), "label": "test"}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "matrix:" in yaml_str
        assert "- 1" in yaml_str

    def test_roundtrip_nested_dict(self):
        """Round-trip through dict should preserve ndarray shape as list."""
        data = {"data": numpy.array([1, 2, 3], dtype="float64")}
        yaml_str = pyrs_yaml.safe_dump(data)
        loaded = pyrs_yaml.safe_load(yaml_str)
        assert loaded["data"] == [1.0, 2.0, 3.0]

    def test_dump_file_with_ndarray(self):
        """ndarray should be serialized to file and read back."""
        import tempfile
        from pathlib import Path

        with tempfile.NamedTemporaryFile(suffix=".yaml", delete=False) as f:
            path = Path(f.name)
        try:
            data = {"matrix": numpy.array([[1, 2], [3, 4]], dtype="int32")}
            pyrs_yaml.dump_file(data, str(path))
            doc = pyrs_yaml.parse_file(str(path))
            assert doc.get("matrix") == [[1, 2], [3, 4]]
        finally:
            if path.exists():
                path.unlink()

    def test_ndarray_in_list(self):
        """A list containing an ndarray should serialize correctly."""
        data = [1, numpy.array([10, 20], dtype="int32"), 3]
        yaml_str = pyrs_yaml.safe_dump(data)
        loaded = pyrs_yaml.safe_load(yaml_str)
        assert loaded == [1, [10, 20], 3]

    def test_yaml_dump_indentation_2d(self):
        """2-D array YAML output should have correct indentation for nested sequences.
        Root-level sequence items with nested sequences use block format:
            - <empty>
              - item
              - item
        """
        arr = numpy.array([[1, 2], [3, 4]], dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        lines = yaml_str.strip().split("\n")
        # Root-level items have `- ` on their own line, nested items indented
        assert lines[0] == "- "
        assert lines[1] == "  - 1"
        assert lines[2] == "  - 2"
        assert lines[3] == "- "
        assert lines[4] == "  - 3"
        assert lines[5] == "  - 4"

    def test_floats_with_infinity(self):
        """Infinity values should serialize and round-trip."""
        arr = numpy.array([float("inf"), -float("inf"), 1.0], dtype="float64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert data[0] == float("inf")
        assert data[1] == float("-inf")
        assert data[2] == 1.0

    def test_float_nan(self):
        """NaN should serialize as literal NaN string."""
        arr = numpy.array([float("nan")], dtype="float64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        assert "NaN" in yaml_str

    def test_nan_in_2d(self):
        arr = numpy.array([[1.0, float("nan")], [float("nan"), 4.0]], dtype="float64")
        yaml_str = pyrs_yaml.safe_dump(arr)
        assert "NaN" in yaml_str

    def test_large_1d(self):
        """Large 1-D array round-trip."""
        arr = numpy.arange(-10000, 10001, dtype="int32")
        yaml_str = pyrs_yaml.safe_dump(arr)
        data = pyrs_yaml.safe_load(yaml_str)
        assert len(data) == 20001
        assert data[0] == -10000
        assert data[-1] == 10000


class TestNumpyConcurrency:
    """Issue #165: serialization must snapshot the NumPy buffer under the GIL.

    Reading Python-owned memory inside ``py.detach`` (GIL released) let another
    thread mutate the array mid-serialize. The fix copies to owned memory first,
    so a concurrent writer that only runs during the detach must NOT appear in
    the output. Skipped on free-threaded builds (no GIL to release).
    """

    def test_serialize_snapshots_under_gil(self):
        import sys
        import threading

        # Free-threaded builds have no GIL to release; the premise does not apply.
        if getattr(sys, "_is_gil_enabled", lambda: True)() is False:
            pytest.skip("requires an enabled GIL")

        pyrs_yaml.safe_dump(numpy.zeros(2, dtype="int64"))  # warm dispatch
        array = numpy.zeros(500_000, dtype="int64")
        mutate = threading.Event()

        def writer():
            if mutate.wait(timeout=30):
                array[-1] = 123

        t = threading.Thread(target=writer, daemon=True)
        t.start()
        interval = sys.getswitchinterval()
        try:
            sys.setswitchinterval(60)  # keep this thread on-CPU through the copy
            mutate.set()  # writer unblocks only once we release the GIL
            serialized = pyrs_yaml.safe_dump(array)
        finally:
            sys.setswitchinterval(interval)
            mutate.set()
            t.join(timeout=30)

        # The copy happened before the detach, so the concurrent write is absent.
        tail = pyrs_yaml.safe_load(serialized)[-1]
        assert tail == 0, "serialization observed a write made after releasing the GIL"
        # The writer did run (the race window was exercised).
        assert array[-1] == 123


class TestNumpyBufferRoute:
    """The route reads the array's own ``tobytes()`` instead of its C API.

    The typed route cast the object and sliced its memory, which is not sound in the limited-ABI
    artifact the cp315 wheel is (0.18.3 died with ``Fatal Python error: Segmentation fault`` on
    ``safe_dump(numpy.arange(8))`` under 3.15 GIL) and, separately, read memory order rather than
    logical order. These rows pin what the buffer route must keep: the widths, the spellings, and
    the shape a non-contiguous array means.
    """

    def test_transposed_array_writes_what_it_means(self):
        arr = numpy.arange(4, dtype="int32").reshape(2, 2).T
        data = pyrs_yaml.safe_load(pyrs_yaml.safe_dump(arr))
        # The memory holds 0,1,2,3; the array says [[0, 2], [1, 3]].
        assert data == [[0, 2], [1, 3]]

    def test_fortran_ordered_array_writes_what_it_means(self):
        arr = numpy.asfortranarray(numpy.array([[1, 2], [3, 4]], dtype="int32"))
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(arr)) == [[1, 2], [3, 4]]

    def test_strided_array_is_dumped_not_refused(self):
        arr = numpy.arange(10, dtype="int64")[::2]
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(arr)) == [0, 2, 4, 6, 8]

    def test_one_element_array_is_not_read_as_a_number(self):
        """Python's number protocol must not be asked of an array carrying a dimension.

        `float(numpy.array([1.5]))` answers 1.5 on every numpy up to 2.2 and raises from 2.3 on
        ("only 0-dimensional arrays can be converted to Python scalars"), so a dump that let it
        through produced a scalar on one install and a sequence on another - and a `float32`
        element left as the double widened from it. These are the rows that went red on the
        3.8-3.10 legs of PR #669 for exactly that reason.
        """
        assert pyrs_yaml.safe_dump(numpy.array([1.5])) == "- 1.5\n"
        assert pyrs_yaml.safe_dump(numpy.array([True])) == "- true\n"
        assert pyrs_yaml.safe_dump(numpy.array([255], dtype="uint8")) == "- 255\n"

    def test_zero_dimensional_array_keeps_the_protocol_answer(self):
        """The other side of that boundary, stated so it cannot move unnoticed.

        A 0-D array is converted by the protocol on every numpy version, so it stays a scalar here
        - the spelling `docs/en/guides/numpy.md` documents. A 0-D *complex* has no protocol
        conversion at all and reaches the route, which is why it is the one 0-D shape that arrives
        as a sequence (`test_zero_dimensional_complex_is_a_one_item_sequence`).
        """
        assert pyrs_yaml.safe_dump(numpy.array(1.5)) == "1.5\n"
        assert pyrs_yaml.safe_dump(numpy.array(42, dtype="int32")) == "42\n"

    def test_both_writer_routes_agree_on_the_one_element_shape(self):
        """The direct writer and the AST conversion answer with the same shape.

        `safe_dump` walks Python objects into the text; a document edit converts through a
        `CustomNode` first. Both handed a one-element array to Python's number protocol on the
        numpy versions that still answer it, so this row is the parity claim between the two writer
        routes rather than a second copy of the rows above. `1.5` is exact in `float32`, so the
        projection is a float rather than the text the loader answers past its integer bound.
        """
        array = numpy.array([1.5], dtype="float32")
        doc = pyrs_yaml.parse("value: 0\n")
        doc.set("$.value", array)
        assert pyrs_yaml.safe_load(doc.to_yaml()) == {"value": [1.5]}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(array)) == [1.5]

    def test_float32_keeps_its_own_decimal(self):
        # The f32 rendering, not the decimal of the double it widens to: 1e30 stored as float32 is
        # 1000000015047466200000000000000 as a double, and the typed route printed the f32's shortest
        # round-tripping form.
        yaml_str = pyrs_yaml.safe_dump(numpy.array([1e30], dtype="float32"))
        assert yaml_str == "- 1000000000000000000000000000000\n"

    def test_integer_widths_are_read_at_their_full_extent(self):
        # Every width decodes whole: 64-bit unsigned keeps all twenty digits, and a negative 8-bit
        # value stays -1 rather than widening to 255.
        assert pyrs_yaml.safe_dump(numpy.array([2**64 - 1], dtype="uint64")) == ("- 18446744073709551615\n")
        assert pyrs_yaml.safe_dump(numpy.array([-(2**63)], dtype="int64")) == "- -9223372036854775808\n"
        assert pyrs_yaml.safe_dump(numpy.array([-1], dtype="int8")) == "- -1\n"
        assert pyrs_yaml.safe_dump(numpy.array([255], dtype="uint8")) == "- 255\n"
        # Past i64 the loader answers with text - the reader's own integer bound, not this route's
        # doing. The digits it is handed are already the whole 64-bit value.
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(numpy.array([2**64 - 1], dtype="uint64"))) == [
            "18446744073709551615"
        ]

    def test_zero_dimensional_complex_is_a_one_item_sequence(self):
        # The generic scalar paths take 0-d int and float; a complex reaches this route and keeps the
        # one-item sequence the typed route reshaped it into.
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(numpy.array(1.0 + 2.0j))) == ["(1+2j)"]

    def test_negative_imaginary_keeps_the_pairs_spelling(self):
        assert "(-1.25+-0.5j)" in pyrs_yaml.safe_dump(numpy.array([-1.25 - 0.5j]))

    def test_float16_is_still_outside_the_accepted_dtypes(self):
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.safe_dump(numpy.array([1.5], dtype="float16"))

    def test_a_registered_handler_is_asked_before_the_buffer_route(self):
        """An array a handler claims is the handler's scalar, on both writers.

        The buffer route reads the object's own ``dtype``/``shape``/``tobytes()``, so it used to be
        reached before the registry and answer with a sequence even though the caller had registered
        a class for it. A handler is the caller's own statement about a class, and the node route has
        always converted a value before asking any layout question - so both now ask the registry
        first. Without the handler, these rows are the buffer route's ordinary output.
        """

        class NdType(pyrs_yaml.CustomType):
            python_type = numpy.ndarray

            def to_yaml(self, obj):
                return "ND"

        pyrs_yaml.clear_type_handlers()
        try:
            pyrs_yaml.register_type("!nd", NdType())
            array = numpy.arange(4, dtype="int32")
            assert pyrs_yaml.safe_dump({"m": array}) == "m: !nd ND\n"
            doc = pyrs_yaml.parse("value: 0\n")
            doc.set("$.value", array)
            assert doc.to_yaml() == "value: !nd ND\n"
        finally:
            pyrs_yaml.clear_type_handlers()
        assert pyrs_yaml.safe_dump({"m": numpy.arange(4, dtype="int32")}) == "m:\n  - 0\n  - 1\n  - 2\n  - 3\n"
