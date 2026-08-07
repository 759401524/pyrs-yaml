"""Shared YAML test data — Single Source of Truth for all test files.

All test files should import from here instead of hardcoding YAML strings.
This ensures consistency and makes it easy to update test data.
"""

# --- Parse samples (YAML strings without trailing newline) ---
SIMPLE_MAPPING = "key: value"
NESTED_MAPPING = "parent:\n  child: grandchild"
SEQUENCE = "- a\n- b\n- c"
MIXED = "name: test\nitems:\n  - 1\n  - 2\nflag: true"
MULTILINE_SCALAR = "description: |\n  Line one\n  Line two"
QUOTED_SCALAR = 'message: "hello world"'
EMPTY_DOCUMENT = ""
WITH_COMMENT = "key: value  # a comment"
ANCHOR = "defaults: &defaults\n  timeout: 30\nref: *defaults"
MERGE_KEY = "base: &base\n  a: 1\nb: &b\n  <<: *base\n  b: 2"
FLOW_MAPPING = "{key: value, num: 42}"
FLOW_SEQUENCE = "[a, b, c]"
NULL_VALUE = "key: null"
EMPTY_MAPPING = "{}"
EMPTY_SEQUENCE = "[]"

# --- Roundtrip samples (YAML strings WITH trailing newline) ---
ROUNDTRIP_SIMPLE = "key: value\n"
ROUNDTRIP_COMMENT = "# Comment\nkey: value\n"
ROUNDTRIP_INLINE_COMMENT = "key: value  # comment\n"
ROUNDTRIP_ANCHOR = "defaults: &defaults\n  timeout: 30\n"
ROUNDTRIP_TAG = "name: !!str John\n"
ROUNDTRIP_CHOMPS_TRIP = "key: |-\n  line1\n  line2\n"
ROUNDTRIP_NESTED = "parent:\n  child1: value1\n  child2: value2\n"
ROUNDTRIP_SEQUENCE = "- item1\n- item2\n- item3\n"
ROUNDTRIP_MIXED = "list:\n  - a\n  - b\nmapping:\n  key: value\n"
ROUNDTRIP_MULTI_ANCHOR = "a: &anchor1 val1\nb: &anchor2 val2\n"
ROUNDTRIP_EMPTY_KEY = "key1:\nkey2: value\n"
ROUNDTRIP_MERGE = "defaults: &defaults\n  timeout: 30\nproduction:\n  <<: *defaults\n  host: x\n"
ROUNDTRIP_CUSTOM_TAG = "key: !custom value\n"
ROUNDTRIP_EXPLICIT_KEY = "? [key1, key2]\n: value\n"
ROUNDTRIP_EMPTY_MAP = "{}\n"
ROUNDTRIP_EMPTY_SEQ = "[]\n"
ROUNDTRIP_FLOW_MAP = "{a: 1, b: 2}\n"
ROUNDTRIP_FLOW_SEQ = "[1, 2, 3]\n"
ROUNDTRIP_FLOW_NESTED = "{a: [1, 2], b: {c: 3}}\n"
ROUNDTRIP_INLINE_FLOW = "key: {a: 1, b: 2}\n"
ROUNDTRIP_MERGE_KEYS = "base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2\n"

# --- Multi-document samples ---
MULTI_DOC_TWO = "a: 1\n---\nb: 2"
MULTI_DOC_THREE = "---\na: 1\n---\nb: 2\n---\nc: 3"
MULTI_DOC_WITH_COMMENTS = "# doc1\na: 1\n---\n# doc2\nb: 2"
MULTI_DOC_SEPARATORS = "---\nfirst: 1\n---\nsecond: 2"

# --- Type resolution samples ---
BOOL_TRUE = "key: true"
BOOL_FALSE = "key: false"
NULL_VARIANTS = "key: null"
INTEGER = "key: 42"
INTEGER_NEGATIVE = "key: -17"
FLOAT = "key: 3.14"
FLOAT_NEGATIVE = "key: -0.5"
OCTAL = "key: 0o14"
HEX = "key: 0x0C"
SCIENTIFIC = "key: 6.022e23"
INFINITY = "key: .inf"
NEG_INFINITY = "key: -.inf"
NAN = "key: .nan"
PLAIN_STRING = "hello"
SINGLE_QUOTED = "key: 'value'"
DOUBLE_QUOTED = 'key: "value"'
SPECIAL_CHARS = 'key: "value:with:colons"'

# --- Block scalar samples ---
LITERAL_BLOCK = "key: |\n  line1\n  line2"
FOLDED_BLOCK = "key: >\n  this is\n  folded"
STRIP_BLOCK = "key: |-\n  line1\n  line2"
KEEP_BLOCK = "key: |+\n  line1\n  line2\n"
FOLDED_STRIP = "key: >-\n  this is folded"

# --- Tag samples ---
TAG_STR = "name: !!str John"
TAG_CUSTOM = "name: !custom value"
TAG_INT = "age: !!int 30"
NESTED_TAGGED = "outer:\n  inner: !custom value"

