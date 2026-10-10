---
title: Changelog
description: pyrs-yaml 项目的完整变更日志，记录所有版本的重要变更、新增功能和性能优化。
tags:

- docs
status: new

---

## 变更日志

本文件记录该项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-cn/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

### [Unreleased]

#### 新增

- **YAML 注释、锚点与标签可完成往返，并以每个种子作门禁。** —
  `crates/pyrs-yaml-core/tests/note_survival.rs` 在每次 `cargo nextest run` 重放已提交的种子语料，
  要求读取器记录的每条注释都出现在输出文本里。往返层的判据是文本幂等，任何「稳定但少一条注释」
  的文档都能通过 —— 五处静默注释丢失正是这样一路绿过来的。
  — (details: quality-ledger — Note survival, (w), (ac))

- **嵌套与被别名引用的合并键会被应用，而不再被丢弃。** — `<<: {<<: {x: 1}}` 原先把两层 `<<` 都当数据
  保留（`{'<<': {'<<': {'x': 1}}}`），而 PyYAML 与 ruamel 读出 `{'x': 1}`；
  `<<: {<<: {x: 1, y: 1}, y: 2}` 直接丢掉 `x`，块式 `- <<:` 嵌在 `<<:` 下同样丢失。收集器不再用整节
  点相等来判断目标「已拥有」某个源键。— (details: quality-ledger (ae), (af), (ag), (o), (q), (s))

- **指令数门禁覆盖每条跨格式桥，双向都数。** — `from_jsonc`、`from_json5`、`to_jsonc_text`、
  `to_json5_text`、`to_python_small/medium/anchors` 以及 JSON/TOML 读取路径都有了
  `.ci/ir-baseline.json` 基线。同一棵 hub AST 上三个写手实测 21,232,786 / 22,176,851 / 22,974,851
  条指令：带注释多花 4.4%，JSON5 拼写多花 8.2%。 — (details: quality-ledger (am), (ap), (aq), (as),
  (ax))

- **质量防线度量自身，缺口以 findings 呈现。** — `scripts/quality_matrix.py` 从声明这套防线的文件
  （`.github/workflows/*.yml`、`prek.toml`、`fuzz/Cargo.toml`、`scripts/check_*.py`、Ir 基准、
  `.ci/ir-baseline.json`）重新推导 `QUALITY_MATRIX.md`，而不是抄写它；每个洞都登记在
  `.ci/quality-holes.json`，两个方向都写明出口，并有注入树来触发。 — (details: quality-ledger (ah),
  (ai), (aj), (ak), (an), (ao))

- **改动产品的变更集必须同时改动发布说明。** — `scripts/check_changelog_mirrors.py` 之外的
  `scripts/check_changelog_coupling.py` 按 PR 文件清单执行两条规则：触及 `crates/`、
  `python/pyrs_yaml/`、`fuzz/`、`scripts/`、`tests/` 或随包清单的 diff 必须改动某个 changelog；改动
  五镜像之一就必须改全五个。这条规则是在某个特性发布时任何地方都没有发布说明之后补上的。
  — (details: quality-ledger (al), (ar))

- **计时地板线相邻采样两个阶段，并打印证据。** — `tests/timing.py` 成为唯一采样器：候选与参照在
  同一块内相邻测量，判据取块最小值并要求至少赢下两对，失败消息打印每一对，于是一次红色会说明是哪
  一侧移动。`macos-latest` 上报 `139.4us vs 359.6us (2.58x)` 而五块中有三块逼近自身地板线 3 倍，就是
  这种情况；跨库地板线保留 5 倍与前三阈值（实测相对 PyYAML 38x-280x，相对 ruamel 71x-77x）。
  — (details: quality-ledger (aw), (ay))

- **文档正文限制在 100 显示列；被格式化器切断的标题会被拒绝。** — `ROADMAP.md` 曾有一行 21,850 字
  符，`docs/ja/changelog.md` 有一行 884 字符。`scripts/check_doc_wrapping.py` 按显示列计量（全角字符
  算两列）并带 `--fix`；`check_doc_headings.py` 拒绝续行以 issue 引用开头时被格式化器升格成的标题。—
  (details: quality-ledger (au), (av))

- **每个引擎都有 fuzz 目标，每周定时运行，PR 层级阻塞。** — 四个 libFuzzer 目标：`parse_yaml`
  （单文档与流）、`yaml_roundtrip`（解析→序列化→再解析→序列化幂等）、`parse_json`（三种方言 × 三种
  写手，每个输出再解析）、`parse_toml`（1.0/1.1 加写手再解析），种子来自 `fuzz/seeds/`。仓库只跟踪
  目标与种子，机器语料永不入库。 — (details: quality-ledger (aj))

- **`pyrs-ast`、`pyrs-schema`、`pyrs-json`、`pyrs-toml` 具备 `no_std` 能力。** — 四个 crate 只依赖
  `alloc` 即可构建：`indexmap` 与 `thiserror` 关掉默认 `std` feature，新增可选 `std` feature 重新启
  用 `std::error::Error` 实现与 `RandomState` hasher。默认仍开 `std`，现有使用者的节点映射类型不变。
  — (details: boundaries — Known Engine Boundaries)

- **`pyq` 随每次发布提供预编译二进制。** — `publish.yml` 新增 `pyq` 作业，为六个平台构建原生 CLI 并
  把压缩包附到 GitHub Release，独立二进制不再需要用户自备 Rust 工具链。Linux 侧在 manylinux2014 容器
  内经 `cross` 原生编译，钉住 glibc 2.17 下限。— (details: perf — Leaderboard & Performance Status)

- **已提交类型存根会对照构建出的扩展检查。** — `scripts/check_stub_drift.py` 沿声明过的路由重新生成
  `python/pyrs_yaml/pyrs_yaml.pyi`，有任何差异即失败；该作业跑在 `windows-latest`，因为 maturin 的自
  省路由无法在 Linux 容器里加载扩展。此前 CI 只断言文件存在且被跟踪，于是一处绑定签名变化可以让公开
  类型契约悄悄落后。— (details: quality-ledger (ap))

- **schema 语言可以指称容器。** — `type: map` 与 `type: seq` 断言节点形状而不涉及其内容；`map` /
  `seq` 也可用作成员类型，于是 `mapping_of: seq` 得以表达「该映射的每个值都是序列」—— 此前无从
  写起，因为 `[*]` 通配只抵达序列下标而非映射键。`mapping`、`object`、`sequence`、`array`、`list`
  这些拼写同样接受，因为 schema 由人书写而非工具生成。不带 `path` 的容器类型在书写处即被拒绝：没有
  指名的节点，它就什么都选不出、什么都查不到。— (details: quality-ledger (bm))

- **给指令文件也装上量尺，而不是轻信它。** — `scripts/check_hole_claims.py` 断言 `AGENTS.md` 与
  `CLAUDE.md` 里点名的每个质量盲区 id，都是矩阵此刻量到的盲区。促成这条检查的文件正是 `AGENTS.md` 自
  己：它在整整一个里程碑里把 `mkdocstrings` 描述成「渲染不出任何东西」—— 一句现在时的指令，说的
  是 #321 已经关闭的缺口，于是继续把下一个读者指向错误的方向。测量本身没有错：矩阵会双向比对
  `.ci/quality-holes.json` 与 `QUALITY_MATRIX.md`，止步于那份派生文档。只有当某个 token 的 kind 是探
  针能发出的种类时才算盲区 id —— 种类是从 `quality_matrix.py` 读出来的，而不是在旁边抄一份 —— 所以正
  文里的 `line:column` 不是一条主张；读不全种类的检查会退 2，绝不假装通过。带日期的记录保留 id：历史
  可以提及已关闭的盲区，指令文件用的是现在时。— (details: quality-ledger (bn))

- **发布的签名比对已发布对象，而不是比对页面。** — `scripts/check_doc_signatures.py` 读取 28 个 API
  页面上的每个围栏签名，并询问运行中的包是否接受那些参数：238 个块、279 个名字。参数名写错仍是一份合
  法 HTML，所以站点会绿着发布出去；而只读生成 `.pyi` 的检查器会把 `read_markdown(path=...)` 判成文档
  的错 ——那恰好是文档对、库错的一例。合法的缩写（只写必需前缀、省略可选尾巴）放行；没有任何声明能匹
  配的名字、顺序或缺掉的必需参数则失败。`tests/test_stub_runtime_parity.py` 钉住更下层：桩里 40 个模
  块函数的参数名必须等于对象自身的——两个缺陷的共同成因就是这条不变量没人看。— (details:
  quality-ledger (bo))

#### 变更

- **给出 `path` 的校验规则会断言它所指名的节点。** — 改动前先测量：`$.config` 配 `mapping_of: str`
  在 `config` 是标量或序列时通过，`$.port` 配 `type: int` 在 `port` 是映射时通过，`sequence_of: int`
  对 `[[1, 2]]` 也通过。每项检查都写在一个 `if let` 里，只覆盖它能描述的那一种节点，其余节点被直接
  走过 —— 校验器对从未看过的文档回答「合法」。成员同样被断言：嵌套序列不是 `int`。不带路径的规则
  保留原先不同的语义：它无法指名节点，于是按形状筛选自己能描述的节点 —— 无路径的 `type: str` 仍意为
  「所有标量都是字符串」，不管文档其余部分是什么。关于别名节点的规则仍不作判定：别名承载的是名字
  而非形状，而校验器不持有锚点表。— (details: quality-ledger (bm))

- **一个 `[*]` 只指代一个元素，而非整棵子树。** — `$.rows[*]` 过去也会匹配 `$.rows[0].a`，因为匹配器
  只比较前缀与后缀，并要求中间存在某些字符。在规则忽视形状时这一点不可见；一旦规则开始断言形状，被
  命中元素之下的每个嵌套值都成了虚假违规。`$.rows[*].a` 仍可抵达它指名的成员。— (details:
  quality-ledger (bm))

- **一条校验规则只承载一项检查。** — 同时书写 `type` 与 `mapping_of` 的规则只运行最后写入的检查，对
  前者只字不提，因为解析器的每个分支都赋值给同一字段。一条规则出现两项检查现会使 schema 解析失败；
  `required` 与检查正交，仍可与任一项组合。— (details: quality-ledger (bm))

- **严格 JSON 写出遇到非有限浮点数时改为拒绝，而不是换个写法。** — RFC 8259 没有无穷与 NaN 的字面
  量，`to_json` / `to_jsonc` 过去的回答是把中枢文本加引号：`{"b": ".inf"}`，而 `load_json` 会把它当
  字符串交回去。数字在输出时变成了字符串，与一份本就写着该文本的文档无法区分——这与 #312 修的是同一类
  失真，只是方向相反。其余候选是量出来的而非拍板的：`null`（`JSON.stringify` 的答案）是另一个数，裸
  `Infinity` 则是本库严格读取器有意拒绝的文本（此处我们比 CPython 的 `json.loads` 更严）。现在写出会
  抛 `YamlSerializeError`，消息就是稳定原因键 `json-cannot-represent-non-finite`；拥有该词法的
  `to_json5` 仍是无损往返。拒绝也不再伪装成内部故障：新增 `SerializeError::UnsupportedValue` 专门表
  示「该格式装不下这个值」，而键、别名、null 与时间戳此前都被报成 `internal-error`。

- **`pyq validate` 接受 `--input`，并按真正读到的格式处理。** — 它原先硬编码 YAML 解析器，指向
  `pyproject.toml`、`package.json` 这类配置时一律拒绝。现在接受
  `--input auto|yaml|json|jsonc|json5|toml`，并走共享加载器。

- **PR 层 fuzzing 改为阻塞，崩溃无法被合并过去。** — 步骤级的 `continue-on-error` 只是暂时棘轮：
  `main` 上还留着这一层正确标记出的漂移。针对当前树重新验证：四个目标干净重放已提交种子语料
  （5/59/6/5 个种子，nightly-2026-08-15，cargo-fuzz 0.13.2）。 — (details: quality-ledger (aa))

- **文档工具链升到当前稳定版，存根生成器刻意按住不动。** — `zensical` 0.0.56 → 0.0.69、
  `mkdocstrings-python` 1.11.1 → 2.0.9（连带 `mkdocstrings` 1.0.6、`griffelib` 2.2.0，两者都要求
  Python 3.11），而 `maturin` 保持 1.14.1 —— 已提交的 `.pyi` 与漂移检查的调和规则描述的就是它的输
  出。配置里的 14 个 handler 选项逐个对照已安装的 `PythonOptions` 校验过；未被识别的选项构建时静默忽
  略，所以只能由测试检出。— (details: quality-ledger (ba))

- **API 参考现在由库自身的 docstring 与类型存根生成。** —
  `docs/<locale>/api/{reference,yaml-document,yaml-instance,node,merged-view}.md` 都加上了
  `mkdocstrings` 指令，站点上的签名出自代码而不是手抄。四个语言均已构建并核验：`YamlDocument` 下 23
  个签名、`Node` 下 41 个、模块页 48 个，而未写指令的那页仍为 0。`exceptions.md` 保持手写 —— 生成的
  类型存根只声明了四个类，不含任何异常类型。— (details: quality-ledger (be))

#### 修复

- **内联 dict schema 不再丢掉序列化器不认识的那部分。** — `safe_load(schema={...})` 与
  `YAML(schema={...})` 靠 `_schema_to_yaml` 把 Python dict 转成引擎解析的 schema 文本。它逐行拼接文
  本，只认 `extends` 与 `rules`，所以带了 `validate` 段的 dict 会被静默删掉那些规则注册进去——而无规
  则的 schema 接受一切文档，于是没有任何东西报告此事。它那唯一一条手写的引号规则（值含撇号就用双引
  号）对同时含两种引号的正则会产生内部未转义的双引号标量：
  `invalid trailing content after double-quoted scalar`。现在整个 dict 交给 `from_dict` 写——就是其他
  地方用来保障往返保真的那个序列化器——于是每个键都能活下来，任意模式文本也能按原样被正确引用。—
  (details: quality-ledger (bp))

- **两个公开函数不接受自家桩声称它们接受的关键词。** — `pyrs_yaml.read_markdown(path="note.md")` 抛
  `TypeError`，而已提交的桩把该参数声明为 `path`：`__init__.py` 里的 Python 包装层把它改成了
  `content`，而这个函数打开的是它收到的文件；于是名字引导出的调用 `read_markdown(content=text)` 反而
  在文件系统上失败。现在包装层叫 `path`，`read_markdown_str` 保留 `content`，两者均写下区别。同一种
  漂移反方向发生在 `validate_against_schema`：桩里叫 `schema_yaml`，而调用方一直用 `schema`；现在原
  生参数已重命名，桩沿声明路由重新生成。— (details: quality-ledger (bo))

- **三个语言的 API 页面写出了库中不曾存在的参数。** — zh/ja/ko 参考页写了
  `register_schema(name, schema: str | dict)` 与 `register_type(tag, type_handler, priority)`，而两
  层声明都是 `register_schema(name, schema_yaml: str)` 与 `register_type(name, handler)`——而且
  `priority=` 会招来 `TypeError`。英文页是对的，所以这是翻译阶段的偏差而非陈旧副本：页面现在写对象接
  受的东西，而门禁会拒绝任何语言里再出现一次。— (details: quality-ledger (bo))

- **带位置的校验错误仍然说出它匹配的 `path`。** — `SchemaValidationError` 要么打印
  `line:column: message`，要么打印 `path: message`，因此节点一旦带有源码区间，schema 作者就失去了自
  己书写的那一半：`1:9: expected map but got sequence` 从不说明是哪个键。现在两半都打印。— (details:
  quality-ledger (bm))

- **映射键终于与同样的文本作值时同义。** — `1: a` 读作 `{"1": "a"}` 而 `a: 1` 读作 `{"a": 1}`，
  `~: 1` 读作 `{"~": 1}` 而 `a: ~` 给 `{"a": None}`：同一份文档因标量位于 `:` 的哪一侧而有两种含义，
  以整数、布尔或 null 作键的配置根本无法被查到位。现在键走与值相同的 schema 路径，与 PyYAML、ruamel
  一致；`load_toml` 会把任一 YAML schema 会重新定性的键加引号。— (details: quality-ledger (as),
  (at))

