---

title: PyYAML 호환성
lang: ko

## PyYAML 호환성

pyyaml-rs는 PyYAML의 **드롭인 교체**를 제공하여 마이그레이션을 간소화합니다.

### 간단한 마이그레이션

```python
# 이전 코드
import yaml
data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)

# 새 코드
import pyyaml_rs as yaml
data = yaml.safe_load(yaml_text)
yaml_str = yaml.safe_dump(data)
```

### 호환 API

| PyYAML 함수 | pyyaml-rs 동등한 것 | 설명 |
|-------------|----------------|------|
| `yaml.safe_load()` | `pyyaml_rs.safe_load()` | ✅ 동일 |
| `yaml.safe_loads()` | `pyyaml_rs.safe_loads()` | ✅ 동일 |
| `yaml.safe_dump()` | `pyyaml_rs.safe_dump()` | ✅ 동일 |
| `yaml.safe_dumps()` | `pyyaml_rs.safe_dumps()` | ✅ 동일 |
| `yaml.load()` | `pyyaml_rs.safe_load()` | ⚠️ 안전 변형 사용 |
| `yaml.dump()` | `pyyaml_rs.safe_dump()` | ⚠️ 안전 변형 사용 |

### 주요 차이점

#### pyyaml-rs가 더 우수한 점

| 기능 | PyYAML | pyyaml-rs |
|------|--------|-----------|
| 순환 보존 | ❌ 주석/앵커 잃음 | ✅ 모든 것 보존 |
| 성능 | 기준 | **25-40배 빠름** |
| 타입 힌트 | 부분적 | ✅ 완전한 `.pyi` 스텁 |
| ABI3 wheel | 해당 없음 | ✅ 모든 Python 버전에 단일 wheel |
| i18n 오류 | ❌ 영어만 | ✅ 영어 + 중국어 |

#### 주의사항

1. **앵커/별칭 처리**: PyYAML은 순환 시 앵커를 잃음; pyyaml-rs는 보존
2. **주석 위치**: pyyaml-rs는 복잡한 중첩 구조에서 일부 주석의 순서를 변경할 수 있음
3. **플로우 스타일**: 둘 다 보존하지만 출력 포맷이 약간 다를 수 있음
4. **오류 메시지**: pyyaml-rs는 더 많은 컨텍스트를 가진 i18n 오류 메시지를 사용

### 마이그레이션 체크리스트

- [ ] `import yaml`을 `import pyyaml_rs as yaml`로 교체
- [ ] 모든 YAML 파싱/저장 워크플로우 테스트
- [ ] 순환 출력이 예상과 일치하는지 확인
- [ ] 앵커/별칭 동작 확인 (사용하는 경우)
- [ ] 사용자 정의 오류 메시지를 위한 오류 처리 검토

### 마이그레이션 예시

```python
# 이전 코드
import yaml

def load_config(path):
    with open(path) as f:
        return yaml.safe_load(f)

def save_config(data, path):
    with open(path, "w") as f:
        yaml.safe_dump(data, f)

# 새 코드
import pyyaml_rs

def load_config(path):
    return pyyaml_rs.parse_file(path).to_dict()

def save_config(data, path):
    pyyaml_rs.dump_file(data, path)
```
