---

title: 개발 환경 설정
lang: ko

## 개발 환경 설정

pyrs-yaml에 기여하기 위한 환경을 설정합니다.

### 사전 요구사항

- **Python** ≥ 3.8 (CPython)
- **Rust** ≥ 1.70 ([rustup](https://rustup.rs/) 경유)
- **Git**
- **uv** (권장) 또는 **pip**
- **NumPy** — NumPy 직렬화 테스트 스위트 실행에 필요 (`uv run --frozen pytest tests/test_numpy.py`)

### 클론 및 설치

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml

# uv 사용 (권장)
uv sync

# 또는 pip 사용
pip install maturin
uv run --frozen maturin develop --release
```

### 설치 확인

```bash
# Rust 테스트 실행
cargo test

# Python 테스트 실행
uv run --frozen pytest tests/

# 벤치마크 실행
cargo bench
```

### 프로젝트 구조

```text
pyrs-yaml/
├── src/
│   ├── lib.rs              # PyO3 모듈 정의
│   ├── ast.rs              # 사용자 정의 AST (CustomNode)
│   ├── parser/
│   │   ├── mod.rs          # saphyr-parser 통합
│   │   └── yaml/           # YAML 특정 파싱
│   │       ├── comment.rs  # 주석 추출
│   │       ├── merge.rs    # 병합 키 해석
│   │       ├── scalar.rs   # 스칼라 파싱
│   │       └── types.rs    # YAML 1.2 타입 해석
│   └── serializer.rs       # YAML 직렬화
├── python/pyrs_yaml/
│   ├── __init__.py         # Python 패키지 초기화
│   ├── pyrs_yaml.pyi       # 타입 스텁
│   └── py.typed            # PEP 561 마커
├── tests/                  # Python 테스트 스위트
├── benches/                # Rust 벤치마크
└── docs/                   # 문서 (mkdocs)
```

### 빌드 명령어

```bash
# Python 확장 빌드
uv run --frozen maturin develop --release

# wheel 빌드
maturin build --release --out dist

# 디버그 정보 포함 빌드
cargo build
```

### 개발 워크플로우

1. **먼저 테스트 작성** (TDD)
2. `src/`에서 **변경 구현**
3. **`cargo test` 실행**하여 Rust 테스트 확인
4. **`uv run --frozen pytest tests/` 실행**하여 Python 테스트 확인
5. **`cargo clippy -- -D warnings` 실행**하여 코드 품질 확인
6. **`cargo fmt` 실행**하여 코드 포맷