- **锚点名文法与读取器对齐，收掉一整族漂移。** — 写手可能发出读取器拒绝再摄入的锚点名
  （`found unknown anchor`），破坏「绝不发出不可解析输出」的契约：以 `:` 结尾的名字每轮掉一个字符，
  带引号的名字吞掉换行，裸扫描器还会从注释文本与重叠的 `&` 里凭空造出锚点。现在按 granit 的方式读，
  即锚点字符的一个极大连续串。 — (details: boundaries — Fuzz findings, (x), (y), (z))

- **注释在标记、标签与容器之间守得住自己的归属。** — 一族「首次发出与再读之间注释换主人或消失」的位
  置：只渲染出标签的值（`k: !`）把紧随其后的注释行据为己有；容器自己的行内注释落在无法承载它的行上；
  引号标量里的 `#` 因为裸字节扫描而挤掉注释；紧凑的 `- key:` 分支只复制文本不复制注释槽位；而前导注
  释的单个 *槽位* 只保留一叠注释的最后一条 —— `# alpha` + `# beta` + `key: 1` 回来时只剩 `# beta`。
  前导注释现在是列表，所有写手都尊重它。— (details: quality-ledger (h), (i), (m), (n), (p), (t),
  (u), (v), (ad))

- **合并的身份与深度问题修复，不再爆原生栈。** — `<<: &b` 是字面的 null 值合并键，却与下方真正的
  `<<:` 折叠在一起，折叠时搬走了被丢弃条目的注释却没搬它携带的锚点；带注释的非标签 `y` 与合并来的
  `y` 是两个不同的 `IndexMap` 键，于是 `to_yaml` 在同一层打出两次 `y:`；嵌套或自引用的合并链在环守卫
  已弹出之后喂给解析器的尾递归，深度无上界地爆栈（58 字节的 libFuzzer 发现）；而带注释的 `<<` 键节点
  对该遍完全隐形。— (details: quality-ledger (o), (q), (r), (s), (y))

- **块标量一次发出即收敛，指示符也被正确识别。** — `>+8\r\r#` 读作值为一个换行、`Keep` 且显式缩进 8
  的折叠标量；写手保留指示符发出 `>+8\n\n`，它再读变成 `Clip` 且无指示符，于是*第二轮*发出 `>\n\n`，
  其值是 `""` —— Clip 会剥掉尾部换行，值就此衰减为空，而每一轮看起来都很「稳定」。折叠换行串、
  `4RWC` 式显式缩进与首行为空的字面块由同一批工作在再解析下闭合。 — (details: quality-ledger (aa),
  (ab), (r))

- **Unicode 空白、BOM 与标签后缀不再损坏文档。** — 方言 fuzzing 触及的空白集、BOM 位置与标签后缀边界
  改按读取器自身的文法处理，每一处都由已提交种子与确定性 Rust 回归测试钉住。
  — (details: boundaries — Fuzz findings, Blank set)

- **JSON 与 JSONC 注释扫描器不再在字符中间 panic。** — 行注释与未闭合块注释扫描把 `pos` 每次只推进一
  个 *字节*，于是一个尾随的多字节字符（U+FEFF，约 25 秒 fuzzing 就命中）会让 `pos` 停在它内部，下一
  次 `&text[pos..]` 切片以「not a char boundary」崩溃。注释现在按完整码位推进，畸形输入干净报错。—
  (details: boundaries — Fuzz findings)

- **重复按键的值判定拒绝，且只有 null 键折叠。** — 比较按值进行，所以 `{a: 1, a: 2}` 仍抛
  `YamlDuplicateKeyError`，而空键与 null 键（`: a` + `: b`、`~: a` + `~: b`）按 yaml-test-suite
  `2JQS` 的要求折叠。带注释的键也不再无法寻址：`CustomNode::hash` 折叠了归一化后的注释视图，而 `eq`
  比较原始槽位，于是 `doc["key"]`、`in` 与合并展开都会对文档明明持有的键回答「无此键」。— (details:
  quality-ledger (ae))

- **空容器内联发出，不再需要第二轮。** — `safe_dump({"a": {}})` 产生 `"a:\n  {}\n"`，而 AST 写手产生
  `"a: {}\n"`；序列项下的 `- \n  {}` 再读为 flow 节点，还要再移动一次。两种文本都读回相同数据，这就
  是往返测试看不见它的原因：fuzz 层断言的不变量是*文本*本身为不动点。— (details: quality-ledger
  (ab))

- **`pyrs-toml` 重新能为裸机构建；内联表注释留在原位。** — 叠注工作把 `std::mem::take` 放进了
  `#![no_std]` crate，任何宿主构建都放过它、而 `no-std-check` 不会；六处改用 `core::mem::take`。另一
  处是多行内联表非末位成员上的注释被发在分隔逗号之后 —— TOML 无法在注释里保留逗号 —— 于是读取器把它
  改归下一个键的前导注释，文本永远不收敛。— (details: boundaries — Known Engine Boundaries)

- **Linux 自由线程 wheel 出货，`pyq` 也产出其构建物。** — wheel 矩阵只为 Windows 与 macOS 构建
  free-threaded 产物，用无 GIL 解释器的 Linux 用户无从安装：开 GIL 的 `cp38-abi3` wheel 与
  `Py_GIL_DISABLED` 构建 ABI 不兼容，而 `abi3t` 从 3.15 才开始。跨架构的 `pyq` 步骤还会注册 qemu
  binfmt 并在 manylinux 镜像内自校验 —— 宿主机只有翻译器、没有外来 loader，直接执行 aarch64/armv7 二
  进制会在 `main` 之前死掉。— (details: quality-ledger (aq))

- **门禁仪器自身的六处缺陷修复，含 MSRV 那条腿。** — 三个检查器无法在 Python 3.8 导入（导入期求值的
  注解、`str.removeprefix`）；场景探针只点名一个 harness 文件，于是第二条通道无从把门；Ir harness 的
  crate 集合停在 `[dependencies]`，描述了它只读了一半的图；一次基线刷新用 rustc 1.99.0 去对照 1.97.1
  的记录，把 `serialize_medium` 移动了 +6.7%；汇聚检查只等 11 个作业中的 3 个；而属性层跑的是
  proptest 默认的 256 个用例，因为没有任何 workflow 设置 `PROPTEST_CASES`。— (details:
  quality-ledger (ah), (an), (ao), (ap), (aq), (ar))

- **条目再也不能被放进没人看的位置。** — 某次提交给五镜像都加了发布说明，却把它放在 `CHANGELOG.md`
  前言之上、en 与 zh 的 `tags:` 列表之内、ja 与 ko 的 front matter 与首个标题之间 —— 在每个文件里都
  位于更新日志正文之外，而镜像检查器仍然绿，因为「版本表头集合相同」完全不说明读者会去哪里看。
  `placement_errors()` 现在管位置。 — (details: quality-ledger (ar))

- **站点更新日志页恢复自身描述，折叠的历史版本也能正常渲染。** — 宽度修复器只跳过了页面 YAML front
  matter 的起始 `---`，于是四个语言页的 `title:`、`description:`、`tags:` 被重排到一行；已部署的生成
  器照样发布，只是把页面描述换成站点描述，而更新版本直接让构建失败。折叠版本还以裸 `<details>` 开
  头， Python-Markdown 对其内容不解析 —— 在部署版本上实测：11 个标题、3 处字面 `####` 泄漏，对比带属
  性时的 69 个标题、0 泄漏。`scripts/check_doc_metadata.py` 同时把住这两关。— (details:
  quality-ledger (az))

- **随 wheel 发布的类型存根不再是非法 Python。** — maturin 1.14.1 把运行时的 `__doc__` 原样写进三引
  号，而本项目有两处 Rust 文档注释含反斜杠，于是 `python/pyrs_yaml/pyrs_yaml.pyi` 以截断的 unicode
  转义通不过 `ast.parse`。这个文件配着 `py.typed` 就是用户的 mypy 与 pyright 所读的类型契约，任何文
  档生成器也无法经它到达 API。漂移路由现在会转义 docstring 内的反斜杠（站点数作 tripwire），并要求派
  生文本可解析。— (details: quality-ledger (bb))

- **发布说明开始描述整个版本，而不只是它的第一行。** — 折叠历史里每个 `<summary>` 都是该版本的第一
  条，于是读者在决定要不要展开时看到的是九十三分之一（v0.16.0：64 条 Added、14 条 Fixed、8 条
  Performance）。现在每条都概括整个版本；`0.11.4`、`0.11.3`、`0.1.0` 这三个当初没被折叠的也一并折
  上，五镜像同步。— (details: quality-ledger (bc))

- **纯文档的 PR 此前合不进去，也从来没有 PR 构建过站点。** — 分支保护只要求一个检查
  `Test matrix (all legs)`，而产出它的 workflow 恰恰把 `*.md` 与 `docs/**` 从触发条件里排除了。
  PR #319 所有能产出的检查看着都绿，合并仍被 `Required status check … is expected` 拒绝；管理员强制
  开启，连 `--admin` 也走不通。现在 `ci.yml` 对每个 PR 都触发，`changes` 作业给变更集分类、重活按该
  结论跳过，而 `docs-gates` 每次都跑文档门禁并对四个语言站点做 `--strict` 构建 —— 登记过的渲染盲区
  正是这样关闭的。 — (details: quality-ledger (bd))

- **随每个 wheel 发布的类型契约居然一个异常类型都没声明。** — 本包导出十个错误类型，而
  `python/pyrs_yaml/pyrs_yaml.pyi` 一个都没写：maturin 1.14.1 的自省路由不会遍历 PyO3
  `import_exception!` 类。于是 `except pyrs_yaml.YamlParseError:` 对 mypy 与 pyright 隐形。漂移路由
  现在会从已构建的类派生这些声明（名、基类、docstring），用 `EXPECTED_EXCEPTION_CLASSES` 计数把关，
  把基类排在子类之前以保证文件可解析，并在无法导入时退出码 2 而不是交出一个缺失声明的契约。因此
  `exceptions.md` 也已在四个语言全部改为生成。— (details: quality-ledger (bf))

- **桩能解析，却依然通不过类型检查。** — mypy 在 `python/pyrs_yaml/pyrs_yaml.pyi` **内部**报出五处错
  误：`Name "u32" is not defined`、`Name "Callable" is not defined`，以及三处 `Py<PyAny>` 的
  `Invalid type comment or annotation`。这些诊断在用户编辑器里归属于本库，而所有 AST 级门禁都是绿
  的。根因是 maturin 1.14.1 把 Rust 拼写抄进注解且从不导入 `Callable`；如今路由会重写这些拼写（计数
  4 处）、把注解引用的名字并入 `typing` 导入，并由 `scripts/check_stub_types.py` 在 CI 同时用 mypy
  与 `ty` 检查产物。`ty` 还指出我们自己保真模板写出的 `-> dict`，现已改为 `dict[Any, Any]`；它余下
  32 条严格性诊断按记录处理，暂不强制。这道门禁随后让每条 CI 测试腿都变红，缺的正是它要调用的
  工具：可执行文件不存在时抛出的是异常而非返回码，于是无法检查如今回答退出码 2 并跳过测试，绝
  不是一次通过。这道门禁自己的反向对照随后也失效了：它断言的是 `origin/main`，而这个修复合并的那天
  main 上的桩已经是干净的了。于是对照改为点名不可移动的引用——携带该缺陷的提交，
  以 `v0.17.0` 作为回退，先检查文件是否含有应有的拼写，再在两个检查器都存在的 CI 上运行。 —
  (details: quality-ledger (bg))

- **排版修复器不再写出自家检查器会报告的空隙。** — 新增一条日文条目后，`check_doc_wrapping.py --fix`
  回答 `1 gap(s) [wrap residue]` 且无法收敛：修复器依据字符类别决定两行物理行之间的连接符，而该类别
  不含 U+3001，于是以 `、` 结尾的行与后面的字符之间被加了一个空格；同一文件里的残渣字符类却包含这个
  码位，并把这样的空隙判为产物缺陷。同一条规则写了两遍，编码却不一样——每一轮修复都在重造它自己报告的
  问题。现在 `join_lines` 直接询问那两个残渣模式；新增的两个测试各自守住一半：`、` 边界会粘连，代码
  跨度前的空格照常保留。— (details: quality-ledger (bi))

- **JSON5 非有限浮点数经中枢往返后，两个方向都保住了类型。** — `from_json5` 交回的是中枢文档，而这段
  文本的两端对同样三个词的理解不一致：YAML Core 把 `.inf` / `-.inf` / `.nan` 解析为浮点数，把裸词
  `Infinity` 解析为字符串，于是照抄来源拼写就把数字变成字符串，凡重新读取该投影的使用方都拿到
  `'Infinity'`；`load_json5("{a: Infinity}")` 在内存里解析，结果是对的。先测量才看清报告只说了一半：
  加引号那一侧错在相反的方向，因为 `quoted_or_plain` 用 `needs_quotes`（一个 core schema 的提问）去
  判断解码后的 JSON 字符串要不要引号，于是 `"Infinity"` 被存成明文标量，`load_json5('["Infinity"]')`
  返回 `[inf]`，而 `load_jsonc`、`load_json` 与 `json.loads` 都返回 `['Infinity']`。同一份文档两种类
  型，取决于哪个加载器读了它。发射器现在从解析后的*值*导出该方言的裸词，中枢存 YAML 自身的拼写，
  JSON5 解析器接受喂给它的这些拼写。严格 JSON 与 JSONC 仍会给拼不出的非有限浮点数加引号：这条裁决留
  在 `ROADMAP.md` 里，不在 bug 修复里悄悄替用户决定。— (details: quality-ledger (bh))

- **含非 ASCII 键的规则路径会让进程中止，而 `$` 现在也能作为规则目标。** —
  `rule_path_to_segments` 每次只把游标推进一个字节，却整个推入解码出的字符，于是 `$.café` 停在 `é`
  的内部，下一次切片直接 panic："start byte index 4 is not a char boundary"。这是一条合法而非畸形的
  规则就会触发的 `PanicException`（`$.emoji😀key` 同理）。游标现在按解码字符的宽度前进。修好它之后
  第二半问题才显形：代码先要求有分隔符、之后才测试路径是否为空，所以裸 `$`（即整个文档）永远无法成为
  规则目标；而 `$x` 仍被判为不可解析，不会被误读成键名。`path: $` 配 `mapping_of` 可以校验根文档的
  值了。找出这个 panic 的那次排查还在 `ROADMAP.md` 留下一项待裁决：`mapping_of`/`sequence_of` 只查
  元素，不查容器本身的类型。 — (details: quality-ledger (bj))

- **模糊测试层此前没有针对 schema 语言的靶子，而缺陷正好住在那里（PR #329）。** — 六个靶子都只把文档
  喂给引擎，没有任何一个把 schema 喂给它。于是 `path: $.café` 能在所有测试层都看不见的地方把进程打掉
  （ledger (bj)）：手写的 schema 测试一律使用 ASCII 键，路径导航那段代码没有任何一层能够触及。第七个
  靶子以第一个 NUL 切分输入，把两半分别交给 `parse_schema_yaml` 与 `validate_node`，因此一条语料同时
  携带两种语法以及它们之间的相互作用。种子就是当初崩溃的那个输入，所以它进入拉取请求层（对种子做确定
  性的 `-runs=0` 重放），而不必只等每周采样。它能发现问题这一点是被验证过的，不是想当然：回退修复后
  重放会在 `schema_language.rs` 浮出一个崩溃输入，装上修复后同一种子干净重放，150 秒的探索同样干净。

#### 性能

- **标签发出改为表驱动，转义不再分配内存。** — 把标签编码挪到读取器自己的字符类上，就让逐字节的归属
  判定进了序列化热路径，于是它变成一张 128 项的编译期表（字母数字判定折进同一次查表），而 `%XX` 转义
  从十六进制字符表写出，不再用 `format!` —— 后者每个被转义字符都要新建一个 `String`。
  — (details: perf — Leaderboard & Performance Status)