# --- Anchor and merge samples ---
ANCHOR_SIMPLE = "defaults: &defaults\n  timeout: 30"
ANCHOR_MERGE = "defaults: &d\n  v: 1\nprod:\n  <<: *d"
ANCHOR_MERGE_OVERRIDE = "defaults: &d\n  timeout: 30\nprod:\n  <<: *d\n  host: prod.com"
ANCHOR_FILE = "base: &base\n  x: 1\nderived:\n  <<: *base\n  y: 2"
ANCHOR_MERGED_VIEW = "base: &base\n  a: 1\nmerged:\n  <<: *base\n  b: 2"

# --- Comment samples ---
COMMENT_TOP = "# This is a comment\nkey: value"
COMMENT_INLINE = "key: value  # inline comment"
COMMENT_BOTH = "# Comment\nkey: value  # inline"

# --- Explicit key samples ---
EXPLICIT_KEY_SEQ = "? [key1, key2]\n: value"
EXPLICIT_KEY_MAP = "? {a: 1}\n: value"

# --- Complex nested samples ---
NESTED_DEEP = "a:\n  b:\n    c:\n      d: value"
NESTED_DEEP_5 = "a:\n  b:\n    c:\n      d:\n        e: value"
NESTED_FLOW_BLOCK = "outer:\n  items: [a, b]\n  nested:\n    key: value"
NESTED_SEQ_OF_MAPS = "- name: Alice\n  age: 30\n- name: Bob\n  age: 25"
NESTED_MAP_OF_SEQS = "fruits:\n  - apple\n  - banana\nvegetables:\n  - carrot"
NESTED_SERVER = "\nserver:\n  host: localhost\n  port: 8080\ndatabase:\n  driver: postgresql\n  host: db.example.com\n  port: 5432\n"

# --- Error samples ---
INVALID_YAML = "{{invalid yaml"
INVALID_COLON = "key: value: extra_colon"
INVALID_UTF8 = "key: \x00value"
INVALID_JSON = "{invalid json"

# --- Edge case samples ---
CRLF_LINE_ENDINGS = "key: value\r\nlist:\r\n  - a\r\n  - b"
DUPLICATE_KEYS = "key: first\nkey: second"
DUPLICATE_KEYS_OVERRIDE = "key: first\nkey: second"
EMPTY_VALUE = "key:"
NESTED_SEQUENCE = "- - nested1\n  - nested2\n- top"
SEQUENCE_SINGLE = "- a"
TAG_SEQ = "items: !!seq [a, b]"
COMMENT_ON_VALUE = "key: value  # comment on value\n"

# --- Schema profile samples ---
SCHEMA_BOOL = "x: true\ny: false"
SCHEMA_NULL = "a:\nb: ~\nc: null\nd: NULL\ne: Null"
SCHEMA_INTEGER = "a: 42\nb: -10\nc: 0"
SCHEMA_OCTAL = "x: 0o10\ny: 0O77"
SCHEMA_HEX = "x: 0xFF\ny: 0X0A"
SCHEMA_FLOAT = "a: 3.14\nb: 1e10\nc: -1.5E-3"
SCHEMA_INF_NAN = "a: .inf\nb: -.inf\nc: .nan\nd: nan"
SCHEMA_STRING = "a: hello\nb: 12abc"
SCHEMA_JSON_INF_NAN = "a: .inf\nb: 0xFF"
SCHEMA_YAML11_BOOL = "a: yes\nb: on"
SCHEMA_FAILSAFE = "a: true\nb: 42"
SCHEMA_YAML11_BOOL_WORDS = "a: yes\nb: no\nc: Yes\nd: No\ne: YES\nf: NO"
SCHEMA_YAML11_BOOL_SHORT = "a: y\nb: n\nc: Y\nd: N"
SCHEMA_YAML11_ON_OFF = "a: on\nb: off\nc: On\nd: Off\ne: ON\nf: OFF"
SCHEMA_MULTI = "---\na: .inf\n---\nb: 0xFF"

# --- File write samples ---
FILE_SIMPLE = "key: value\nlist:\n  - a\n  - b"
FILE_NAME_VALUE = "name: test\nvalue: 42"
FILE_WITH_COMMENT = "# comment\nkey: value\n"

# --- Serialize options samples ---
LONG_VALUE = "key: " + "x" * 100

# --- Misc samples ---
SIMPLE_INT = "42"
SIMPLE_MAPPING_FILE = "a: 1\nb: 2"
SIMPLE_MAPPING_XY = "x: y"
SIMPLE_MAPPING_SERIALIZE = "n: 42\nf: 3.14\nb: true\ns: hello"
SIMPLE_SEQUENCE = "- 1\n- 2\n- 3"
SIMPLE_STRING = "hello world"
USER_MODEL = "name: Alice\nage: 30\n"
USER_MODEL_INVALID = "name: Alice\nage: not_an_int\n"
PRODUCT_MODEL = "name: Widget\nprice: 9.99\nin_stock: true\n"

