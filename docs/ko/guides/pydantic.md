---
title: Pydantic 통합
description: YAML을 Pydantic v2 모델로 파싱하고, 모델을 YAML로 직렬화하며, pyrs-yaml을 파서로 사용하여 BaseSettings를 로드합니다.
tags:
  - docs
status: new
---

## Pydantic 통합

pyrs-yaml은 [Pydantic](https://docs.pydantic.dev/) v2와
[pydantic-settings](https://docs.pydantic.dev/latest/concepts/pydantic_settings/)와
통합되어 YAML을 검증된 모델로 변환하고 그 반대도 수행합니다. 두 모두 선택적 의존성입니다:

- 모델 파싱/직렬화: `pip install pydantic` (또는 `pip install 'pyrs-yaml[pydantic]'`)
- `BaseSettings` 로드: `pip install 'pyrs-yaml[settings]'` (`pydantic-settings` 포함)

### YAML을 모델로 파싱

`parse_as()`는 YAML을 파싱하고 Pydantic `BaseModel` 서브클 Against against 검증하여
모델 인스턴스를 반환합니다. `**yaml_kwargs`는 `YAML()` 생성자에 전달됩니다
(예: `resolve_merges`).

```python title="Parse YAML into a Pydantic model"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
print(user.age)  # 30
```

`parse_as()`는 다음을 발생시킵니다:

- `ImportError` — pydantic이 설치되지 않음
- `TypeError` — `model`이 `BaseModel` 서브클레스가 아님
- `pydantic.ValidationError` — 파싱된 데이터가 모델 검증에 실패함

### 모델을 YAML로 직렬화

`dump_pydantic()`는 Pydantic 모델을 YAML 문자열로 직렬화합니다. 먼저
`model_dump(mode="json")`를호출하여 문자열 형식 필드를 유지합니다
(「10001」과 같은 우편번호가 정수로 강제 변환되지 않음) — 그 후 `safe_dump`에 위임합니다.

```python title="Serialize a Pydantic model to YAML"
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
print(yaml_str)
# name: Alice
# age: 30
```

`dump_pydantic()`는 다음을 발생시킵니다:

- `ImportError` — pydantic이 설치되지 않음
- `TypeError` — `model`이 `BaseModel` 인스턴스가 아님

### pydantic-settings로 설정 로드

`PyrsYamlConfigSettingsSource`는 `pydantic_settings.YamlConfigSettingsSource`의
동등한 대체제입니다. pyrs-yaml의 YAML 1.2 파서로 YAML 설정 파일을 읽고,
env 변수·dotenv·시크릿과 함께 `BaseSettings` 모델에 값을 공급합니다. 우선순위와
동작은 동일합니다.

```python title="Load BaseSettings from a YAML file"
from pydantic_settings import BaseSettings, SettingsConfigDict
import pyrs_yaml


class Settings(BaseSettings):
    app_name: str

    model_config = SettingsConfigDict(yaml_file="config.yaml")

    @classmethod
    def settings_customise_sources(
        cls, settings_cls, init_settings, env_settings, dotenv_settings, file_secret_settings
    ):
        return (
            init_settings,
            env_settings,
            dotenv_settings,
            file_secret_settings,
            pyrs_yaml.PyrsYamlConfigSettingsSource(settings_cls),
        )
```

이 소스는 `YamlConfigSettingsSource`와 동일한 옵션을 지원합니다:

- `yaml_file` — 경로 또는 경로 목록 (`SettingsConfigDict`로 지정 또는 직접 전달)
- `yaml_file_encoding` — 파일 인코딩
- `yaml_config_section` — 점 표기법 중첩 섹션 경로
- `deep_merge` — 여러 파일을 교체가 아닌 깊이 병합

!!! note "지연 import"
    `import pyrs_yaml`만으로는 pydantic 또는 pydantic-settings가 필요하지 않습니다.
    해당 의존성이 설치되지 않은 상태에서 `pyrs_yaml.parse_as`,
    `pyrs_yaml.dump_pydantic`, `pyrs_yaml.PyrsYamlConfigSettingsSource`에
    접근하면 설치 힌트와 함께 `ImportError`가 발생합니다.

### 주석 포함 라운드트립

`parse_as()`는 `safe_load`를 기반으로 하므로, 주석과 앵커 보존은
모델 경로에는 포함되지 않습니다 — 라운드트립 편집이 필요한 경우 `parse()`와
`YamlDocument`를 사용하고, 검증된 모델이 필요할 때만 `parse_as()`를 사용합니다.

!!! tip "올바른 파싱 경로 선택"
    설정 검증에는 `parse_as()`를, 주석·암커·서식이 라운드트립에서
    유지되어야 할 경우 `parse()`를 사용합니다.

### 참고

- [YAML 파싱](parsing.md) — 문자열·파일·여러 문서 파싱
- [직렬화](serialization.md) — YAML 문서와 Python 오브젝트의 상호 변환
- [설정 관리](tutorial-config-management.md) — 엔드투엔드 워크스루
- [API 참조](../api/reference.md) — `parse_as`, `dump_pydantic`, `PyrsYamlConfigSettingsSource`의 전체 시그니처