- **大量 null 键的文档解析时间降为线性。** — null 键折叠原先每见一个 null 键就重扫整个映射：在全
  null 文档上看不见（折叠项就在 0 号槽），在真正要紧的形状上是二次的 —— 2k 个不同键后接 2k 个 null
  键，输入放大 4 倍耗时增长 12.4 倍（8k + 8k 时 99 ms）。映射现在记住 null 键的槽位，扫描只作为正确
  性兜底。 — (details: quality-ledger (ae))

- **10% 以下的问题由已提交的指令基线来裁定。** — 两个 runner 作业测出的 `serialize_block_scalars` 相
  差 1.44%，而同一二进制自身重复在 ±0.0005% 内 —— 低于 runner 噪声的墙钟时间说明不了任何事。基线现在
  记录产生它的环境，`scripts/ir_gate.py` 在来源不同时报注一句。— (details: quality-ledger (am),
  (as))

### [v0.17.0] — 2026-10-01

<details markdown="1">
<summary>pyq 对齐参数 · JSONC/JSON5 输入 · diff/merge · Release 自动化</summary>

#### 新增

- **pyq CLI 对齐参数** — `pyq fmt` 新增 `--indent N`（块缩进，默认 2）、`--width N`（纯量软换行列
  宽，0 为关闭）、`--sort-keys`（序列化器级全文键排序）与 `-i/--inplace`（就地改写文件），暴露
  `pyrs-yaml-core::SerializeOptions` 全部旋钮并对齐 Python CLI 的 `fmt --indent`。`pyq to-json` 新增
  互斥的 `--jsonc` / `--json5` 方言输出，接入 `pyrs-json` 的注释保留与 JSON5 写法序列化器
  （`to_jsonc_text*`、`to_json5_text*`）。
- **pyq JSONC/JSON5 输入方言** — `Format` 枚举新增 `--input jsonc|json5`（自动识别同样支持 `.jsonc`
  / `.json5` 扩展名），经由 `pyrs-json` 原生方言解析器，注释与 JSON5 写法挂在 AST 上；配合
  `to-json --jsonc` 一条命令即可完成保留注释的 JSONC→JSONC 往返。`--all-docs` 对单文档方言报出稳定
  错误信息。
- **`pyq diff` / `pyq merge`** — 原生 CLI 新增语义文档对比与右侧优先的深度合并（yq `*+` 形态）。
  `diff` 遍历两棵 AST，比较解析后的值、结构与标签（注释/引号/排版永不出现），输出 `-`/`+`/`~` 路径
  行，相同退 0、有差异退 1。`merge` 递归叠加映射、追加倍列（`--replace-arrays` 整体替换），输出往返
  保留的 YAML；两命令均可通过 `--input`/扩展名识别读取任意支持的输入方言。

#### 变更

- **GitHub Release 改由 `publish.yml` 自动创建** — 过去每次发布后都需人工执行 `gh release create`，
  既多一个容易遗忘的步骤，也让已发布版本与 tag 多一个漂移点。现在 `release` job 在 `uv publish` 成功
  后自动执行 `gh release create`，复用同一个 `refs/tags/` 条件，自动生成 release notes 并附上构建出
  的 wheel —— notes 与产物均来自发布到 PyPI 的那个 tag。`workflow_dispatch` 运行行为不变（既不发布
  PyPI 也不建 Release），与原有行为一致。
- **`README.md` / `README.zh-CN.md` 补齐原生 `pyq` CLI 文档** — 两份 README
  在 Python CLI 小节旁新增 `pyq` 小节：从源码安装方式、三条实际可跑的示例，
  以及完整命令清单，并指向 pyq 指南获取细节。

#### 修复

- **`pyrs-json` 模块文档** — 旧文案仍声称注释读入即丢、不再回写；
  自 #122 起注释挂在 AST 注释槽位上，并由 JSONC/JSON5 序列化器还原。

</details>

### [v0.16.0] — 2026-10-01

<details markdown="1">
<summary>跨库 parity · `load_json` · TOML 1.1 与 toml-test · JSON5/JSONC · pyq · 性能</summary>

#### 新增

- **JSONC 块注释热点基准** — 目标 §测试覆盖 5 将「block-comment」列为必需热点样本；之前仅行内 `//`
  注释入基。新语料驱动 `test_load_jsonc_block_comments`：50 对 pair + header/footer，每 pair 一个独
  立 `/* item N */` 以及一个尾追 `value /* trailing */`，块扫描回归从此在 CodSpeed 上显形。
- **YAML 的 PyYAML + ruamel.yaml 跨库对拍** — 目标 §测试覆盖 3 将两库点名作为 oracle；之前仅在
  `test_benchmark_crosslib.py` 用于基准与特性支持 printout，从未做**正确性**断言。
  `tests/test_yaml_crosslib.py` 20 个规范文档 × 5 个对拍面 + 2 个文档化分歧（重复键严格、YAML 1.1 传
  统 bool schema 域）= 122 测试。可选依赖 `skipif` 自动降级。
- **`load_toml` tomlkit 跨库对拍** — 目标 §测试覆盖 3 点名 tomlkit 作为 oracle；之前 tomlkit 仅在
  benchmark 出现。`tests/test_toml_crosslib.py` 新增 24 个测试，覆盖 11 种规范构造的三方对齐（pyrs /
  tomlkit / tomllib），将 `>i64` 拒收钉为规范严格（TOML v1.0 §Integers： 64 位有符号），并断言
  `-2^63` 边界（PR #174 修复）。可选依赖， `skipif` 自动降级。
- **orjson 作为 `load_json` 的严格 JSON oracle** — 目标 §测试覆盖 3 要求与 orjson 逐位对比；之前
  orjson 仅用于基准。16 个规范文档断言逐字节对齐，12 个非规范形式（注释、尾逗号、单引号、裸
  `NaN`/`Infinity`/`-Infinity`、十六进制、前导 0、`+.5`、`5.`）两侧一同拒收。orjson 拒绝 stdlib
  `json.loads` 在 `allow_nan=True` 下默认的裸字面量，严格度上高于 stdlib，作为 RFC 8259 oracle 比
  json.loads 更接近标准。可选依赖，`skipif` 自动降级。
- **CLI ↔ Binding 对等 gate（`tests/test_cli_binding_parity.py`）** — Pillar 1「CLI 与 Python
  Binding 两端均需具备同等功能」从文档声明升级为可执行契约：CLI 注册命令需与 18 命令固定清单对齐
  （过滤 cyclopts 的 `--help`/`-h`/`--version` 伪命令）；每个 `to-X` / `from-X` 动词必须有对应的
  `YamlDocument.to_X` / `from_X` / `load_X`；断言 `load_*` 家族（json/jsonc/json5/toml）四兄弟齐
  全；编辑/validate/compliance 动词都映射到实时 Python API。两侧任何一者漂移现在都会破 CI。
- **`load_json` 属性测试 + CodSpeed 基准** — Hypothesis（`test_load_json_matches_stdlib_json` 与
  `test_load_json_matches_load_jsonc_on_strict_domain`）为每个生成的规范文档固定严格 loader 与
  `json.loads` 对齐，并断言两个 loader 在 strict 域逐字节等价；快速路径越权或回退漂移将作为属性失败
  暴露。三个 CodSpeed wall-time 基准（`test_load_json_large` / `_floats` / `_escapes`）镜像
  `load_jsonc` 样本，将严格 binding 层纳入回归追踪。
- **`load_json`（严格）——补齐 `load_*` 家族对称** — binding 已有 `load_jsonc` / `load_json5` /
  `load_toml`，唯独缺严格 RFC 8259 对应物。`pyrs_yaml.load_json(s)` 在规范输入上与 `json.loads` 逐位
  对齐，并对 JSONC/JSON5 扩展（`//`、`/* */`、尾逗号、单引号、裸 `Infinity`/`NaN`、`0x…`）抛出类型
  化 `YamlParseError`。快速路径与 `load_jsonc` 共用 `json_fast::try_load`（任何非规范字节即 bail，
  零语法越权风险）；被拒对象走 STRICT `from_json` AST 解析。这就此补完下方 CLI ↔ Binding 对等声明中
  的最后缺口：CLI 每种格式都有对应的 `load_*` 兄弟——支柱 1 完成。已从 `pyrs_yaml.__init__` 重新导出
  并入 `__all__`；`.pyi` 通过 `maturin generate-stubs` 重生成。
- **方言 writer 定点属性测试** — `fmt_pbt.rs` 模块注释一直承诺 writer 定点
  （对 writer 自身输出重解析后再序列化应逐位相同）却从未实现。四个 proptest
  现在为 JSON/JSONC/JSON5/TOML 兑现该承诺；唯一的输入过滤（`json_object_domain`）
  排除不同键拼出同一 JSON 名的手搭 AST——那超出 RFC 8259 对象域，本就无可无损
  往返的文本。门控立刻抓到三个真实的注释保真 defect（见下方修复）。
- **热点基准语料** — 七个 CodSpeed wall-time 基准瞄准历史上脆弱的序列化路径：
  YAML 块标量文档（全部六种头部写法 `|`、`|-`、`|+`、`>`、`>-`、`>+`）与注释密集
  文档、TOML 多行字符串/进制整数/下划线分隔/指数/日期时间，以及 JSON5 的特殊数字
  形式（十六进制、`+.1`、`5.`、`Infinity`、`NaN`、单引号、尾逗号）。语料位于
  `tests/data/yaml_samples.py`，基准位于 `tests/test_benchmark_api.py`。正是构建
  这套语料暴露了下面修复的嵌套块标量缩进 bug。
- **文本级重解析门控（`prop_output_always_parses`）** — Rust proptest 套件现在断言每个生成的 AST 序
  列化后都能被解析器重新接受。此前的 AST 对 AST round-trip 属性会静默跳过序列化文本无法重解析的形态
  （`try_roundtrip` 返回 `None`），使整类 defect 不可见；新门控首轮就抓到六个真实的 serializer bug
  （见下方修复条目），每个都已由针对性 Rust 单元测试和 Python 回归类
  （`tests/test_roundtrip_bugs.py` 的 `TestNestedBlockScalarIndent`）钉住。