# --- Benchmark samples (larger, for performance testing) ---

BENCHMARK_SMALL = """
# Application config
app:
  name: pyrs-yaml
  version: 0.2.0
  debug: false
  log_level: info
"""

BENCHMARK_MEDIUM = """
# Server configuration
server:
  host: 0.0.0.0
  port: 8080
  ssl: true
  workers: 4

database:
  type: postgresql
  host: db.example.com
  port: 5432
  name: myapp
  pool:
    min_size: 5
    max_size: 20
    timeout: 30

logging:
  level: INFO
  format: "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
  handlers:
    console:
      class: logging.StreamHandler
      stream: sys.stdout
    file:
      class: logging.FileHandler
      filename: app.log
      max_bytes: 10485760

features:
  authentication:
    enabled: true
    provider: oauth2
    token_expiry: 3600
  rate_limiting:
    enabled: true
    requests_per_minute: 100
  caching:
    enabled: true
    ttl: 300
"""

BENCHMARK_LARGE = """
# Complex application configuration with all YAML features
---
metadata:
  title: "YAML Test Suite — Large Config"
  version: 1.0
  author:
    name: Test User
    email: test@example.com
  tags:
    - production
    - configuration
    - benchmark

# Anchors and aliases
defaults: &defaults
  timeout: 30
  retries: 3
  backoff: exponential

services:
  api:
    <<: *defaults
    port: 8080
    endpoints:
      - path: /api/v1/users
        methods: [GET, POST]
      - path: /api/v1/users/{id}
        methods: [GET, PUT, DELETE]
      - path: /api/v1/orders
        methods: [GET, POST, PATCH]

  worker:
    <<: *defaults
    concurrency: 8
    queue:
      type: redis
      url: redis://localhost:6379/0

description: |
  This is a literal block scalar that
  preserves newlines and formatting.
  It's used for multi-line strings.

formatted: >
  This is a folded block scalar that
  converts newlines to spaces.
  Useful for wrapping long text.

chomped: |+
  Keep all trailing newlines


stripped: |-
  Remove all trailing newlines

# Flow collections
flow_mapping: {key: value, another: 42}
flow_sequence: [1, 2, 3, 4, 5]

# Special values
null_value: null
empty_value: ~
boolean: true
float_value: 3.14159
integer: 42
octal: 0o77
hexadecimal: 0xFF
scientific: 1.23e-4
infinity: .inf
nan: .nan

# Tags
explicit_string: !!str 123
explicit_int: !!int 0xFF
explicit_bool: !!bool yes
explicit_null: !!null ~

# Comments everywhere
database:  # main database connection
  host: localhost
  port: 5432
  name: mydb
  credentials:  # authentication info
    username: admin
    password: secret123

cache:
  backend: redis
  url: "redis://localhost:6379/1"
  key_prefix: "app:"
  serializers:
    - pickle
    - json

monitoring:
  enabled: true
  metrics:
    - cpu_usage
    - memory_usage
    - disk_usage
    - network_io
    - request_count
    - error_count
  tags:
    environment: production
    region: us-east-1
    team: platform
"""

BENCHMARK_MULTI_DOC = "\n---\n".join(
    f"doc: {index}\nvalues: [1, 2, 3]\nnested:\n  key: value_{index}\n" for index in range(20)
)

BENCHMARK_BLOCK_STYLE = (
    "key1: value1\n"
    "key2: value2\n"
    "nested:\n"
    "  subkey1: subvalue1\n"
    "  subkey2: subvalue2\n"
    "list:\n"
    "  - item1\n"
    "  - item2\n"
    "  - item3\n"
)

BENCHMARK_CONFIG_DATA = {
    "server": {
        "host": "0.0.0.0",
        "port": 8080,
        "ssl": True,
        "workers": 4,
        "tags": ["production", "eu-west-1"],
    },
    "database": {
        "type": "postgresql",
        "pool": {"min_size": 5, "max_size": 20, "timeout": 30},
    },
    "items": [{"name": f"item_{index}", "value": index * 10} for index in range(50)],
}

BENCHMARK_CONFIG_JSON = (
    '{"server": {"host": "0.0.0.0", "port": 8080, "ssl": true}, '
    '"items": [{"name": "a", "value": 1}, {"name": "b", "value": 2}]}'
)

BENCHMARK_SCHEMA = {
    "type": "object",
    "required": ["server", "database"],
    "properties": {
        "server": {
            "type": "object",
            "required": ["host", "port"],
            "properties": {
                "host": {"type": "string"},
                "port": {"type": "integer"},
                "ssl": {"type": "boolean"},
            },
        },
        "database": {"type": "object"},
    },
}

BENCHMARK_ANCHOR = """
defaults: &defaults
  timeout: 30
  retries: 3
  backoff: exponential

api:
  <<: *defaults
  port: 8080

worker:
  <<: *defaults
  concurrency: 8
"""