- **toml-test 一致性测试框架** — `tests/test_toml_test_suite.py` 以与 `test_yaml_suite.py`
  运行 YAML 套件相同的方式运行官方 [toml-test](https://github.com/toml-lang/toml-test) 语料：
  未跟踪的本地工件、缺失时 `skipif`、实测下限阈值，并用类型标签适配器做解码比对。
- **TOML 时间类型正确解码** — 仅日期与仅时间的值现携带不同的 `!date`/`!time` 标签（日期时间仍用
  `!timestamp`），从而走 `date`/`time.fromisoformat` 而不再崩溃。toml-test 发现裸时间
  （`07:32:00`）、省略秒的时间（`13:37`）以及小写分隔符的日期时间（`1987-07-05t17:45:00z`）在有效
  TOML 上抛 `ValueError`；`!time` 现补齐省略的秒，`!timestamp` 规范小写 `t`/`z`。
- **TOML 控制字符严格性** — 基本、字面与多行字符串内现在拒绝原始 C0 控制码（NUL、FF、DLE、US 等）与
  DEL（U+007F），仅保留制表符（及多行形式中的换行）。 toml-test 的 `invalid/control` 语料暴露了 13
  份被误承受的文档；注释体与裸 CR 检查列为后续项。
- **TOML 数字字面量严格性** — 前导零十进制（`01`、`-01`）、给进位前缀整数加符号（`+0x1F`、`-0b101` —
  `signed-int` 只支持十进制），以及尾随/双下划线（`1_`、`1__0`）现均被拒绝。toml-test 的
  `invalid/integer` 与 `invalid/float` 暴露 23 份误承受（总数 71 -> 48）；此前“进位整数可带符号”属违
  反规范。
- **TOML 内联表键冲突严格性** — 内联表现拒绝与已定义路径相等、扩展或被其遮蔽的点号键
  （`{ a = 1, a.b = 2 }`、`{ a.b = 1, a.b.c = 2 }`）；同属相路径（`{ a.b = 1, a.c = 2 }`）仍合法。
  toml-test `invalid/inline-table` 的 duplicate-key/overwrite 暴露此类（误接受总数 48 -> 39）。
- **修复 TOML 非 ASCII 字符串崩溃** — 基本与多行基本字符串解析器按字节前进，在多字节
  输入（U+00A0 等）处可能切到字符中间而 panic；现改为按整字符消费。由 toml-test 发现；
  补齐 #153 对单行/JSON 路径的同类修复。
- **格式模糊测试 + 健壮性修复** — 新增 `proptest` 属性测试模糊测试 TOML/JSON/JSONC/JSON5
  的解析器与写入器（no-panic + 可重解析）。发现并修复：TOML 与 JSON 字符串解析器的
  中字符切片 panic；JSONC/JSON5 内联 `//` 注释未换行导致吞掉后续 `,`/`}` 而不可重解析。
- **YAML merge/别名属性模糊测试** — 新生成良构的锚点/别名/merge 键文档（单别名、别名序列、
  含内联映射的序列、内联映射合并，以及 #166 拒绝的标量/空合并源），并含自引用锚点与
  重复别名引用。此前所有属性测试均使用 `arb_custom_node()`，它只产生 `meta.anchor` 而从不
  产生 `Alias` 节点，别名解析与合并展开路径（正是 #163/#166 的结构类）从未在进程内被
  模糊测试。`prop_merge_alias_never_panics` 断言解析 + 合并求解不 panic 也不溢出原生栈，
  且可解析的树再次序列化与再次解析保持稳定。
- **CLI 格式对等性** — CLI 新增 `to-toml`/`from-toml`、`to-jsonc`/`from-jsonc`、
  `to-json5`/`from-json5`（镜像现有 `to-json`/`from-json`），使绑定层支持的所有格式
  均可从命令行使用。
- **JSON 字符串转义快路径（性能）** — `load_jsonc` 将八个简单的双字节转义内联解码，
  不再因转义将整个文档回退到 AST 路径；含转义的 JSON 走快路径（比 AST 路径快约 15 倍）。
  值与 `json.loads` 一致；`\u` 与非法转义仍走 AST 路径。
- **JSON 浮点数快路径（性能）** — `load_jsonc` 现将规范化浮点数（小数/指数）直接
  解析为 Python 对象，不再因浮点将整个文档回退到 AST 路径；值与 `json.loads` 完全一致
  （正确四舍五入的解析）。新增一个 bench 覆盖该分支以防回退。
- **JSON 字符串序列化提速（性能）** — 无需转义的字符串改为一次 `push_str` 整体拷贝，
  不再逐字符重编码 UTF-8；字符串密集的 `to_json` 快约 35%（41→27 ns/项），输出逐字节一致。
- **`YamlDocument.to_toml()`** — 文档现在直接从 AST 输出 TOML（镜像 `to_json`/`to_jsonc`/
  `to_json5`），不再需要 `to_toml(doc.to_yaml())` 的序列化-再解析往返。输出逐字节一致；
  写入器自身比 `tomli_w` 快约 4.3 倍。
- **`to_json` 原生序列化器（性能）** — `YamlDocument.to_json` 改用原生引擎，不再
  经 `to_dict()` + `json.dumps`。ASCII 逐字节一致，快约 10 倍（1200 项 ~1450µs→
  ~120µs，快过 `json.dumps`）。非 ASCII 改为原始 UTF-8（同 `to_jsonc`/`to_json5`）
  而非 `\uXXXX`；仍为合法 JSON。
- **JSON 对象键直写（性能）** — 写入器将映射键直接追入输出缓冲（不再每键
  `String` 分配）；紧凑 `to_json` 再快约 2 倍（~120µs → ~60µs），输出逐字节一致，
  序列化现测得榜内 #2（仅次于 orjson）。
- **JSON 载入快路径** — `load_jsonc` 将规范化严格 JSON 直接解析为 Python 对象，
  跳过 `CustomNode` AST（实测快约 5-6 倍，现已超过标准库 `json.loads`）。
  非规范输入（浮点、转义、注释、超大整数、尾逗号）回退到通用路径，值与报错不变。
- **TOML 多行字符串保真** — TOML 多行字符串现在被投影为 `ScalarStyle::Literal`
  YAML 块标量（从而在文本中枢往返中存活），`to_toml` 重新将其发出为 `"""` 块而
  不再是转义单行。值逐字节往返、输出幂等，单行字符串仍保持单行。复用现有
  `Literal` 表示，未改 AST 结构。
- **TOML 文档级注释保真** — `to_toml` 现在会输出根映射的前导注释，因此文档
  开头的独立 `# 注释` 能在 TOML → 中枢 → TOML 往返中存活而不再被丢弃（对应
  JSON 写入器的 `emit_root_leading`）。原生 TOML 解析与无注释文档不受影响。
- **JSON5 Unicode 标识符键** — 无引号对象键不再仅限 ASCII，现在接受完整的 Unicode `ID_Start` /
  `ID_Continue` 集合，因此 `from_json5` / `load_json5` 可解析 `{ é: 1, 名: 2, हिन्दी: 3 }`。基于
  `unicode-ident` 表（rustc 自身的词法分析器所用的 crate）实现，逐文字精确符合规范，含标识符中间的结
  合字符。仅在 JSON5 模式下启用，因此严格的 `from_json` 与 `from_jsonc` 仍要求为此类键加引号。
  `\uXXXX` 转义中的孤立 UTF-16 代理项仍被拒绝（Rust `String` 无法无损表示）。新增依赖
  `unicode-ident`。
- **JSON5 Unicode 结构空白** — `from_json5` / `load_json5` 现在将 JSON5 在
  RFC 8259 四个空白（制表符 / 空格 / LF / CR）之外新增的空白视为 token 间
  分隔符：垂直制表符、换页符、NBSP（U+00A0）、所有 Unicode `Zs` 空格分隔符、
  LS/PS 行终结符（U+2028 / U+2029）以及 ZWNBSP（U+FEFF）。基于 `std` 的
  `char::is_whitespace`（减去 JSON5 不归为白空的 U+0085 NEL）加上显式的
  U+FEFF 实现，无新增依赖。仅在 JSON5 模式下启用，因此严格的 `from_json` 与
  `from_jsonc` 仍照旧拒绝全部这些空白，行为逐字节不变。
- **JSON5 行连接与 `\'` 转义** — 双引号 JSON5 字符串现在接受两种转义形式：
  紧邻换行符的反斜杠（续行，会同时移除反斜杠与换行符），以及转义单引号
  （`\'` → `'`）。仅在 JSON5 模式下启用，因此严格的 `from_json` 与 `from_jsonc`
  仍照旧拒绝两者。与 #125 的单引号字符串处理保持一致，补齐 JSON5 字符串保真。
- **JSON5 字符串转义 `\v` 与 `\0`** — `from_json5` 现在接受垂直制表符（`\v`）与 NUL（`\0`），双引号
  与单引号字符串均可；严格 JSON / JSONC 仍拒绝。与 #120（数值）、#124（数值语义）一起补齐 JSON5 语
  法。
- **load_json5 的 JSON5 数值语义** — `load_json5` 现在把 JSON5 独有的数值形式（`0x1F`→31、`+7`→7、
  `5.`→5.0、`Infinity`/`NaN`）解析为真正的数字（新增 `Schema::Json5`），严格 JSON/JSONC 加载器不
  变，`to_json5_text` 仍按源文拼写输出。
- **JSON5/JSONC 开放到公开 API** — `pyrs_yaml.from_json5` / `load_json5`，以及
  `YamlDocument.to_jsonc()` / `to_json5()` （走原生引擎，注释与 JSON5 风格不丢）。同时修正一个可达性
  缺口：`from_jsonc` / `load_jsonc` 以前未从 `pyrs_yaml` 包重导出， `pyrs_yaml.from_jsonc(...)` 会报
  `AttributeError`；现已列入 `__all__`。`to_jsonc`/`to_json5` 新增 `emit_root_leading`，保留文档级
  独立注释。`test_benchmark_api.py` 补充了 JSON 家族基准。
- **JSON5 writer（`to_json5_text` / `to_json5_text_pretty`）** — 契约 B 第 2 步。把 AST 序列化回
  JSON5，还原解析器保留的单引号字符串与 `0x…`/`.5`/`+7`/`Infinity`/`NaN` 数字形式，并输出 `//` 注
  释。key 总是加引号（无损）。内部共用一个 `Mode`（Json/Jsonc/Json5）；严格与 JSONC 输出不变。
- **JSON5 数值形式解析** — `from_json5`（新增 `allow_json5_numbers` 轴）现在接受十六进制
  （`0xDECAF`）、前/后小数点（`.5`、`5.`）、前导 `+`（`+7`）、前导零（`07`）以及裸 `Infinity` /
  `NaN` / `-Infinity`，每项都保留原文本供后续 JSON5 writer 使用。STRICT / JSONC 仍默认关闭该轴，像
  以前一样拒绝。同时修正了 `from_jsonc` 陈旧的“注释被丢弃”文档（#112/#115 后注释已保真）。
- **TOML inline table 内部注释保真** — PR #119 捕捉 inline table 内部的 `# ...` 注释（成员上方独立行
  → leading，值同行后面 → trailing）并贯串 IR，使它们能往返而不丢失。无装饰的 inline table 保持紧凑
  单行形式；嵌套在数组内的有装饰项会提升为多行。同时修复了 #114 遗留 bug：`skip_all_blank` 将独立注
  释自身的终止换行误判为空行。
- **YAML receiver 将独立行注释写入 `decor.leading_comment`** — PR #117b 把最后一个引擎
  （granit-parser receiver）迁到 #114 / #115 确立的新槽。scalar / mapping / sequence 的独立行注释现
  在落在 `NodeMeta::decor.leading_comment`，不再写旧的 `comment(standalone = true)`。手建 fixture
  因 #117 的规范化保持相等；`CustomNode::remove_comment` 同时清空**两槽**，保证 Python
  `Node.remove_comment()` 在 YAML 文档上行为不变。
- **跨槽独立行注释规范化 + Python `Node.leading_comment`** — `NodeMeta::eq` / `Hash` 现在把“独立行注
  释”视为单一概念，无论它存于新的 `leading_comment` 槽（TOML / JSON 引擎使用）还是旧的
  `comment(standalone = true)` 槽（YAML receiver 与手建 fixture 仍在用）。setter / remover 对两槽原
  子操作，YAML serializer 读规范化视图，`to_yaml(toml_ast)` 不再丢头注。Python
  `Node.leading_comment` getter / setter / remover 镜像 `Node.comment`，TOML / JSONC 来文档的独立行
  注释首次对 Python 调用方可见。
- **TOML 1.1.0 语法** — `from_toml` 解析到 TOML v1.1.0（2025-12-18 发布）。四项新增：**(A1)** inline
  table 可跳行 + 允许尾逗号；**(A2)** 基本字符串中 `\xHH` 字节转义（0x00..=0xFF）；**(A3)** `\e` =
  U+001B；**(A4)** time / date-time 秒可选（`t = 14:15`、`dt = 2010-02-03 14:15`）。
  `TomlDialect::V1_0` 与 `from_toml_v1_0` 保留为严格 1.0.0 逃生舱；1.0.0 文档两种方言下解析结果一
  致。同时修复 `space_time_sep` 检测中的索引 off-by-one（导致无 `T` 分隔的 date-time 在 1.0 模式下也
  未能识别）。
- **JSON 双槽注释保真** — JSONC 解析器现在把独立行 `// ...` 注释写入 #114 引入的 `leading_comment`
  新槽，同行行尾 `// trailing` 仍留在 `comment`。对象成员与数组元素因此可同时拥有两个注释，这
  是 #112 单槽模型不能表达的形状。`to_jsonc_text_pretty` 优先读新槽，回退到 `comment` 十
  `standalone = true` 保持手建 fixture 兼容。严格 JSON (`to_json_text`) 行为不变，仍不写 `//`。
- **TOML 空行与双槽注释保真** — `NodeMeta` 新增 `leading_comment: Option<Comment>` 与
  `blank_before: bool`（均从结构化 `Hash` / `PartialEq` 中排除），让 section 头部或 AOT 元素同时携
  带上方的独立注释与 `]` 后的行尾注释，互不隐盖。`to_toml` 现在会重现源文中的空行分隔
  （`a = 1\n\nb = 2` 字节稳定往返）；文档开头第一对KV不写前导空行。手建 / YAML 来源的节点仍通过
  writer 的 fallback 读取保持兼容渲染。
- **JSON5 方言** — `pyrs_yaml_core::json::from_json5(text)` 与
  `from_json_with_options(text, JsonParseOptions)` 接受 JSON5 全部四个轴：尾逗号、单引号字符串、无
  引号标识符 key 以及行/块注释。每个轴可以单独开关；`STRICT`、`JSONC`、`JSON5` 常量作为预设提供。
- **JSONC/JSON5 绑定与 CLI** — `pyrs_yaml.from_jsonc(str)` 输出 YAML 文本；
  `pyrs_yaml.load_jsonc(str)` 直接返回 Python dict / list。 `pyq from-json` 新增 `--jsonc` 与
  `--json5` 旗标，`tsconfig.json` 与 `settings.json` 能直接进入 verb 流水线。
- **JSONC 注释保真** — `from_jsonc` 现在会把采到的 `// 行` 与 `/* 块 */` 注释挂到 AST 的
  `NodeMeta::comment`（独立部分在 key 节点，行尾部分在 value 节点），与 PR #109 建立的 TOML 模型对
  齐。配套的 `to_jsonc_text(node)` 与 `to_jsonc_text_pretty(node, indent)` 按原位置写回；块注释输出
  时均一为 `//`（AST 仅存主体文本）。严格 writer `to_json_text` / `to_json_text_pretty` 保持字节一
  致，使消费者可以选择性地启用保真。
- **pyq 多文档编辑** — `-A/--all-docs` 现覆盖全部编辑命令
  （set/delete/rename/move/append/insert/sort-keys）及 `to-json -A`（JSON 数组，与 Python 对等）。
  每个文档针对流中自己的文本段做 splice（`MultiDocEditor` + `DirtyUnit::shifted`）：未触碰的文档与
  所有 `---` 分隔行逐字节保持原样；路径未命中的文档跳过（与 Python try/skip 语义一致，全部未命中仍
  报错退出）；布局异常的文档单独回退，不连带邻居。
- **JSONC 解析** — `pyrs_yaml_core::json::from_jsonc(text)` 与
  `from_json_with_options(text, JsonParseOptions)` 接受任意空白位置的 `// 行` 与 `/* 块 */` 注释（即
  TypeScript `tsconfig.json` 与 VS Code `settings.json` 使用的方言）。注释仅剥离，不保留。尾逗号及其
  他 JSON5 独有形式仍拒绝，接受语言保持为 RFC 8259 的严格超集。`from_json` 默认行为不变（仍为严格模
  式）。
- **TOML 注释保真** — 解析器现在会采集独立注释（`# ...` 单独一行，位于键值对或 section 头部之上）与
  行尾注释（`key = value # ...` / `[name] # ...`），并通过 `NodeMeta::comment` 挂载到共享 AST（独立
  部分挂在 key 节点，行尾部分挂在 value 节点）。`to_toml(from_toml(src))` 按原位置重新写回，
  `pyq edit` 与 `YamlDocument.set()` 不再剥离 TOML 往返中的注释。空白行分隔仍采用 writer 默认样式
  （见设计文档）。
- **TOML 数字源码保真** — `to_toml(from_toml(src))` 现在保留十六进制 (`0xDEADBEEF`) 与八进制
  (`0o755`) 整数的源拼写，以及带指数的浮点 (`1e10`、`-3.14e-2`)。下划线分隔符、显式 `+` 号、带负号的
  radix 形式 (`-0x1F`) 与二进制 (`0b101`) 仍归一为十进制，因为 YAML Core schema 无法重新读取它们，从
  而保证共享 AST 与 YAML 流水线的互操作性。注释保真与 JSONC 支持按设计文档后续 PR 落地。
- **pyq 功能补齐** — CLI 追平 Python CLI 功能面：`rename`/`move`/`append`/`insert`
  splice 编辑、`validate`（解析检查，或用 `--schema rules.yaml` 按 schema 语言规则
  校验）、`frontmatter`（`--body-out` 分离正文），以及 `get`/`fmt`/`to-json` 的
  `-A/--all-docs` 多文档流。接线过程揪出核心引擎 bug：`move_path` 只返回目标
  INSERT 单元，splice 文本会残留被移动子树的副本（文档回退全重序列化时不可见）；
  现返回两个单元，bindings 经批量 splice 通道依次应用。
- **`pyq` 过滤动词** — 匹配流上的结构化 jq 风格后处理：`--select 'PATH OP LITERAL'`、
  `--sort-by PATH` / `--desc`、`--unique`、`--first` / `--last`、`--skip N` / `--take N`、
  `--join SEP`，在 `get` 与 `from-*` 上按固定管线 `select -> sort -> unique -> slice` 后接 `join` 作
  用。有意选择旗标而非表达式语言：谓词仅一次微语法解析（约 40 行），类型不匹配一律 `false`（与 jq 全
  序的已知差异，已入文档），启动保持瞬时。
- **`pyq completion`** — 输出 bash、zsh、fish、PowerShell 的 shell 补全脚本
  （`pyq completion bash > ...`），由 `clap_complete` 驱动（已批准添加到 CLI crate 的依赖；仅存在于
  `pyrs-yaml-cli` 二进制内，不影响 Python 分发）。
- **`pyq sort-keys`** — 对任意路径（`$` 为根）的映射键排序，可原地回写
  或输出到 stdout，与 `set`/`delete` 共用核心 plan/splice 引擎，补齐与
  Python CLI `sort-keys` 的对等性。
- **命令行工具** — 新增 `pyrs-yaml` 命令（通过 `pip install "pyrs-yaml[cli]"` 安装，需 Python
  3.10+），在终端中暴露库的核心能力：`fmt`（往返重新格式化，保留注释/ 锚点/顺序）、`get`（JSONPath
  查询，支持 `--format yaml|json|text`）、`set` / `delete` / `rename`（基于路径的编辑，支持
  `--inplace`、`--string`、`--create-missing`）、`validate`（对 CI 友好的退出码）以及 `to-json` /
  `from-json` 转换。所有命令通过 `-` 读取 stdin，默认输出到 stdout。实现为纯 Python
  （`python/pyrs_yaml/cli/`），基于 [Cyclopts](https://github.com/BrianPugh/cyclopts) 作为可选
  extra，基础安装保持零额外依赖并继续支持 Python 3.8。基础安装保持零额外依赖并继续支持 Python 3.8。
- **CLI 扩展** —— 新增 `sort-keys`（对路径处映射键排序）、`move`（将子树移动到已存在的目标）、
  `frontmatter`（提取 Markdown front matter 为 YAML，可选拆分正文）与 `compliance`
  （YAML Test Suite 报告，支持 `--json`）命令；为 `fmt`/`get`/`set`/`delete`/`rename`/
  `sort-keys`/`validate`/`to-json` 提供 `-A/--all-docs` 多文档模式；`validate` 改为互斥的
  `--schema <名称>` 与 `--schema-file <路径>`。未成文的 `python -m pyrs_yaml.compliance`
  入口已被子命令取代并移除。
- **`YamlStream` 可导入** — `from pyrs_yaml import YamlStream` 现在与 API 文档和类型
  标记一致；此前该类仅能作为 `YAML().load_stream*()` 的返回值，从未从原生模块导出。
- **CLI `move --all-docs`** — `move` 现在支持 `-A/--all-docs`，在每个两端路径均可解析的
  文档上应用子树移动（与 `set`/`delete`/`rename` 语义一致），多文档标志覆盖全部编辑命令。
- **文档 ↔ API 一致性守卫** — `tests/test_docs_api.py` 扫描所有语系文档页中的
  `pyrs_yaml.…` 属性链、`import pyrs_yaml…` 与 `from pyrs_yaml … import …` 声明，
  任一引用符号在运行时不存在即失败（约 965 条声明纳入检查）。
- **可选第三方类型插件** — `!duration`（`pendulum.Duration`）、`!arrow`（`arrow.Arrow`）、
  `!ulid`（`ulid.ULID`）在安装对应库时自动注册（`python/pyrs_yaml/plugins/_builtin.py`
  中的 `_register_third_party`）。每个插件使用独立标签，不影响现有 `!timestamp` /
  `!date` / `!uuid` 处理器；标准库 `timedelta` 绝不会被 `!duration` 匹配。
- **pydantic-settings YAML 配置来源** — `PyrsYamlConfigSettingsSource`
  （`python/pyrs_yaml/settings.py`）是 `pydantic_settings.YamlConfigSettingsSource` 的即插即用替代，
  使用 pyrs-yaml（YAML 1.2 核心 schema）而非 PyYAML 解析。采用惰性导出，`import pyrs_yaml` 不依赖
  pydantic-settings；通过 `pip install "pyrs-yaml[settings]"` 安装（Python 3.10+）。`dump_pydantic`
  与 `parse_as` 也已改为相同的模块级 `__getattr__` 惰性导出模式。
- **`pyq` — Rust 原生 CLI crate** — `crates/pyrs-yaml-cli`（workspace 成员，
  基于 clap）将 `pyrs-yaml-core` 直接接入 jq/yq 风格命令行，运行时无需
  Python：`fmt`（保留注释的往返格式化）、`get <path>`（JSONPath-lite，支持
  `--json`/`--raw`）、`set <path> <value>` 与 `delete <path>`（yq 风格编辑，
  支持 `--create-missing` 与 `-i/--inplace` 文件回写，输出经往返序列化器，
  保留注释与值自身风格）、`to-json`（保序）、`to-toml`，以及导入命令 `from-json` /
  `from-toml` / `from-ini`；输入格式按扩展名识别（`--input` 可覆盖），`-` 或
  省略时读 stdin，失败时以 core 的稳定错误文本非零退出。
- **TOML 与 INI 交换格式** — 轮毂-辐射式多格式支持，YAML 仍是唯一可编辑表示：`from_toml`/`to_toml`
  在 TOML 文本 ⇄ YAML 文本间转换（Rust `toml_edit`）；`load_toml` 将 TOML 直接读为 Python 值
  （datetime 经内建 `!timestamp` 插件；TOML 字符串不会被重新解析）；`load_ini` 经标准库 configparser
  读取 INI（严格模式，只读）。TOML 输出对不可表达结构报稳定错误；往返编辑按设计仅 YAML 支持。

#### 变更

- **granit-parser 1.1 → 1.3** — 将 YAML 事件解析器从 1.1.0 升级到 1.3.0。这是 1.x 版本线内语义化版本
  兼容的次级升级：1.2.0 为特殊文档的限制新增了可选的 `Options` 字段，1.2.1 按 YAML 规范收紧了若干解
  析结果，1.3.0 为 `Input` trait 新增了两个带默认实现的方法（`fetch_block_scalar_line` 与
  `take_quoted_scalar_ascii_chunk`），让扫描器更快地跳过块级与引号标量字节。本项目通过
  `Parser::new_from_str` 使用解析器，只实现 `EventReceiver` / `SpannedEventReceiver`，从不实现
  `Input`，因此无需改动源码——新增的 trait 方法解析到其默认实现。全套测试通过：
  `cargo nextest run --all`（359）、`pytest`（1436 + 43 numpy）、纯 Rust `--no-default-features` 构
  建，且 YAML 测试套件合规门控保持不变。
- **原生 JSON 与 TOML 内核** — `serde_json` 和 `toml_edit` 依赖已全部移除。
  `pyrs-yaml-core` 自带 RFC 8259 JSON 引擎（字节级扫描、数字保留原文，因此
  `from_json → to_json` 字节稳定且不会丢失大整数/浮点精度，行/列错误带类型，
  严格拒绝尾逗号、前导零、孤立代理对、未转义控制字符、多根文档）以及覆盖
  完整语法的 TOML 1.0 引擎 — bare/quoted/dotted 键，basic/literal/多行字符串，
  十进制/十六进制/八进制/二进制整数允许下划线分隔，浮点支持 `inf`/`nan`/指数，
  以及 offset/local 日期、时间与日期时间 — 所有拒绝都以 granit-parser 风格的
  `ParseError::Syntax` 携 0-indexed `line`/`col` 上报。公开 API 保持不变，
  回环测试与 `tests/test_toml.py` 在新引擎上全绿。
- **内部重复代码清理** — 基准 fixture 改由共享块拼接，PyO3 路径编辑方法委托给现有
  `apply_metadata_edit` 助手，重复的文件读取/错误映射与行偏移样板收敛为共享函数。
  公开行为无变化；jscpd 测量的重复代码率从 5.25% 降至 3.45%。
- **`YamlDocument.validate()` 缓存编译后的 validator** — schema（JSON 文本或 dict）首次校验成功后即
  缓存编译好的 `jsonschema` validator；后续调用跳过 schema 解析、meta-schema 检查与 validator 构建。
  dict schema 按对象身份键控并辅以深拷贝快照守卫：原地修改会在下次使用时通过 `==` 检出并透明重编
  译。缓存路径抛出 `exceptions.best_match(validator.iter_errors(instance))`，与
  `jsonschema.validate()` 语义完全一致。WSL 实测：`document_validate` −98%。
- **解析/序列化内核结构化去重** — mapping 与 sequence 渲染共享单一 `write_container_node` 骨架（输出
  字节级一致，`serialize_*` 中位数 −5~11%）；单/多文档解析入口共享同一 `load_ast` 错误契约；schema
  解析链共享 `bool_word`/`numeric_tail`，YAML 1.1 不再逐标量重复 core 的 null/bool 检查；锚点注册
  （`register_anchor`）与独立/行内注释分类（`is_standalone_placement`）在 AST 与流 receiver 间单源
  化。仓库重复代码率 3.38% → 2.60%。

#### 修复

- **`\u` / `\x` 转义后紧跟多字节字符时解析器 panic** — 定宽转义读取器按字节偏移切
  `&self.text[pos..pos+width]`；JSON `\u` 或 TOML `\xHH`/`\uXXXX`/`\UXXXX` 后跟多字节字符时切片落在
  字符中间而 abort（#153 非 ASCII 切片崩溃的同族）。现改为字节切片 + UTF-8 校验，畸形转义干净报错。
  由方言 fuzz 发现，两处解析器均有确定性 Rust 回归测试固定。
- **字面 `<<` 键（非 merge 值）被静默丢弃** — `load(safe_dump({"<<": None}))` 返回 `{}` 丢了键。
  merge 解析器把任意 `<<` 都当 merge 消费，即使值是 Null/标量。按 YAML，`<<` 仅当值为映射别名/内联映
  射/ 其序列时才是 merge；Null/纯标量 `<<`，以及不含别名且无内容可合并的 `<<`（`<<: []`、
  `<<: [1, 2]`、`<<: {}`）现保留为普通键并往返保真。Alias/映射/序列路径（含 #166 自引用守卫）不变，
  yaml-test-suite 仍 405/406。由往返属性 fuzz 非确定性地暴露（正是 #163/#165/#166 类缺陷），并新增确
  定性 Rust 回归测试锁定。
- **TOML 深嵌套耗尽原生栈并 abort 进程** — TOML 解析器此前无嵌套预算（JSON 有 `DEFAULT_MAX_DEPTH`、
  YAML 有 `parse` `max_depth`），`parse_value` → `parse_array`/`parse_inline_table` 无界递归。深嵌
  套数组/内联表直接崩掉解释器（已验证：退出码 `0xC00000FD` STACK_OVERFLOW）——即 TOML 版的 #166 YAML
  merge 栈溢出。解析器现追踪 `depth`，超过 1000 返回类型化 `ParseError::MaxDepthExceeded`，与 JSON
  对称。由进程内 Python 边界测试 + subprocess 崩溃金丝雀 + 大栈 Rust 单元测试共同守护。
- **方言 writer/parser 丢失或错置文档级注释** — 三个定点属性抓到的 defect：(a) JSONC/JSON5 值前的文
  件首 `// note` 被误判为行内注释（空白扫描启发式中偏移 0 前无换行）并被首个对象成员占取，而非落在
  writer `emit_root_leading` 所标注的根容器上——空 `{}` 或根标量时彻底丢失；(b) JSON 家族与 TOML
  writer 逐字输出注释体而 parser 存的是 trim 后的文本，未 trim 的注释会在多轮 pass 间振荡尾随空白
  ——writer 现在输出时也 trim，首次拼写即稳定；(c) 纯注释 TOML 文档（`# note` 后无 key 消费）重解析
  时丢注释、空根序列化为 `""`——遗留的独立注释现在挂到空根表上。至此五格式的 leading 注释均达到逐位
  稳定的定点。由 `jsonc_file_leading_comment_stays_on_the_root`、
  `jsonc_comment_text_is_written_trimmed`、`comment_only_document_keeps_its_note_on_the_root` 钉
  住。
- **嵌套块标量的正文保持父行缩进** — 嵌套键下的字面/折叠标量把正文行按固定
  一级缩进从第 0 列输出，而不是落在 `b: |` 头部行下一层，导致所有嵌套块标量
  形态（键值对、序列项、紧凑 dash 映射、任意深度）序列化出的文本重解析为报错
  或错值。标量写入器现在贯穿 `block_base`（父行列位）参数；七种嵌套形态的
  round-trip 文本逐位忠实。由 TOML 热点基准经由共享 serializer 暴露。
- **serializer 只输出可重解析的 YAML** — 文本级门控抓到的五个拼写 defect：
  换行分支对子节点预输出 anchor/tag 而子节点（标量/null/flow 容器）自己也会
  输出，产生双头部（`A: !a` … `!a null`），现仅限 block 容器；flow 容器内的块
  标量（`[|`、`{k: >}`）及键位块标量降级为双引号；起新行的 flow 容器丢失行首
  缩进，复杂键（`?`）的值标记 `:` 落在第 0 列而关闭外层集合——两者现在都从
  父级缩进；带独立注释/tag 的复杂键产生歧义文本（注释现在移到 `?` 上方，键体
  整体下移一级独占行）；flow 集合内首尾带空白或嵌入 flow 指示符（`,` `[` `]`
  `{` `}`）的 plain 标量现在加引号——不加引号会截断 token 或在重解析时消失。
  另：带 tag 的空 block 容器把头部并入 `{}`/`[]` 行；紧凑 dash 项不再内联带独立
  注释的值。九个钉住的 Rust 测试加参数化 Python 回归守护每类缺陷。
- **TOML 拒绝合法的最小 i64 整数** — `from_toml`/`load_toml` 在 `-9223372036854775808`（`i64::MIN`）
  上失败：带符号路径先按无符号绝对值解析，取负号前就溢出。现在符号与数字一并解析（`i64::from_str` 向
  负方向累加），带符号浮点保留指数拼写，旧的取负通道已删除。由新的 Python 侧 Hypothesis 方言模糊测试
  （`tests/test_property_dialects.py`，以 stdlib `json`/`tomllib`/`pyjson5` 为预言机做类型严格相等比
  较；同时钉住两类 AST 歧义拼写——JSON5 裸 `Infinity`/`NaN` 字面量与超 i64 数字串）发现。Rust 回归：
  `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`。
- **错误缩进的流序列续行再次被拒绝** — 将 YAML 解析器升级到 granit-parser 1.3（见*变更*）后，开始静
  默地*接受*这样的输入：多行流集合的续行缩进不比其所在块键更深（yaml-test-suite `9C9N`：`flow: [a,`
  后接列 0 的 `b,`），使严格性从 `405/406` 回退到 `404/406`——由于 suite 的 ≥95% 阈值门，它对 CI 不可
  见，因而带着“绿”混过。现在 AST receiver 里一个解析后的 in-tree 守卫会跟踪所在块的缩进，并拒绝缩进
  不足的流续行，恢复 `405/406`。守卫只用解析器已算好的 span，因此正确缩进的多行流不受影响。`9C9N` 现
  被固化为逐例硬门（字面输入、无 `skipif`）写进 `tests/test_yaml_suite.py`，另加一个 Rust 单测
  （`parser::tests::flow_continuation_under_indented_is_rejected`）。
- **自引用合并键不再溢出原生栈** — 展开后指回自身锚点（`a: &a` 内含 `b: {<<: *a}`）的 `<<` 会在
  `resolve_merge_keys` 中无限展开，耗尽原生栈并拖垮整个解释器进程（Windows 退出码 `0xC00000FD`，即
  段错误）。循环 guard 此前只在*收集*合并对时生效，从不在*遍历*展开结果时生效，因此递归重入从未被拦
  截。现在锚点 guard 与别名展开一样按路径作用域：某锚点名在其展开被遍历期间始终留在递归路径上，解析
  回该路径上已存在的祖先的合并会终止为空展开而非递归。无环 AST 无法承载 PyYAML 的循环 dict，因此自
  引用合并现在收敛到 `{}` 而不再崩溃。同批修复 4 个相关合并语义缺陷：null/标量/序列合并源不再残留为
  字面 `<<` 键；直接作为合并值的内联映射（`<<: {x: 1}`）现在会被合并；合并序列中的非别名元素
  （`<<: [*a, {y: 2}]`）会保留其内联映射。由 6 个 Rust 与 9 个 Python 回归测试覆盖（`merge::tests`、
  `tests/test_gaps.py::TestSelfReferentialMerge166`）。由
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) 在 #166 报告。
- **NumPy 序列化不再在不持有 GIL 时读取 Python 内存** — ndarray 写入器曾通过 `unsafe { as_slice() }`
  借用数组数据缓冲区，并在 `py.detach` **内部**（即已释放 GIL 之后）遍历该借用切片。无论来源为何，
  `&[T]` 都是 `Send`，因此借用检查器无法拦截；但这块内存归 Python 所有，其他线程可并发 resize 或写
  入，构成不健全的数据竞态 / UB，仅在并发下暴露。现改为在**仍持有 GIL** 时把缓冲区快照为 Rust 自有
  内存（`slice.to_vec()`），仅标量→节点转换离线程执行。这是绑定层**唯一**一处 `unsafe` 缓冲区借用；
  其余全部 `py.detach` 站点已审查，仅触及 Rust 自有状态（AST、源文本、`BufWriter<File>`）。回归覆盖
  见 `tests/test_numpy.py::TestNumpyConcurrency`。由
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) 在 #165 中报告。
- **重复的别名引用不再解析为 `None`** — `to_dict()` 在一个**全局** visited 锚点集合
  后展开别名，且从不清理，导致任一锚点只有**第一次**引用产出值，之后全部静默降级
  为 `None`：
    ```yaml a: &x 1 b: *x      # 1 c: *x      # 原为 None，现为 1```
    影响面比“第二次引用”更宽：同一容器内的两个兄弟引用也会互相污染
    （`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}` 中 `b` 有值而 `c` 为 `None`）。现将该 guard 限定在当
    前递归路径上——仅在一次展开期间压入，随后弹出——因此重复引用与兄弟引用各自获得完整构建的值，而真
    正的环路仍会终止。`<<` 合并解析与 AST 本身经审查不受影响。`tests/test_direct_load.py` 新增 6 个
    与 PyYAML 对齐的用例锁定该行为，另有两个此前将错误输出当作预期的测试被重写。由
    [@bourumir-wyngs](https://github.com/bourumir-wyngs) 在 #163 中报告。
- **首层值为嵌套容器时文档头注释不再丢失** — 解析器曾为所有进行中的容器共用单一注释槽，嵌套容器的
  start 会提前抹掉尚未落位的 standalone 头注释（在 parse 层即丢弃；`to_dict` 不可见，`dump` 致命）。
  现改为每容器独立槽的栈式管理。
- **splice 编辑不再重复前导注释** — 重生成的区域文本携带 pair/item 自身的 standalone 注释时，被替换
  区域未覆盖旧注释行，两条注释并存；plan 现将区域回扩至注释行（`pyq set`/`delete` 与 bindings splice
  路径共享此修复）。
- **`!timestamp` 在所有受支持 Python 上接受结尾 `Z`** — `datetime.fromisoformat` 仅从 3.11 起识别
  UTC-`Z` 后缀；插件现将 `...Z` 归一为 `+00:00`，修复 3.8–3.10 上 YAML `!timestamp` 标量及
  `load_toml` / `from_toml` 引入的 TOML datetime 报 `Invalid isoformat string` 的问题。

#### 性能

- **事件流→Python 对象的直接物化** — `safe_load`、`safe_loads`、`YAML().safe_load*` 现在单次遍历
  granit 事件流直接构建 Python 对象，不再先建完整 AST 再在 `convert.rs` 中二次遍历；schema 解析、原
  文映射键与重复键报错语义完全一致。带锚点/tag/merge/多文档的输入经零成本预否决回退 AST 管线。WSL 实
  测：标量密集 `safe_load` −21~25%，家族整体 −13~18%，回退形态不变。
- **锚点提取字节门控** — `extract_anchors` 先做一次 `&` 字节包含检查，无锚点文档
  直接返回空，整体跳过逐字符引号状态机。Rust 侧 `parse_*` 基准中位数提升 11–18%，
  扫描本身从 1.5µs 降至 38ns。
- **流事件字典键驻留** — `parse_stream`/`load_stream` 每事件的固定键复用
  `pyo3::intern!` 常驻字符串，消除每键一次的 Python 字符串分配。WSL 实测：
  `parse_stream` −34%、`parse_stream_multidoc` −39%、`load_stream` −22%。
- **分解微基准** — 新增 `granit_events_*` 基准，将 granit 纯事件管道成本与 AST
  构建分离（仅基准）。
- **多文档解析免除逐文档深拷贝** — `on_document_end` 改为将完成文档的所有权移动进集合而非深拷贝（下
  一文档会重建 result，克隆是纯开销）。WSL 实测：`parse_all_docs` −9.7%、`safe_loads`（多文档）
  −9.5%、`YAML().safe_loads` −6.7%。
- **流式写入跨文档复用单一缓冲** — 新增 `direct_dump_into` 将每个文档写入复用的
  `String`，`dump_iterable` 在文本已以恰好一个换行结尾（正常情况）时跳过
  `normalize_doc` 的重新拷贝。WSL 实测：`dump_stream_multi_doc` −27.2%、
  `dump_stream` −4.4%。
- **AST 构建器标量快速路径** — `unescape_double_quoted` 对无背斜杠字符串提前返回；
  `detect_chomping` 改为惰性取行而非每个块标量收集全文。WSL 实测：`to_dict` 族
  −4~9%、标量类型加载 −3~4%，无回退。

#### 文档

- **修正 numpy 指南的 0-D 标量章节（全部语系）** — 原文声称 0-D 数组会“重塑为单元素
  列表”（`assert data == [42]`），而实际行为（由 `tests/test_numpy.py` 锁定）是序列化为
  裸标量（`assert data == 42`）；四语系文本已纠正，en 版新增 0-D `bool` → `1.0`
  的 rust-numpy 特性警告块。

</details>

### [v0.15.0] — 2026-08-19

<details markdown="1">
<summary>Node 元数据/样式 API · Schema 文件 IO · 深编辑 · cp314t 重开 numpy</summary>

#### 新增

- **Node 元数据 setter/getter** — 新增 `Node.comment` / `Node.anchor` / `Node.tag` 只读属性和
  `set_comment` / `set_anchor` / `set_tag`（及 `remove_*` 系列）。编辑别名或不存在路径会报错；内联标
  量值和序列项上的独立注释现在输出到独立的缩进行（修复 `child:\n  # c\n  val` 与 `- a\n# c\n- b` 既
  有的 round-trip 缺陷）。
- **Verbatim 标签** — `set_tag("!<tag:yaml.org,2002:str>")` 现在生成 verbatim 标签（空 handle），且
  从源码解析的 verbatim 标签在 round-trip 中保留：`Tag` 的 `Display` 对空 handle 标签以 `!<...>` 包
  裹输出，`parse_tag` 识别 `!<...>` 形式，流事件通过 `Display` 序列化标签。
- **Schema 文件 IO 与列表** — `load_schema(name, path)` 从文件读取 schema 定义并注册，
  `list_schemas()` 返回所有已注册的 schema 名称（内置 `failsafe`/`json`/`core`/`yaml1.1` + 自定
  义）。
- **Node style/format setter/getter** — 新增 `Node.scalar_style` / `Node.flow_style` /
  `Node.chomping` 只读属性和 `set_scalar_style` / `set_flow_style` / `set_chomping` 方法。
  ScalarStyle/Chomping 现在 derive `Copy`。非标量节点返回 `None` / no-op，别名和缺失路径报错。
- **Schema 结构化校验** — 在 schema 定义的 `validate` 段添加结构检查（路径限定标量类型、
  `sequence_of`/`mapping_of` 容器、`required`）。`validate_against_schema(data, schema_yaml)` 列出所
  有失败项并抛 `YamlValidateError`。
- **`Node.copy()`** — 将子树深度复制为与文档分离的独立 Python 值（dict/list/scalar），可用于通过
  `set_value()` 粘贴。
- **深度编辑 API** — `doc.set_many({path: value})` 在单次 splice 突发中设置多个路径（支持通配符
  `[*]` 和深度扫描 `..`）；`doc.sort_keys()` 原地排序映射键；`Node.move(new_path)` 移动子树；
  `Node.path` / `Node.find_first()` / `Node.value_eq()` 新增路径访问、首通配符查找、值比较。
- **0.14+ 新功能的属性测试** — `validate_node` / schema 解析 / style round-trip 的 Rust proptest，
  `set_many` 通配符 / metadata 编辑 / `sort_keys` 的 Python hypothesis 测试。`hypothesis` 移至
  `test` 组，确保 CI 运行属性测试。
- **序列化器修复** — 空 flow 容器（`key: {}` / `key: []`）上的独立注释不再产生无效 YAML（降级为行
  内）。

#### 变更

- **NumPy 在自由线程 (cp314t) wheel 上重新启用** — 从 cp314t 构建参数移除 `--no-default-features`；
  rust-numpy 0.29 支持自由线程 Python，`numpy.ndarray` 序列化现可在自由线程 wheel 上使用（运行时自动
  检测 NumPy 是否安装）。

#### 文档

- **修正全部语言（en/zh/ja/ko）文档中的过时引用** — `saphyr-parser` → `granit-parser`，YAML 合规率
  98.1% → 99.75%（405/406 套件用例），ABI3 支持 3.9–3.13 → 3.8–3.15（py3.9+ → py3.8+），并更新基准测
  试表为当前 CodSpeed CI 数据（解析快 21–43 倍、序列化快 55–177 倍于 PyYAML）。Rust 侧基准测试章节从
  Criterion 迁移到 divan（`benches/yaml_bench.rs` → `crates/pyrs-yaml/benches/yaml_bench.rs`）。

</details>

### [v0.14.1] — 2026-08-15

<details markdown="1">
<summary>引号与转义边界：单引号反斜杠、BOM、非字符、多字节换行</summary>

#### 修复

- **含反斜杠+控制字符/非字符的单引号标量** — 此类值改用双引号输出；单引号无法转义控制字符/非字符。
- **非字符与 BOM 引用** — `needs_quotes` / `needs_double_quoted` 现对 U+FFFE/U+FFFF/平面末尾非字符及
  U+FEFF（BOM）要求引用。
- **双引号转义宽度** — U+FFFF 以上的码点现以 8 位 `\Uxxxxxxxx` 形式转义（4 位 `\u` 仅限 BMP）。
- **折叠 plain 标量续行缩进** — 续行缩进改由值起始列推导，使嵌套序列/映射项续行缩进超过父块缩进。
- **多字节折叠边界** — `wrap_plain_scalar` 对折叠切片做 char boundary 向下取整，避免 4 字节 UTF-8 跨
  边界时 panic。
- **publish 测试依赖含 `hypothesis`** — `.ci/requirements-test.txt` 固定 `hypothesis>=6.113.0`，使发
  布工作流能运行属性测试。

#### Added

- **`scripts/fuzz_panics.py`** — 本地大规模 Hypothesis fuzz 脚本，含恶意策略覆盖 dump/parse/edit/幂
  等。

</details>

### [v0.14.0] — 2026-08-14

<details markdown="1">
<summary>YAML Schema 语言与可插拔解析 · 引号标量 · 空集合输出</summary>

#### Added

- **YAML Schema Language** — 定义自定义 schema，将正则模式映射到 YAML 类型。
  通过 `register_schema()` 注册。
- **内联 dict schema** — `schema` 参数可直接传入 `dict`。
- **Community Plugins** — 通过 `CustomType` 基类注册自定义节点类型。
  使用 `register_type()` 注册。
- **内置插件** — 默认注册 `!timestamp`（datetime）和 `!set`。

#### Changed

- **Schema 解析可插拔** — `SchemaResolver` trait + `Schema` 枚举 +
  全局 `SchemaRegistry`。内置 schema 保持零开销分发。
- **`node_to_pyobject` 和 `direct_dump` 检查 `CustomType`** —
  带标签的标量通过 `from_yaml()` 转换，Python 对象通过 `to_yaml()` 序列化。

#### 修复

- **带引号标量恒为字符串** — 隐式类型解析仅作用于纯标量（YAML 1.2）。`safe_load('"true"')` 返回字符
  串 `"true"`（而非 `True`）。序列化器保持文档（`to_yaml`）路径下的负数正确往返。
- **单引号/双引号单字符键可往返** — 值为单个 `'` 或 `"` 的映射键以引号标量输出，不再产生无法解析的
  YAML。
- **空集合输出 `{}`/`[]`** — 空映射/序列序列化后不再是解析为 `None` 的空文档。

#### 变更

- **`get()` 仅接受字面键** — `YamlDocument.get()` 不再将含 `.`/`[` 的键视为 JSONPath；所有键都按顶层
  映射键处理（与 `__getitem__`/`__setitem__` 一致）。路径访问请使用 `find()`/`node()`。

</details>

### [v0.13.0] — 2026-08-10

<details markdown="1">
<summary>MSRV 1.96 与 edition 2024 · 直接写入器与加载快路径 · granit 迁移</summary>

#### 变更

- **Rust MSRV 提升至 1.96，edition 升级为 2024** — 两个 crate 均声明 `rust-version = "1.96"` 和
  `edition = "2024"`；CI 将 `build`/`test-freethreaded` 任务固定在 Rust 1.96 以生成确定性 wheel；新
  增 `msrv-check` 任务在 MSRV 上运行 `cargo check`/`cargo test` 防止静默漂移（`rust-lint` 仍使用
  `stable`）。版本基线高于 PyO3 0.29 自身的基线（rustc 1.83），目的是获得 std API 的前瞻性支持（如
  `assert_matches!`，1.96 稳定），无需代码迁移。`TAG_REGISTRY`（标签处理器存储）重构为
  `std::sync::LazyLock`，移除了 `Mutex<Option<...>>` 间接层。

#### Performance

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer** — Python→YAML 序列化
  无需中间 `CustomNode` AST。单次 `direct_dump` 替换旧的两次传递 `pyobject_to_node` + `to_yaml`。
  `safe_dump` 提速 7 倍（28ns→4ns），`from_dict` 提速 6 倍（35ns→6ns）。(#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking** — 当输入不含 `&` 字符
  时，跳过 `collect_anchors` 和锚点解析，使用更简单的 `node_to_pyobject_simple` 路径。(#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — 非数字/
  非布尔首字节立即返回 `Str`，避免常见情况下的 schema 解析开销。(#59)
- **迁移到 granit-parser** — 用 granit-parser 1.0.1 替换 saphyr-parser，借助原生 `Event::Comment` 输
  出消除了全文 `scan_yaml()` 预扫描。parse_small -18%、parse_large -21%、roundtrip_large -18%。

#### Fixed

- **紧跟“只有标签、没有正文”的值之后的注释，既保住归属又一轮即稳定** — 只渲染成标签的值
  （`k: !`、`k: !-`、`k: !!str`）会让扫描器停留在“值未完成”状态，因此紧随其后的注释行会被报成
  该**值**的前导注释：注释换了主人，文档要第二轮才稳定。有三条路径以这样到达——
  `crash-cf49fe85`（容器自己的注释在 pair 行的槽位已被占用时只能另起一行）、
  `crash-c5b367d3`（23 字节；merge 键的注释被重新安家到它贡献的那一对之前），以及前两者在任意
  tag 写法下的表现。写手现在于写出注释行之前先闭合挂起行，写出该值自己的 null 正文
  （`k: !- ~`），从而让注释留在 AST 指定的节点上；注释本可行内留在 pair 行时，输出保持原样不变。
  若两者确实冲突（容器注释必须另起一行、又要保住归属），则以往返契约的稳定为先，且该取舍已写在
  决策处而非默认假设。
- **映射自己的注释不再漂到“裸 tag 值”那一行后面** — 当块式映射 body 的最后一对，其值只
  渲染成一个孤立的 `!`（非特定标签、无正文）时，把容器的 inline 注释追加在那行之后，重读
  会变成该**值**的 leading 注释，于是容器在重读时失去它，文档要晚一轮才稳定（`~: ! # -` →
  `~:  # -\n  # -\n  ! `；libFuzzer `yaml_roundtrip`，`crash-cf49fe85`，最小化到 13 字节）。
  写手现在把注释放在读者会报告它的位置，因此一次发射即是不动点，而 AST 仍按解析结果把注释
  留在容器上。*具名* 标签（`!-`）会闭合自己的属性、注释留在行内也能正确重读，所以未被改动
  ——为该形状已存的两个 pin（`crash-11ced252`、`crash-22cb5f67`）仍断言其原文本。代价：指令数
  门禁三次拒绝后的结果——逐对做判定时 `serialize_small` +2.75%，判定提出循环但实参仍被急切求值
  时 +1.39%，直到罕见路径单独成循环、常规路径保留原形状后 +0.54%。
- **被折叠的重复 `<<` 不再把它定义的 anchor 变成孤儿** — `<<: &b` 是字面量、空值的合并键，
  按我们自己的规则它是普通键；它与下方真正的 `<<:` 折叠到了一起，而折叠只把被丢弃条目的
  **注释**重新安家，没有管它携带的 anchor。于是发射出的文本用了 `*b` 却全文再无 `&b` 定义
  ——那是我们自家解析器都会拒绝的文本（`found unknown anchor`，破坏了"从不发射不可解析
  输出"的契约，`crash-43eca7a3`，18 字节，用修复前的二进制最小化到 15：已修复的输入无法再
  被缩减）。接收器现在会留下被丢弃的节点，并在文档收尾时把它内联进每一个"名字已无定义"的
  别名处：正常共享的别名完全不被动到，且只有折叠真的删掉了带 anchor 的条目时才走这一次遍历，
  常规路径零开销。`<<: &b LF <<: LF : *b` 现在发射 `<<: ~ LF ~: &b ~`——一轮即稳定，值不变。
  一句实话：孤儿别名会被它指向的节点替换，因此值语义保住而"共享同一节点"的拼法不再保住；
  定义都没了，本就无可共享的身份。
- **只由换行组成的块标量值不再退化成空串** — `>+8\r\r#` 读作一个值为单个换行、chomping 为 `Keep`、显
  式缩进为 8 的折叠标量。写手保留了缩进指示符，发射出 `>+8\n\n`；它重读时值不变但变成 `Clip` 且无指
  示符，于是下一轮发射 `>\n\n`，值就成了 `""`——Clip 会剥掉尾部换行，而该值仅有的那一个换行无处安放。
  让*空* body 保持幂等的那两条规则（丢弃不可恢复的缩进指示符；写出能重读回同一值的 chomping）都以
  “空”为条件，而“全是换行”的 body 并不为空（libFuzzer `yaml_roundtrip`，`crash-2f6b1eff`，6 字节——
  已是最小：输入不再崩溃，`tmin` 无从缩小）。两个写手现在都以“没有内容行”为共同条件，形状一轮即达不
  动点，值也保住了。代价：首版对同一个值多扫了一遍，`serialize_block_scalars` 实测 +0.65%——在 runner
  上是 +2.11%，越过指令数门禁的 2% 容差，于是门禁在合并前拦下了这次改动。改为复用写手已经算出的首个
  内容行判据后降到 +0.15%，回到容差内。
- **合并键不再把 anchor 挪到别名的后面** — 展开 `<<:` 时合并对曾被前插到映射开头，于是
  在较早的自有键上定义 `&b`、又在被合并的映射里使用 `*b` 的文档，会先发射 `*b` 再发射
  `&b`：那是自家解析器都会拒绝的文本（`found unknown anchor`），破坏了引擎“从不发射不可
  解析输出”的契约（`crash-9b77aea4`，78 字节，最小化到 15）。展开现在插入在 `<<:` 原本占
  据的下标，也就是作者写下的顺序 —— 合并键在首位时仍是 0，所以文档里 `<<: *defaults` 的
  写法不受影响（Rust 测试 487 → 489、Python 测试 1781，全部无需改动即通过）。
- **`float_to_yaml_string` round-trip 修复** — Rust Display 丢失小数部分时
  补 `.0`（`42` → `42.0`），使 float 按 float 而非 int 正确往返。
- **回退 `count_nodes` 预分配** — 全 AST 遍历的开销大于其避免的重新分配
  （serialize_10mb 慢约 14%）；缓冲扩容交给 Vec。

#### Added

- **`max_depth` 支持流式与 frontmatter API** — `parse_stream(yaml, on_event, max_depth)`、
  `read_markdown(path, schema, max_depth)`、`read_markdown_str(content, schema, max_depth)`
  接受 `max_depth`（默认 1000）。流式解析现通过核心 `parse_stream_with_options`
  强制嵌套深度限制（此前流式事件没有深度限制）。
- **Pydantic 集成** — `dump_pydantic()` 将 Pydantic 模型序列化为 YAML 字符串
  （`model_dump(mode='json')` + `safe_dump`）；`parse_as()` 将 YAML 字符串解析为 Pydantic 模型实例。
  两者均使用延迟导入，无硬性 pydantic 依赖。(#61)

#### Internal

- **拆分 `py/mod.rs`** — 单体 1786 行模块拆分为 `document.rs`（YamlDocument）、`yaml_instance.rs`
  （YAML 类）、 `functions.rs`（模块级函数）、`stream_iterator.rs`、 `walk_helpers.rs`。`mod.rs` 缩
  减至 128 行。(#61)
- **`needs_quotes()` 守卫 + `double_quoted_scalar()` 构造器** — `'true'` / `'42'` / `'null'` 等字符
  串现以双引号标量输出，避免 core schema 重新解析时被误读（`pyobject_to_node` +
  `json_value_to_node`）。
- **CodSpeed 基准统一到 `codspeed-divan-compat`** — `exclude-allocations` 去除分配器噪声；跨库基准合
  并到 `tests/test_benchmark_crosslib.py`，引入共享 `tests/data/yaml_samples.py` 夹具和流式覆盖。

</details>

### [v0.12.1] — 2026-08-06

<details markdown="1">
<summary>`set(create_missing)` · `walk`/`scalars` · monorepo · 解析热路径</summary>

#### Added

- **`set(create_missing=True)`** - 编辑路径上缺失的中间映射键会创建为嵌套映射
  （例如，对 `a: 1` 设置 `a.b.c` 会创建 `b` 和 `c`）；索引段缺失仍报错，
  路径上的标量中间层仍会引发异常。
- **`doc.walk()` / `doc.scalars()`** - Rust 后端的深度优先 AST 遍历，返回 `Node` 对象，避免逐节点
  `to_dict()` 解析。`walk()` 返回所有节点；`scalars()` 仅返回标量/null 节点。
- **Rust 核心模块测试** - 39 个新测试，覆盖 `editing::navigate` （key_eq、navigate、navigate_mut、
  normalize_index、mapping_key_index）、 `editing::region`（行辅助函数、node_is_flow、
  extend_delete_over_comments、 nav_err）、`editing::dirty`（DirtyKind/DirtyUnit 构造函数）以及
  `editing::metadata`（with_metadata_from、needs_quoting）。
- **Python doc.walk() 边界测试** - 9 个新测试，覆盖空文档、空值、
  深度嵌套、流集合、混合类型。

#### Changed

- **Monorepo workspace** - 源码拆分为 `crates/pyrs-yaml-core/` （纯 Rust，无 PyO3）和
  `crates/pyrs-yaml/`（PyO3 绑定）。根 `Cargo.toml` 现在是 workspace。旧的 `src/` 目录和 `build.rs`
  已移除。
- **pyproject.toml** - 新增 `tool.maturin.manifest-path` 指向
  `crates/pyrs-yaml/Cargo.toml`。
- **解析热路径** - 单次注释/锚点提取、延迟重复键检测、`shift_insert` 合并预处理，以及单文档解析跳过
  `DocumentEnd` 深拷贝，大文档解析成本降低约 19%（CodSpeed: parse[large] +13.9%，parse[medium]
  +16.6%，roundtrip[large] +12.2%）。
- **`Arc<str>` 标量存储** - `CustomNode::Scalar` 和注释/事件文本通过 `Arc<str>` 共享分配；AST 节点
  缩减 8 字节，克隆变为引用计数递增而非深拷贝。

#### Fixed

- **`set(create_missing=True)` 嵌套链构建** - 创建的映射链
  不再将第一段重复为嵌套键层级。
- **`set(create_missing=True)` 资格检查** - 新创建的键现在
  可参与值写入（资格检查不再在合成对插入后运行）。
- **简单映射键前的独立注释** - 往返之前会丢弃附加到
  简单键节点的独立注释；现在保留（两个回归测试）。

</details>

### [0.11.7] - 2026-08-04

<details markdown="1">
<summary>release-guard 静态断言取代常红的 stub-build-check · numpy 追踪</summary>

#### Changed

- **stub-build-check 替换为 release-guard** — 故意失败以复现 v0.10.0 `--generate-stubs` 失败模式的总
  是失败的容器构建（`validate.yml`）被三个静态断言替换，当仓库正确时**通过**：`grep` 保护
  `publish.yml` 不含 `--generate-stubs`，`git ls-files` 断言提交的 `.pyi` 已追踪，`test -f` 检查
  `py.typed` 存在。任务现在在正确状态下给出绿色 CI，仅在回归时红色。

#### Added

- **Numpy free-threaded 跟踪** — ROADMAP.md 现在跟踪 `rust-numpy` free-threaded 支持状态
  （PyO3/rust-numpy#476），作为 Rust 绑定成熟后在 cp314t wheel 上重新启用 ndarray 序列化的依赖。

</details>

### [0.11.6] - 2026-08-04

<details markdown="1">
<summary>cp314t wheel 去 numpy · free-threaded CI 校验 · 安装文档</summary>

#### Changed

- **Free-threaded（cp314t）wheel 不再包含 numpy** — 使用 `--no-default-features` 构建，rust-numpy 完
  全排除（更小的二进制，无运行时探测）。free-threaded 构建上对 `numpy.ndarray` 调用 `safe_dump` 会
  引发 `YamlTypeError`；GIL 构建（Python 3.8-3.15）保留完整的 ndarray 序列化。

#### Added

- **Free-threaded CI 验证** — `test-freethreaded` 任务现在使用 `--no-default-features` 构建和测试，
  与分发的 free-threaded wheel 配置匹配。
- **安装文档** — `docs/{en,zh,ja,ko}` 注明 free-threaded
  wheel 不含 numpy（cp314t 上不可用 ndarray 序列化）。

</details>

### [0.11.5] - 2026-08-04

<details markdown="1">
<summary>解析健壮性 3/4/5：审计结论为空，关闭并钉住回归语料</summary>

#### Changed

- **解析器健壮性项目 3/4/5 通过 Phase 0 严格性审计关闭** — 70 探针语料库（缩进、块映射键、流上下文）
  与 PyYAML 预言机对比，显示**无可修复的接受但无效案例**（64/70 匹配；6 处分歧是有意为之的 YAML 1.2
  / yaml-test-suite 要求，PyYAML 是异常项，另有一个有意为之的重复键严格性）。合规率保持在 **99.75%
  （405/406）**。完整说明见 `ROADMAP.md` §v0.11.5 和 `tests/test_strictness_audit.py`。

#### Added

- `tests/test_strictness_audit.py` — 70 探针严格性回归语料库，固定当前拒绝/接受行为（两个方向），使
  未来解析器变更无法静默降低严格性或过度拒绝。

</details>

### [0.11.4] - 2026-08-04

<details markdown="1">
<summary>重复空键不再报错（2JQS）· 合规统计计入正确拒绝 · tab 解码</summary>

#### Fixed

- 重复的空/空映射键不再报错（`: a\n: b`、`~: a\n~: b`）— 与
  yaml-test-suite 2JQS 匹配；真实重复键仍会引发 `YamlDuplicateKeyError`
- 合规检测工具：正确拒绝的无效 YAML 现在计为通过（之前尽管行为合规
  却降低了通过率）
- 合规检测工具：`convert_special_chars` 通过正则表达式解码制表符 —
  任何 `—`/`‖` + `»` 序列均为一个制表符，修复制表符编码的套件用例

#### Changed

- YAML Test Suite 通过率阈值从 >75% 提升至 **≥95%**；当前通过率
  **99.75%**（405/406）
- 记录已知偏差：`ZYU8`（`%YAML 1.1 1.2`）按设计拒绝（YAML 1.2
  语法无效，与 PyYAML/libyaml 一致）

</details>

### [0.11.3] - 2026-08-03

<details markdown="1">
<summary>流式写入 · 行偏移缓存 · publish 预校验 · 合规报告</summary>

#### Added

- 流式写入：`YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)`，文档级恒定内
  存，自动 `---` 分隔符，以及 `explicit_start`/`explicit_end` 标志
- `YamlDocument` `with` 上下文管理器：快照/回滚事务作用域
- `compliance_report()`：公开 YAML Test Suite 通过率报告（版本一致）

#### Changed

- 编辑爆发行偏移缓存：拼接层内部 O(N+edit) 传递（公开 API 不变）
- `compute_compliance` 从测试移至 `pyrs_yaml.compliance`；版本不再硬编码

#### Fixed

- 变更日志镜像漂移防护：prek 钩子 + CI 任务断言根/镜像
  `[Unreleased]` 同步
- 发布存根预验证：CI 在 Release 前复现 v0.10.0 类 `--generate-stubs`
  容器失败

</details>

### [0.11.2] - 2026-08-03

<details markdown="1">
<summary>解析不再预计算拼接资格 · 线性游标布局检查</summary>

#### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`：
  O(锚点数 + 块) 内存的惰性事件迭代器

#### Performance

- **解析不再计算拼接资格** — O(文档) 布局检查现在在首次编辑时通过 `YamlDocument.splice_checked` 惰性
  运行，恢复 v0.11.0 回归： parse_comments -59%、parse_anchors -42%、parse/roundtrip/edit -10~35% 全
  部回到 v0.10.0 水平
- **线性游标布局检查** — 取代基于预计算行偏移的逐节点二分查找
  （单调源码顺序遍历）

#### Changed

- `parse_with_options` 返回 `CustomNode`（原为 `(CustomNode, bool)`）；
  拼接资格现在内置于 `YamlDocument` 并按需计算

</details>

### [0.11.0] - 2026-08-02

<details markdown="1">
<summary>外科手术式序列化：编辑写回不重排整篇文档</summary>

#### Added

- **精准序列化** — 字节级源码范围追踪；基于段的拼接 — 编辑仅重新生成
  触碰区域，未触碰文本按字节复制
- proptest 保真度属性测试（新开发依赖）
- 10MB 编辑-刷新基准测试（divan）

#### Changed

- `flush_source` 现在使用分段拼接；回退到全量序列化：流风格区域、非默认布局文档、合并键、CRLF/BOM 文
  档，以及 materialize 之后（单次爆发模型）
- 拼接编辑保留 `---`/`...`/指令标记行为未触碰字节
  （全量序列化之前会丢弃它们 — 设计上的行为差异）

</details>

### [0.10.0] - 2026-08-01

<details markdown="1">
<summary>就地编辑 API 与编辑基准</summary>

#### Added

- **就地编辑** — 编辑已解析的文档而不丢失格式元数据：
    - 路径 API：`doc.set(path, value)`、`doc.insert(path, index, value)`、
      `doc.append(path, value)`、`doc.delete(path)`、`doc.rename(path, new_key)`，使用 JSONPath 风格
      路径（`$.a.b[0]`）；根节点语法糖 `doc["key"] = value` 和 `del doc["key"]`
    - 节点 API：`doc.node()` / `doc.find(path)` 返回 `Node` 对象，支持 `set_value` / `append` /
      `insert` / `delete` / `rename`，以及树遍历（`parent`、`children`、`walk`、`filter`）
    - 完整元数据保留 — 被替换的标量保留注释/锚点/标签/引号；
      重命名的键保留位置和注释；删除时映射顺序保留
    - 原子编辑 — 失败的操作不会改动文档（及其修订号）
    - 惰性源文本重新同步 — `source()` / `to_yaml()` / `reparse()`
      仅在编辑成功后重新序列化
    - 过期节点检测 — 文档编辑后访问 `Node` 引发
      `YamlDocumentError`（并发出 `RuntimeWarning`）
    - 新异常：`YamlEditError`、`YamlPathError`
      （支持 en/zh-CN/ja-JP/ko-KR 国际化）
    - 别名感知编辑 — 设置别名自身路径会就地替换它；
      穿过别名编辑引发 `YamlEditError`
- **编辑基准测试** — `benches/yaml_bench.rs` 新增 6 个 divan 基准
  （小到大文档的 set/insert/delete）

#### Changed

- `YamlDocument.source()` 现在返回 `str` 并在就地编辑后惰性重新序列化

</details>

### [0.9.0] - 2026-08-01

<details markdown="1">
<summary>CPython 3.13/3.14/3.15 与 no-GIL · 标签处理器注册表 · pydantic · `.pyi`</summary>

#### Added

- **Python 3.13、3.14 和 3.15 支持** — PyO3 `abi3-py38` wheel 覆盖 Python 3.8-3.15（GIL 构建）；
  `abi3t` + `abi3t-py315` 提供 free-threaded 稳定 ABI
- **Free-threaded CPython（无 GIL）支持** — `#[pymodule(gil_used = false)]` 声明模块对 free-threaded
  Python 线程安全；`Py_GIL_DISABLED` cfg 标志门控 numpy（rust-numpy 尚不支持 free-threaded — 通过
  `--no-default-features` 为 free-threaded 构建禁用 numpy feature）
- **CI free-threaded 任务** — 新增 `test-freethreaded` 工作流任务，
  针对 Python 3.14t 验证编译和测试
- **`pyo3-build-config` 构建依赖** — 通过 `build.rs` 启用
  `#[cfg(Py_GIL_DISABLED)]`、`#[cfg(Py_3_15)]` 等编译器标志
- **`numpy` 改为可选** — 由 `numpy` feature 门控（默认启用）；
  在 `Py_GIL_DISABLED` 下自动排除
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`、
  `parse(..., allow_duplicate_keys=True)`、`parse_file`、`safe_load`、`safe_loads`、
  `parse_all_docs` 均接受该标志；重复映射键默认引发 `YamlDuplicateKeyError`，允许时采用"最后值生效"
- **`SerializeOptions` 扩展** — `doc.to_yaml_with_options()` 新增 `width`（行包裹，0 = 关闭）、
  `indent_mapping`、`indent_sequence`、`indent_offset`，与现有的 `indent_size`/`explicit_start`/
  `explicit_end`/`sort_keys`/`max_depth` 并列（`src/py/mod.rs:432`）
- **标签处理器注册表** — `register_tag("!custom")` 装饰器和命令式形式
    - `clear_tag_handlers()`；携带已注册标签的标量节点通过处理器转换
  （`src/py/tag_registry.rs`）
- **标签处理器优先级链** — 同一标签的多个处理器按升序 `priority`
  执行；`YamlTagSkip` 让处理器传递给下一个，fallback 保留原值
- **Pydantic 集成** — `parse_as(Model, yaml, **yaml_kwargs)` 解析 YAML 并针对 Pydantic v2 模型验证；
  缺少 pydantic 时引发 `ImportError` 并附指导信息（`python/pyrs_yaml/pydantic.py`）
- **`.pyi` 类型存根** — 由 maturin 自动生成并提交，使 `register_tag`、`parse_as`、
  `to_yaml_with_options` 和新异常对类型检查器可见

#### Changed

- CI Python 矩阵扩展：ubuntu、windows、macos 上的 3.8-3.14
- 稳定 ABI：`abi3-py39` → `abi3-py38`（更广的 Python 3.8+ 支持），
  新增 `abi3t` + `abi3t-py315`（free-threaded 稳定 ABI）
- `pyproject.toml` classifiers 更新 3.13、3.14、3.15 条目
- **CI 优化：消除冗余 Rust 编译** — 单个 `rust-lint` 任务运行 `cargo clippy` + `cargo test` 一次；
  `build` 任务为每个 OS 生成一个 abi3 wheel，测试任务安装而非运行 `maturin develop`，将 Rust 编译从
  21 个矩阵任务中移除（减少约 86% 编译量）；所有任务添加 `Swatinem/rust-cache`
- **pydantic 测试依赖** — `pydantic>=2.10.6` 加入 `[dependency-groups] test` 和
  `.ci/requirements-test.txt` （通过 `uv sync` 在 ci.yml 中统一管理）

#### Fixed

- **Windows DLL 加载** — 移除 `src/py/tag_registry.rs` 中的 `#[cfg(test)]` 块，该块在 Windows 上破坏
  了 `import pyrs_yaml` （`250b8d0`）
- **Python 3.8 兼容性** — `pydantic.py` 中添加
  `from __future__ import annotations`（`63d2495`）
- **CI pydantic 跳过** — 使用 `pytest.importorskip("pydantic")`
  使测试在未安装 pydantic 时通过（`7be011d`）
- **CI Windows glob 展开** — `pip install dist/*.whl` 使用
  `shell: bash`（PowerShell 不展开 `*`）（`2f7778d`）
- **非字符串标签处理器返回值现在引发 `YamlTagError`** — 返回非 `str` 值的处理器（之前被静默忽略，保
  留原标量）现在报错 `Tag handler '!x' must return a string`（`src/py/mod.rs:resolve_tags`）
- **`to_yaml_with_options` 缩进连线** — `indent_mapping`/ `indent_sequence`/`indent_offset` 现在被序
  列化器尊重（之前为死字段）；省略时分别默认 `indent_size`/0（`src/serializer.rs`）
- **`width` 不再对极小值死循环** — `width < 续行缩进` 时回退为
  直接输出未包裹的剩余内容而非无限循环（`src/serializer.rs:write_plain_scalar`）
- **`remove_tag(name)`** — 新增函数用于注销标签处理器；
  补充 `register_tag`/`clear_tag_handlers`（`src/py/tag_registry.rs`）
- **`duplicate-key` 错误国际化** — `YamlDuplicateKeyError` 消息现通过 `format_i18n_error` 在所有 4
  个语言区域中传递（`src/i18n/locales/*.yml`）

</details>

### [0.8.0] - 2026-07-30

<details markdown="1">
<summary>`YAML()` 实例 API · Python `Node` API · `MergedView` · 生命周期告警</summary>

#### Added

- **`YAML()` 实例 API** — `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)`，
  可复用配置；`.parse()`、`.safe_load()`、`.safe_loads()`、 `.parse_file()`、`.parse_all_docs()` 方
  法
- **Python `Node` API** — `Node` 类，具有 `find()`、`filter()`、`walk()`、`to_yaml()`、`parent`、
  `children`、`root_type`、`value`，用于 AST 导航；JSONPath 风格查询语言（`$.key.sub`、 `$.arr[0]`、
  `$..deep`）
- **`doc.version` 元数据** — `YamlDocument.version()` 返回 YAML
  规范版本（默认 "1.2"）
- **`MergedView`** — `doc.merged()` 返回解析合并键后的只读
  类字典视图
- **生命周期警告** — `Node.release()` 显式使节点失效；过期
  访问发出 `RuntimeWarning` + `YamlDocumentError`

#### Changed

- `parse()` / `safe_load()` 现在作为语法糖委托至
  `YAML().parse()` / `.safe_load()`
- `YamlDocument` 现在存储 `version` 字段用于文档元数据

</details>

### [0.7.1] - 2026-07-30

<details markdown="1">
<summary>ryaml 基准对比 · 合规阈值上调 · Divan 迁移</summary>

#### Added

- **ryaml 基准对比** — `tests/test_benchmark.py` 现在与 `ryaml`（Rust YAML 库）alongside PyYAML 和
  ruamel.yaml 进行基准测试；`benchmark_compare.py` 重写为特性对比报告
  （`tests/test_benchmark.py:25-28`、`.github/workflows/ci.yml:219`）
- **CI 合规阈值提升** — YAML Test Suite 合规阈值从 70% 提升至 75%（`test_compliance_report()`）；有
  效解析率阈值 95% （`tests/test_yaml_suite.py:251`）
- **CI 依赖整合** — 新增 `.ci/requirements-test.txt` 和 `.ci/requirements-test-lite.txt`，用于发布工
  作流和本地开发的统一测试依赖管理
- **基准测试现代化** — 从 `pytest-benchmark` 迁移至 `pytest-codspeed` 以实现更快的 C 扩展统计基准测
  试；所有 CI 任务现在使用 `-r .ci/requirements-test.txt`
- **Rust 基准测试迁移至 Divan** — 用 `codspeed-divan-compat` v5.0.1 替换
  `codspeed-criterion-compat`；16 个基准测试从 Criterion 组重写为 `#[divan::bench]` 属性
  （`Cargo.toml`、`benches/yaml_bench.rs`）

#### Changed

- CI 基准任务安装 `ryaml` 用于跨库对比
- `benchmark_compare.py` 现在委托计时至 `pytest-benchmark`，
  作为特性对比/报告工具

</details>

### [0.7.0] - 2026-07-29

<details markdown="1">
<summary>序列化器 `max_depth` 防护 · 热路径优化 · pytest-benchmark 迁移</summary>

#### Added

- **序列化器 `max_depth` 守卫** — `serialize_node_internal` 现在跟踪递归深度，超出限制时引发
  `YamlMaxDepthError`（默认 1000），与解析器保护一致（`src/serializer.rs:135-145`）
- **序列化器热路径优化** — 5 项针对块风格序列化的优化，约
  4.9% 往返加速：
    - 内联 `write_anchor_tag` 和 `write_inline_comment` None 检查
      （消除约 99% 节点的函数调用）
    - `write_indent` 热/冷路径分离（缓存级别 ≤64 直接索引）
    - `write_plain_scalar` 短 ASCII 字母数字字符串快速路径
      （≤8 字符）
    - `write_scalar_for_key` Plain 标量直接分派（避免分派链）
- **pytest-benchmark 迁移** — Python 基准测试从原始 `time.perf_counter()` 迁移至 `pytest-benchmark`
  以获得统计严谨性、结构化 JSON 输出和 CI 集成（`tests/test_benchmark.py` + 更新的
  `tests/test_performance.py`）

#### Changed

- `pytest-benchmark` 替换 Python 基准测试中的原始 `timeit`
- CI 基准任务现在运行 `pytest --benchmark-json` 而非独立脚本

#### Removed

- `write_inline_comment` 方法 — 在所有调用点内联
- 序列化器的 `Comment` 导入 — 不再需要

</details>

### [0.6.0] - 2026-07-27

<details markdown="1">
<summary>异步序列化 · JSON Schema 校验 · `to_json()` · 增量重解析</summary>

#### Added

- **异步序列化** — `safe_dumps_async`、`safe_dump_async`、 `safe_loads_async`、`safe_load_async`，通
  过 `asyncio.run_in_executor` （`python/pyrs_yaml/async_dump.py`）
- **JSON Schema 验证** — `YamlValidateError` 异常 + `YamlDocument.validate(schema)` 方法（接受 `str`
  或 `dict`）；委托至 Python `jsonschema` 模块
- **`YamlDocument.to_json()`** — 将文档序列化为 JSON 字符串
  （使用 Python `json.dumps`）
- **增量重新解析** — `YamlDocument` 现在存储源文本（`doc.source()`）；
  `doc.reparse(resolve_merges=True, schema="core")` 就地重新解析
- **29 个新测试** — 跨 `test_async.py`（8）、`test_validate.py`
  （14）、`test_reparse.py`（7）

#### Changed

- `YamlValidateError` 注册为新自定义异常（继承 `ValueError`）
- `rust_i18n::i18n!` 宏路径更新为 `"src/i18n/locales"`
- `validate_translations()` 测试路径更新以匹配新语言目录

#### Removed

- 删除冗余的 `src/i18n/en.ftl`、`src/i18n/zh-CN.ftl`（从未被
  rust-i18n 引用）
- 将 `locales/*.yml` 移至 `src/i18n/locales/`（与 i18n 模块共置）

#### 依赖变更

- 运行时依赖：`jsonschema>=4.25.1`
- 开发依赖：`pytest-asyncio>=0.23`（从运行时移至开发依赖，不再固定）

</details>

### [0.5.0] - 2026-07-27

<details markdown="1">
<summary>`Serializer::write_node` 修复 · `YAML_SCHEMA` 常量 · 开发文档</summary>

#### Fixed

- **`Serializer::write_node`** — `block_mapping`/`block_sequence` 中 `.unwrap()` on
  `values.iter().next().unwrap()` 替换为安全索引访问，消除边缘 AST 的潜在 panic
- **`YAML_SCHEMA` 常量** — 拼写错误 `yamorg2002` 修正为
  `yamlorg2002`（匹配 YAML 1.2 规范 URL）
- **开发文档** — `AGENTS.md` 更新，为 Python 命令添加强制
  `uv run` 前缀，Rust 命令直接使用 `cargo`

</details>

### [0.4.0] - 2026-07-27

<details markdown="1">
<summary>132 个补漏测试：i18n、多文档、字节输入、逐例套件、保真</summary>

#### Added

- **132 个新功能填补测试** — 全面覆盖之前未测试的 API
- **i18n 函数测试** — `set_language`、`get_language`、
  `list_languages`、`detect_language`、`negotiate_language`
- **`parse_all_docs` 专用测试套件** — 单文档、多文档、空、注释
- **`parse_file` 成功用例测试** — 基本解析、注释保留、文件未找到错误
- **`to_yaml_with_options` 测试** — `explicit_start`、`explicit_end`、
  `indent_size`、`sort_keys` 顺序保留
- **`to_dict()` 方法测试** — 标量根、嵌套、列表、布尔、空、
  锚点解析、空映射/序列
- **YamlDocument dunder 方法测试** — `__repr__`、`__str__`、
  `__contains__`、`__len__`、`__iter__`、`__getitem__`、`root_type()`
- **字节输入测试** — `parse(b"key: value")`、UTF-8 字节、无效 UTF-8 错误
- **Unicode 及特殊字符测试** — 中日韩、emoji、往返、CRLF 换行、重复键
- **`safe_load`/`safe_loads` 特性覆盖** — 锚点、合并键、块标量、
  流集合、特殊浮点数、类型解析
- **`from_dict` 边界用例** — 键中的特殊字符、嵌套列表、None 值、
  空字典/列表
- **`from_json` 往返** — 嵌套结构、数组、无效 JSON 错误
- **`dump_file` 测试** — 成功路径、无效路径错误
- **YAML Test Suite 单个用例测试** — 八进制、十六进制、科学计数法、NaN、无穷大、合并键、显式/隐式
  键、布尔/空变体、块标量截断（`|-`）、流集合
- **`resolve_merges` 参数测试** — 禁用时保留 `<<`，默认时解析
- **流集合往返** — 根级和嵌套流映射/序列
- **非标量节点上的锚点** — 映射锚点（`&defaults`）和序列锚点（`&items`）
- **序列索引测试** — 正索引、越界错误
- **合并键集成** — 解析和未解析合并键的往返
- **标签保留** — `!!seq` 和 `!!map` 标签测试覆盖
- **注释保留** — 复杂结构上的内联和独立注释测试

#### Changed

- 修复版本同步：`python/pyrs_yaml/__init__.py` 的 `__version__`
  从 0.2.0 更新至 0.4.0 以匹配 Cargo.toml/pyproject.toml
- 移除 `dist/` 中过时的 0.2.0 wheel 产物

</details>

### [0.3.0] - 2026-07-27

<details markdown="1">
<summary>NumPy ndarray 序列化（N-D）· 引号标量类型 · 负数往返</summary>

#### Added

- **NumPy ndarray 序列化** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` 现在支持
  所有维度的 `numpy.ndarray`（0-D 至 N-D）
    - 支持的数据类型：`int8/16/32/64`、`uint8/16/32/64`、
      `float32/64`、`complex64/128`、`bool`
    - 多维数组序列化为嵌套 YAML 列表，缩进正确
    - 复数序列化为 `(re+imj)` 字符串格式
    - `0-D` 标量数组重塑为 1-D 并序列化为单元素列表
    - 通过 `numpy` Rust crate 的 `PyUntypedArray` + `PyArrayDyn`
      实现零拷贝 dtype 分派
    - 切片迭代期间释放 GIL 以获得最大性能
- **`quoted_scalar()`** — 新增 `CustomNode::quoted_scalar()`
  构造函数，用于需要单引号 YAML 风格的值
- **引号标量的类型解析** — `resolve_yaml_type` 现在应用于
  `SingleQuoted`/`DoubleQuoted` 标量，以正确往返引号负数
- **全面 NumPy 测试套件** — 42 个测试覆盖所有数据类型、
  维度（0-D 至 4-D）、负数、无穷大、NaN、空数组及边界用例

#### Fixed

- **负数往返** — YAML 1.2 块序列不能包含以 `-` 开头的纯标量；
  序列化时负数现在加引号，解析时正确还原为整数/浮点数
- **N-D 数组支持** — 用 `PyArrayDyn<T>` 替换 `PyArray1<T>`，
  支持任意维度数组而不仅限于 1-D
- **正确嵌套深度** — 多维数组现在产生恰好 N 层嵌套
  （shape[1..] 处理内部维度，根维度由 `plain_sequence` 包裹）

#### Changed

- 新增 `numpy` crate（v0.29）作为 ndarray 类型分派的依赖

</details>

### [0.1.0] - 2026-07-25

<details markdown="1">
<summary>首个版本：YAML 1.2 全元数据 AST、往返保真、PyYAML 兼容 API</summary>

#### Added

- 初始发布，通过 saphyr-parser 实现 YAML 1.2 合规
- 自定义 AST，完整元数据（注释、锚点、标签、chomping、标量风格）
- 注释、锚点、标签和格式的往返保留
- PyYAML 兼容 API（`safe_load`/`safe_dump`）
- `from_dict`/`from_json` 转换函数
- `read_markdown`/`read_markdown_str` 用于 YAML frontmatter 提取
- 块标量（`|`/`>`）带 chomping 指示符（`|-`/`|+`/`>-`/`>+`）
- 转义序列（`\n`、`\t`、`\uXXXX`、`\xXX`）
- YAML 1.2 类型解析（null、bool、int、float、infinity、NaN）
- 合并键解析（`<<: *alias`）
- 复杂键（序列/映射作为键）

</details>
