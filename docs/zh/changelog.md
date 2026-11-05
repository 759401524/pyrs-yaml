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

#### 修复

- **三条关于穿过别名编辑的说法是假的，而消息打印的那套原因词汇表现已列出** — 那些页面写道穿过别名的
  一次写入会抛 `cannot-edit-alias`，又写道 `set()` 在一次写入会让还活着的定义失去引用时回
  `anchor-is-referenced`；在发布的构建上实测，`set`、`delete` 与 `rename` 都会改写 `use: *x`，且没有
  任何 `set` 路由能到达那个令牌 — 它住在 delete、rename、move 与元数据这几条路径上。别名的*父节点*仍
  然被拒绝，别名节点上的那十二个元数据动词也一样。`api/exceptions.md` 得到一张编辑原因令牌表，列出十
  五个可达令牌、应答每个令牌的那次调用与完整的实测消息，并说明哪些已声明的变体没有构造点
  （`wildcard-or-deep-scan`、`invalid-index`、`invalid-path`）。`validate()` 的委托现在写在使用者会
  碰到它的位置：本项目发布的任何东西都不安装 `jsonschema`，所以全新安装回一个裸的
  `ModuleNotFoundError`，而作为文本传入的 schema 先由 `json.loads` 读取，因此 YAML 拼法的 schema 抛
  `json.decoder.JSONDecodeError` — 两条登记为
  `api:validate-evaluator-is-not-installed-by-anything-we-ship` 与
  `api:validate-schema-text-is-json-only`，这里都不修。墙钟表也在 2026-11-04 按本版本的构建重测过一
  次，因此每一格、正文区间与出处日期都出自同一份快照；日文的两页还删掉了一个英文版从未有过的私有错误
  消息块，并把只写 `dict` 的 `validate()` 签名补成 `str | dict`。

### [v0.19.0] — 2026-11-04

<details markdown="1">
<summary>在调用点命名的策略 · 重复键 · 注释落点 · 按执法镜像键控的地板</summary>

#### 新增

- **`duplicate_keys=` 给读者的重复键策略命名，而 `as_is` 保留四种答案** — 一个开关向一个能答出四种的
  读者问出了两种行为：`a: 1` 与 `a: 2` 相邻抛 `YamlDuplicateKeyError`，`: 1` 与 `: 2` 相邻折叠成后一
  对，`0set: 1` 与 `!base 0set: 2` 相邻两对都保留、只有 `to_dict()` 拒绝，`'a': 1` 与 `a: 2` 相邻抛
  错。`as_is`（默认）*就是*这些按键的身份各自给出的答案；`error` 连默认豁免的空键与合并键一并拒绝，
  `last_wins` 正是 `allow_duplicate_keys=True` 一直以来的含义，而 `first_wins` 保留先出现的那一对，
  并把被折叠那对的注释与锚点定义移到幸存者身上。同时传入两种写法会被拒绝，无法识别的取值会说出它想要
  的四个值，两条 YAML 读路径答案一致，每个读取入口和十二个 CLI 子命令都以 `--duplicate-keys` 收它。
  在本树上的刷新抬升九条地板（`parse_medium` 322,863,958 → 325,633,529，最差 +0.86%），其余四十六条
  仍在 ±0.07% 以内；fuzz 目标按默认策略解析，所以三个具名替代值还没进 fuzz 层。
- **调用点命名这份歧义：`note_placement=` 与 `on_duplicate=`** — 一对键值的行尾注释槽位已被 `&a` /
  `!tag` 标头占掉时，写器把注释停在读者归档位置之外的标头缝隙里（`edit_yaml` crash-547a9705）；
  `note_first`（默认）按读取给出的位置写，所以第一次发射就是不动点，`header_first` 保留旧落点并注明
  不安定。两个键也能拼出同一个名字，只靠读者留存的标记或锚点区分，于是删掉一个后重读时另一个仍应答那
  个名字（crash-46d35d71）；`delete(path, on_duplicate=...)` 答 `one`（默认）、`all`，或 `error`
  （`duplicate-key-target`，树不动）。引擎接缝 `delete_path_with_policy` 每次移除返回一个变更单元并
  重跑锚点守卫，所以别名还指着的定义是被拒绝而非发出悬空；`delete_path`、`del doc[key]` 与
  `Node.delete()` 保持 `one`，而那轮的 `b7e16929`（锚点发在其别名之前）作为它本来的锚点次序问题仍然
  开着。
- **fuzz 语料做了内务：旧 artifact 归档、抽样器发现开单、种子按目标计数** — 三个没有种子也没有记录的
  artifact（`yaml_roundtrip` crash-3528db38、crash-d461a3f2、`parse_json` crash-331b36ec）回放干净，
  以 `former-crash-*.seed` 加入 `fuzz/seeds/`；这里不声称修好了什么，被证明的是归档。2026-10-31 那轮
  的六个 `toml_roundtrip`/`edit_yaml` artifact 拼出本版本 TOML 注释修复已经了结的同一种形状，六个回
  放干净。在 2026-10-03…-31 五次定时运行带着没人读的输入失败之后，定时抽样器终于有了消费者：
  `scripts/fuzz_file_issues.py` 按目标开一张 issue，带上运行链接、签名计数与输入名；
  `fuzz_seed_inventory()` 按 `fuzz.yml` 复制的方式统计每个 `fuzz/seeds/<target>/`，发布
  `fuzz_seed_counts`，并拒绝 `fuzz-corpus-unseeded` 与 `fuzz-corpus-orphan`。
- **注释预言机跨格式加宽，而这一轮的注释形状被钉住** — 删掉一叠 TOML 注释曾让不动点回放保持绿色，于
  是存活断言现在被语料管住：一条 TOML 注释存活测试向带注释的 `toml_roundtrip`/`parse_toml` 种子收费
  （2026-11-02 实测），`note_survival.rs` 的 YAML 行走得到 `edit_yaml` 的文档那一半；
  `corpus_notes.rs` 把该规则带到两种可写注释的 JSON 方言上，从 `to_jsonc_text` 或 `to_json5_text` 拿
  掉根注释那一笔就在新的 `shape-root-line-note.seed` 上判红，而
  `prop_yaml_writer_carries_every_note` 对生成的树问同样的问题。这一轮最后的发现都已了结：
  crash-1bbd4d7f 只差一个名字，它第一轮就已提升，五种形状被钉住；一行只有标记的标签（`bg: !`）把键注
  释写进上一行的陈旧起点，那是它自己的读者拒绝的文本，于是两个键分支现在都会重新取回
  （crash-2c76568e，逐字节钉住并归档）。在 `rustc 1.97.1` 上定价，没有 YAML 写器场景越过 0.5 % 那条
  线（最差 `serialize_anchors` −0.24 %）。
- **Python 生成层画出它的叶子从来说不出的那些家族** — `arbitrary_json` 把 dump 属性的叶子限制在
  `None`、`bool`、有界 `int`、有限 `float` 与 `str`：25,000 次抽样里没有别的类型。`hard_leaf` 加上无
  界整数、非有限浮点、元组、`bytes`、`date`、`time`、`datetime`、`Decimal` 与四个没有 YAML 拼法的类
  型；`test_dump_never_raises` 只接受文本或 `pyrs_yaml.YamlTypeError`，家族表钉住每种答案。每一行文
  本都重 dump 回同样的字节，只有 `safe_dump(-(2**64))` 例外 — 一处引号守卫里的正负号不对称，登记在
  `NON_SETTLING` 里作为未决缺陷。可达性守卫会点名任何在 400 次抽样里没出现的家族，
  `_register_builtins()` 守卫让答案与导入顺序无关；没有引擎代码被移动。
- **JSON Schema 表面得到一层属性测试，其中两条契约被登记** — `tests/test_property_json_schema.py` 拿
  元合法的 draft2020-12 schema 对带 YAML 类型键的文档抽样，与 `jsonschema` 比较判定与消息，并用
  `check_schema` 见证者证明重复调用命中校验器缓存。键拼法抽样暴露了那次折叠：`to_dict()` 把 `1: one`
  与 `true: two` 折成一个，而 `to_yaml()` 与 `to_json()` 两对都发，于是 `validate()` 从折叠那道门读
  数，`minProperties 2` 就拒绝 JSON 投影接受的双键文档 — PyYAML 的加载器在同一处折叠（2026-10-31 实
  测），所以那是对手的形状。两条 YAML 读路径在相邻键对上答向相反（`1:` 挨着 `'1':`、也挨着
  `true:`），登记在 `.ci/known-residuals.json` 里作为一个待裁定的问题。委托者的代价与 `str` 那道门的
  `json.loads` 在这里都不修；PR #517 设计的那台求值器会让这个问题失去意义。
- **跨格式注释入口、公开调用表面，以及 JSON 树等值** — 一次覆盖 134 个测试模块的 AST 行走发现
  `json -> jsonc` 另外四对从未被走的单向格式对；`tests/test_cross_format_note_entry.py` 就是那张表，
  每个单元断言注释的令牌抵达文本、或导出以一条对目标为真的理由拒绝，桥以字面量解析后交叉过去（`0xFF`
  以 `255` 抵达 TOML），而严格的 `to_json` 不在表内，因为 JSON 没有注释可写。
  `tests/test_api_call_surface.py` 要求已发布桩里的每个公开可调用对象都有调用点，并拒绝这类门禁悄悄
  变绿的四种方式；它发现 `get_plugin` 与 `YamlDocument.flush_source` 从未被调用。
  `prop_json_roundtrip_preserves_the_tree` 跨三种 JSON 方言比较值、形状与顺序：把 `null` 写成 `0`
  时，七条文本级属性仍绿，只有这条判红。
- **地板与地板引文向注释路由与 Python 对象写器加宽** — TOML 的注释路由被定价（七条注释：每条 1,945
  指令），没人读的行内表注释槽位有了读者；携带 #673 的 `safe_dump_*` 地板的那次派发在 `accept_drift`
  为空时动了 53 条里的 47 条，向下越过 0.5 % 线的只有 `from_json5_medium` −1.132 %、
  `from_jsonc_medium` −1.130 % 与 `parse_inline_merge` −0.551 % — 那是 harness 自身的代码体积。绑定
  层的写器由 `safe_dump_small`/`safe_dump_medium` 定价：`needs_quotes` 占 `safe_dump_medium` 的
  5.45 %，`DirectWriter::write_key` 3.70 %，完全没有 `type_registry` — #672 的空处理器快路径。
  `QUALITY_MATRIX.md` 里十一处被引用的地板有九处已在所有门禁全绿的情况下过期，于是
  `scripts/check_ir_floor_quotes.py` 拒绝过期数字、未知场景、孤立标记与未声明页面（退出 2），按镜像
  键控的标记要写出镜像名（`load_toml_medium @ 20261023.334.3`，文件里是 144,902,515 与 142,467,241，
  差 1.71 %），而 `profile`/`profile_top` 输入在执法镜像上跑 `--profile`，因为手跑的一次无法与已提交
  地板配对。
- **本仓库自己的断言现在要么被门禁管住、要么被钉住** — CI 的宽域 clippy 曾因 `required-features` 静
  默丢掉两个 Ir harness；一条被门控的步骤与一个 `prek` 钩子现在覆盖它们。九个 `python_version` 上限
  里有五个排除了其实发布了 wheel 的解释器（PR #665 在 3.13 读到 4,433 条断言、3.14 只有 4,197），现
  已抬高并由 `scripts/check_counterparty_caps.py` 双向拒绝过期。共享对象的写出形状被钉住：对象路由发
  两份（重读相等，而不是一个对象），而往返保留 `&x`/`*x`；环抛 `YamlMaxDepthError`。pytest 层的宽度
  由 `scripts/ci_matrix_plan.py` 派生（12 个单元，ubuntu 承载全部八个解释器）；`rumdl fmt` 会把段落
  或代码跨度行首的 `#` 串提升成标题且事后无法察觉，所以 `scripts/check_doc_headings.py --predamage`
  在格式化器之前拒绝该形状；ledger 的 `(xy)` 小节次序是一个钩子。四个测试（JSON/TOML 读写）把标量字
  符串扫描 — aarch64 wheel 的整条路径 — 与 SSE2 循环在一个 80 字节正文的十三个偏移上逐字节比较。
- **行分隔 JSON 抵达 Python 层：`load_json_lines`、每分片一个汇、可选 `mmap`** — 引擎自 #599 起就在
  描述 NDJSON，但 Python 层没有东西读它；`pyrs_yaml.load_json_lines(text)` 返回一个每次 `__next__`
  解一行的一文档在世的迭代器，空行、只有注释的行与续行都以点名该行的方式抛出。实测（10,000 条记
  录）：整条流 3.861 ms、首条 0.343 ms、逐行 `load_json` 8.440 ms。可选的 `parallel` feature 让每个
  分片有自己的汇与线程：200,000 行走一个汇是 112.3 ms、走 24 个是 18.9 ms（5.96x）。`parse_file` 增
  加可选的 `mmap` 路径，省掉一次整文件分配；默认关闭，空文件、目录与非 UTF-8 输入保持它们原有的错
  误。
- **流在消费者说停时停下、点名它读过的源字节，而 dumper 接受元组** —
  `parse_stream(..., on_event=...)` 曾把每个事件收集完才回调第一次，所以回 `False` 仍要付整次扫描的
  代价；`pyrs_event::EventSink` 现在返回 `ControlFlow`。`False` 停住的是格式事实：JSON/JSON5 在下一
  个事件的字节之前停，TOML 从它内建的 IR 停止发出、被拒的文档仍先抛，YAML 的适配器在不可中断的解析之
  后停止交付 — `load_stream` 仍是 YAML 的增量表面。每个 `StreamEvent` 带 `offset`/`end` — 是令牌的区
  间，不是子树的，记录位置而非范围的生产者处 `end == offset` — 事件字典也得到这两个键。dumper 接受加
  载器交回的元组（`? [a, b]` 投影成 `("a", "b")`，可两个写器都曾拒绝元组，于是
  `safe_dump(safe_load(text))` 在被接受的输入上抛错）；只有键的外层序列会投影，所以 `? [[1], [2]]`
  仍抛 Python 自己的 `unhashable type: 'list'`。

#### 修复

- **别名唯一的定义被一次遍历丢掉时，现在会在引用够得着的地方补一个** — 重复键折叠丢掉定义 `x` 的那一
  对，而活下来的 `&x` 骑在一个落在别名之后的键上，于是 `*x` 出现在所有 `&x` 之上，本库自己的读者拒绝
  自己的发回 `found unknown anchor`（`edit_yaml` crash-b7e16929）。修复原先只问定义是否在别处活下
  来；现在问这个名字是否在这次引用之前被定义，并把节点写进够不着的引用里（`inline_dropped_uses`）。
  合并遍历会删掉携带锚点的 `<<` 条目且原先没有对应的修复，把 `<<: &b` / `  x: *b` 变成第 3 列就被拒
  的裸 `x: *b`，于是 `resolve_merge_keys` 在该遍历跑完后把幸存者的锚点读出来并内联。`delete_path` 原
  先只向值节点要锚点，于是键上的 `&x` 被漏看、`k8: *x` 被接受（crash-6a9b6538、crash-718abefa）；现
  在也读键节点，而绑定层对同一次编辑仍拒以 `anchor-is-referenced`。锚点图行走 135 份文档，对照
  2026-10-20 读到的 102。
- **从别名改写而来的键在落地处折叠，而以引号结尾的锚点名保住那个名字** — `repair_orphaned_anchors`
  把被丢那对的节点内联进每个留下的 `*name`，包括键位置，但重建用普通 `pairs.insert` 再插回去，于是本
  来自身空键的映射有了两个都写作 `~:` 的键，重读折掉一个（crash-49f44d18；crash-fce042d0 是改写到普
  通对旁边时的同一次折叠）。重建现在对它插入的每个键都做折叠 — 后一对胜出、被丢那对的注释搬回幸存者
  — 并把该对的锚点节点排进下一轮内联；crash-663b71ab 与 crash-cf1a63bf 回放干净。锚点名可以像
  `is_valid_anchor_char` 对齐 granit 那样以 `"` 结尾，但 `opens_quoted_node` 让紧贴锚点或标记令牌的
  引号开启一个被引节点，于是重读一次发射时把自己行尾注释里的 `#` 当成引号内容、把其中的 `&` 当成锚
  点，在两轮之间把节点改名成 `!`（crash-abc541ff、crash-c06e8a8f）；现在性质与被引节点之间必须有一个
  空格。
- **第一条目的注释爬到标记之上，而第一项的注释留在自己的横杠之下** — 在 `?` 标记体内，写器只在容器写
  出锚点或标记标头时把第一条目的键注释抬到标记之上，而键值对写器把那一行的行尾槽位交给值的行内注释，
  于是读者报告在键映射上的注释在第二轮被提升（crash-430f4f0e）；现在只要值占有那个槽位，提升就拿走该
  注释。`lift_first_item_notes` 把块序列首项的前置栈抬到外层标头之上，对一个逐行拼写的项来说高了一层
  （crash-439f5d2c）；现在非紧凑分支把栈留在体内，只有紧凑拼法才提升。
- **标记提升把一条注释多抬一层标头，而一次改名丢掉了键携带的保真** — #726 的三个 `edit_yaml` 输入在
  两个缺陷之后仍然失败。`rename_path` 用新文本重建键，丢掉简单键携带的 `Fidelity`，于是把
  `!base: &x 0se: *x` 里的 `&x 0se` 改名后发出 `k7: *x`，一个指向未定义锚点的别名；键现在保住自己的
  锚点与标记。`spine_has_notes` 只问前置栈，而 `hoist_marker_note` 在标头下也会带走第一个键的行内注
  释，于是那条注释每轮多定居一层标头；两条行走现在问同一个问题（`first_entry_inline_note`）并以标头
  为门（crash-094d4d6a、crash-8492f5ee、crash-701f76bf、crash-6cfbaa57）。六个写器场景付在 0.5 % 线
  内（最差 `serialize_block` +0.18 %），对照没有守卫时是 +0.00 %。
- **注释不能再把一个值劈成两行：闭合移动之后键值对重新锚定自己的键行** — 一个映射条目记得自己那行从
  哪开始，而闭合一个待处理的只有标记的行会在记住的偏移之前插入该值的 `~` 却不移它，于是下一个键的行
  内注释落在刚被拉长的行中间：对 `: !` / `<<: #b` / `<<: #b` / `3 #~` 调 `to_yaml` 发出第 0 列一个裸
  `~`，本引擎自己的读者拒绝（`simple key expected ':'`）。`note_line_above` 现在把"这个偏移指着一行"
  当作不变量，并把陈旧的捕获推进到下一行起点；同一个陈旧捕获也移动过键行本身，因为值自带注释时键注释
  不能与键值对同行，于是键值对在自己的前置注释之后重新锚定键行（crash-76e05a78，从 104 字节最小化到
  25）。crash-2c76568e 来自定时发现层，而它的报表已连续五个红色星期把这条断言丢掉。
- **立在空容器之上的注释把条目留在它的横杠行上，而横杠与冒号不再为一个已决定转块的子节点开启** — 读
  者把容器第一对之上的注释归档为容器自己的前置注释，所以删掉最后一对会让注释悬空，而
  `write_mapping_pair` 本来就以 inline `{}` 并把注释降级到键值对行尾来应答；`is_compact_item` 对任何
  携带独立注释的值都拒绝紧凑的 `- key: value` 行，于是第一轮只写横杠、第二轮把横杠行拿走
  （crash-ad42b9cc）。上一层，当一个成员携带的注释比花括号装得下更多时带标记的容器会被降级，但七个判
  断只信 `flow_style` 标志，各自在自己的位置写出指示符、同时被降级的子节点在父列开块体；七个现在都问
  降级自己那个问题。新增的测试把写器家族从 +0.18 % 推到 +0.72 %（最差 `serialize_small`），没有读器
  场景超过 0.19 %，所以两条地板是重测而不是放宽。
- **只有节点元数据能区分的键被拒绝** — 读者把带标记的键当整节点比较，连样式与锚点都算，而写器正是靠
  这些挑选：带标记键的普通与引号拼法都发 `!a "…"`，带锚点的 `Null` 发 `!a null`，于是自己的发射回来
  是 `duplicate key`（crash-4bf6ee37）。现在按标记与值跟踪，即 YAML 的标量等值，合并问同一个问题；一
  张按键的表读过 0.50 % 线。键位置更糟：crash-b88f1b4f 的两个映射键压在同一个被写器同样引成 `!? ""`
  的带标记空标量之上；现在问写器一个容器键拼成什么，两个读回相等就拒绝该源文档，姊妹 crash-469125e3
  是有拼法可学。立在最外层 `?` 之上的独立注释也不是键的差异（早于 #645）：`keys_collide_in_text` 丢
  掉每个独立槽位，行内注释才区分。投影按键的文本取键，所以 `c: 0` 挨着 `!A c: "!"` 发出 `c = 0` 与
  `c = "!"`，本项目的 TOML 读者拒绝、`json.loads` 折成一项；两个写器现在抛 `YamlSerializeError`
  （`toml-keys-collide-without-metadata`、`json-object-keys-collide-without-metadata`），
  `to_dict`/`safe_load` 抛点名该文本的 `YamlTypeError`；`to_yaml()` 不变。
- **重复的 JSON 键保住被它替换那个值内部的注释，而重复之间带注释的键留下最后一个值、不是倒数第二个**
  — 重复的名字保住第一个位置与最后一个值，所以较早那个值被整块丢掉，而救援只把被丢键与值节点上的注释
  交给幸存者 — `notes_of` 只读递给它的那个节点，于是一个被丢容器内部的注释随之消失，而事件流仍报告它
  （`json_events_replay` crash-e294755f：报告八条注释、保住五条）。`subtree_notes` 现在按源次序收集
  一个节点及其之下的一切；Ir 门在 `from_json_medium` 上读到 +0.09 %、`from_jsonc_medium` 上
  +0.07 %，两个夹具都不含这样的重复，所以这次移动是布局。闭合时的折叠另有原因：几个 `"a"` 键之一上的
  行内注释让那个键与其余不等，于是映射持有两项，下一个干净的 `"a"` 替换掉第一项的值，而折叠把带注释
  那项的旧值又搬回去 — `{"a": 1, /* n */ "a": 2, "a": 3}` 解析成 `{"a": 2}`。折叠现在终结于该名字的
  最后一项，`file_duplicate` 把最新的值停在那里，树行走与 `from_events` 共用这个函数。
- **JSON 折叠把元素第二组注释放进树所在的地方，而根的行尾注释被写出来了** — 一个元素的注释，它之上的
  与它之后的，都落进节点那个唯一的行内槽位：`array` 在 `value()` 之前取走待定列表、并把随后flush 的
  东西刷出去，于是第二次 `set_comment` 覆盖第一次，树携带一条注释而 `parse_stream` 报告两条
  （crash-0449c0ae，以及从嵌套数组抵达同一槽位的 crash-0466d7f6）；`crates/pyrs-json/src/parser.rs`
  里的 `apply` 现在按树行走 `attach_pending` 的方式安放第二次同行注释，把较早那条连同 `standalone`
  标志移到前置列表。折叠路由在自己的输入上撞同一次覆盖，归档为 `former-crash-e72e24ef.seed`。根的行
  尾注释先前根本没有写器：JSONC 与 JSON5 写器发出的每条注释都来自成员循环，而根不是任何成员的子节
  点，所以 `{"a": 1} // after` 原样写回 `{"a": 1}`，穿过 YAML 中枢同样丢失；四个入口现在都在顶层值之
  后发出它，`prop_jsonc_writer_carries_every_note` 与它的 JSON5 姊妹在此给跨格式层一个预言机。没有计
  数场景抵达 `from_events`。
- **JSON5：只看着像数字的字符串再次被引起来，而键的拒绝点名它的语法允许的一种拼法** — 裸令牌测试在数
  字检查之前剥掉一个前导正负号，然后问主体是否为严格 JSON 数字，于是 `+-33337333333373333333733333`
  被当成 `+` 加上一个可接受的 `-333…`，`to_json5_text` 发出自己的读者拒绝的文本
  （`expected integer digits in number`）；只有无符号主体之上的前导 `+` 才是 JSON5 拼法，于是这样的
  字符串再次被引起来，而 `+7` 与 `+1.5` 保住裸令牌（libFuzzer `json_roundtrip` crash-0a929a32）。在
  键位置上消息写着 `expected a quoted object key`，而 JSON5 也接受裸标识符，于是该错误要求方言从不要
  求的形式；措辞现在来自方言自己的标志 — 允许非引号键时是
  `expected an object key (a quoted string or a bare identifier)`，不允许时保留原句 — 一个 `Parser`
  服务三种方言，意味着这层共享的 helper 同时覆盖树与流两条路由，而严格 JSON 与 JSONC 保住它们的措
  辞。
- **TOML 标头注释：没人读的键行槽位，以及本是替代解释的那两叠** — `to_toml("sec: # note\n  x: 1\n")`
  返回 `[sec]\nx = 1\n`：注释被丢掉，因为小节那一遍只读映射自己的行内槽位，而中枢把 YAML 文档的键行
  注释交在键节点上 — 正是 #146 教写器往下一层读的那个槽位。这让本文给过的两个解释作废：`pyrs_toml`
  自己的扫描器确实捕获 `[sec] # note`，granit 也不在那条路径上；只有
  `api:set-comment-container-owner` 记录的落点规则仍开着。写 `key = value` 的那一行也没读标头的槽
  位，于是 `x: 1` 之上的 `b: # note` 发出 `b = {x = 1}` 而丢了注释；`emit_pair` 现在回退到该槽位。小
  节键与它带领的映射各持一个前置列表，那一遍先问小节的第一个、再回退到键的，于是携带 `# T` 的小节之
  上的 `# K` 只发出 `# T\n[a]\nx = 1\n`；现在两个都写。每一对都要付：执法镜像上 `to_toml_medium`
  +1.58 %、`to_toml_long_strings` +1.25 %，37 个场景里没有别的过 0.32 %。
- **五个没人读的 TOML 成员注释槽位，以及被写两次的元素注释** — 只要它需要的那一行不存在，`to_toml`
  就丢注释：携带自身注释的空表或空数组、注释独立成行的标量，以及一行同时应答两个 `or_else` 槽位注释
  的两处；新的 `prop_toml_writer_carries_every_note` 在两万次抽样里点名全部四种形状，它们取键之上的
  那一行。代价是守卫而不是注释：钉住的编译器上 `to_toml_medium` +3.29 %、`to_toml_long_strings`
  +3.43 %、`to_toml_notes` +1.24 %（相对不含 `#` 的文档），执法镜像上同样是 +1.256 %、+2.081 % 与
  +6.872 %，而两条 TOML 读地板动 +0.0014 % 与 −0.0007 % — 读路由从不查询那个槽位。上一层，中枢把成员
  上一行行尾的注释安在容器自己的前置列表上，而两个紧凑分支都忽略它 — 是 PR #146 的第五槽位类高一层；
  两个现在都通过一个 `container_leading_notes` 问那张列表，正如 PR #114 对表头那样，并降级，一种有些
  读者拒绝的拼法（2026-10-29）。第五槽位还写两次：读者把元素之上的 `#` 归档在该元素的行尾，而对行内
  表元素那也正是 `write_notated_table` 的前置注释，于是数组把它提升、表又在花括号里写一遍
  （crash-3e95a4cf、crash-6802dabe、crash-9c0b2acb、crash-1ec1ea89、crash-20cc6b69）。
- **点号路径像其它下降一样被限界** — `parse_inline_table` 把成员键路径铺在 `nest_inline` 里，于是
  `t = { a.b.… = 1 }` 每段嵌一张表、绕开 `parse_value` 漏斗与 `register_kv` 的检查：crash-2a5ea394
  就是 1059 段时这样，`from_toml` 接受、1.1 读者拒绝 `to_toml` 的再拼写
  （`max depth exceeded (max=1000)`），于是预算把铺出来的路径加到它之上展开的每条路径
  （`depth + inline_path_shift + key.len()`）。`DEFAULT_MAX_DEPTH` 只限界 `parse_value`，而点号键与
  表头用一条迭代指针下降建表、守卫从不计数，于是长路径建出更深的树并以两种方式失败：再拼写被拒，或者
  原生栈溢出（`0xC00000FD`；来自 2026-10-24 那轮的 crash-7a3aac52 在 2 MiB 线程上中止，只有 512 MiB
  才干净）。两条下降现在共用一个预算：至多 1000 段可往返并定居，1001 段以上是带类型的
  `max depth exceeded`（2026-11-01 那轮的 crash-2503e079、crash-6c15f693、crash-837bd790 正是该形
  状）。没有守卫时读路径只动 `load_toml_medium` +0.006 % 与 `load_toml_long_strings` +0.046 %，而看
  着像超过地板的 +1.65 % / +2.98 % 在父树上复现，是 WSL 宿主的基线漂移。
- **注册的处理器拥有它声称的那个标量，且在数组与数字协议之前被问** — #671 留下的形状现在归成一条规
  则：只带 `__bool__` 的类，`safe_dump` 发出 `m:\n  !booly B\n` 而 AST 路由发出 `m: !booly B\n`，因
  为直写器是经 `write_other` 到达注册表的，而它自己发缩进；同一个类在序列项里丢掉横杠行、作键时抛
  `YamlTypeError`。带 `numpy.ndarray` 的处理器时数组路由先来，在 AST 路由答 `!nd ND` 处答出数组。两
  个写器现在在每个位置都先问处理器，再问数组、协议与自己的拼写规则。该注册检查当初只为浮点形状写，于
  是 `python_type` 带 `__index__` 的 `register_type("!money", ...)` 被绕过：
  `safe_dump({"m": Money(500)})` 发出协议交回的整数 `m: 500` — `Decimal` 被尊重，整数形状的值类型从
  来没有 — 现在注册表先被问，只有 `int` 或 `int` 子类（`bool` 就是一个）保住读者能解析的整数拼法；
  `str` 处理器仍不动文本，`dict` 或 `list` 按调用方的容器来写；`type_registry::claims` 存在，是因为
  一个布局问题不该运行处理器。
- **转储 NumPy 数组不再终结进程（Python 3.15）** — 数组路由经 `rust-numpy` 强转对象并读它的内存，而
  `cp315-abi3t` wheel 无法可靠地做这件事：在已发布的 0.18.3 上，3.15 GIL 下
  `safe_dump(numpy.arange(8))` 以 `Fatal Python error: Segmentation fault`（退出 139）死掉，同样字节
  在 3.15t 下答 `YamlTypeError`，而 3.12、3.14、3.14t 转储正确。确定性、单线程，且
  `numpy.zeros((0,))` 也崩，所以故障坐在第一次读取数组内部。`crates/pyrs-yaml/src/py/ndarray.rs` 现
  在向数组自己要 `dtype`、`shape` 与 `tobytes()`，在 Rust 里解码那个缓冲区：任何制品都不再触及 numpy
  C API，而 `numpy` Cargo feature 门的是路由而不是依赖。两个缺陷随之倒下：旧路由读内存序、按逻辑形状
  嵌套，所以转置视图写出未转置的网格；两个写器都在数组路由之前问 Python 的数字协议，而单元素数组它会
  丢掉维度答出来 — 只有 numpy 2.3 起才算错 — 于是一台安装写 `1.5`、另一台写 `- 1.5`。携带维度的数组
  现在被拒绝该协议，0 维数组保住标量拼法。
- **注释不能再折断它自己的行，而块标量能拼出它携带的那个断行** — `arb_comment` 从 `[ -~]+` 抽注释文
  本，所以没有属性测试能到达单行注释载体欠下的那个展平；连 `\n` 与 `\r` 一起抽就判红四条属性，因为
  YAML 与 TOML 写器会把断行之后的文本放进文档 — TOML 拒绝自己的输出
  （`bare carriage return (must be part of a CRLF pair)`），序列项的注释 `"<\n&"` 回来成了裸 `&`。两
  个写器现在在一个 `push_note_text` 里把断行展平成一格空格；免分配的 `Cow` 形状仍要
  `serialize_note_flow` +8.80 %，所以发布的拼法由它移动的那条地板定价，62,066,033 → 64,411,980
  （+3.78 %）。块的那一半：生成器抽不带断行的可打印 ASCII — 而对 `Literal`/`Folded` 断行正是该样式唯
  一关心的东西；抽 `[ -~\n\t]+` 让 `needs_double_quoted` 判红，因为边界检查守了空格与制表符却没守断
  行，而非字符串捷径先跑，于是解析成 null 的 `"\n"` 被裸发出、回来成 `~`。最后一类是这层自己的：`|-`
  按设计丢掉尾随断行，所以这层现在按读者的方式先解析断言再比较；把简单键也解析之后，`"'\n"` 塌到旁边
  的裸 `'` 上。
- **一行文档的流式重新回到线性时间** — `LineTable::line_col` 为每个事件从行首数起，所以源里没有换行
  的生产者为每个事件付 O(n)、流式变成二次：1.5 MB 压缩后的 JSON 数组为报出 224,006 个事件花掉 4.9
  秒，输入每翻一倍就慢四倍。表现在把游标停在最后一次应答的位置，只走一个事件与下一个之间的字节，而落
  后于游标的位置仍从行首数；同一份文档流式用 19 ms，对照每记录一换行的孪生是 18 ms。JSON 与 TOML 共
  用这张表；YAML 自己算位置，从未有这个代价。
- **六条门禁能在什么都没量的情况下答"通过"，十条断言也能这样报绿** — 类别 `(fz)` 被扫清了：
  `check_changelog_coupling.py` 在空路径清单上退出 0，`check_doc_headings.py` 数的是交进来的路径而不
  是打开的文件，`check_line_endings.py` 从不检查 `git ls-files` 的退出码，`check_doc_signatures.py`
  与 `check_i18n.py` 在空的通配结果上打印 OK 行，而 `ir_gate.py --only a,b` — `--only` 是追加，所以
  那是一个匹配不到任何东西的名字 — 没有比较任何场景却把地板报成守住。六条现在都退出 2，各由一条测试
  逼出，而原本没有任何自测的 `check_i18n.py` 得到 `tests/test_i18n_gate.py`。同样的形状在套件里：断
  言只在 `for` 体内出现的测试在空可迭代对象上通过，而 `all(x for x in [])` 是 `True` — AST 行走点出
  41 个候选，可迭代对象来自测试之外的那十个现在带上写明实测数字的地板，其余 32 个是字面可迭代对象、
  别处已钉住的常量，或以空为断言本身的循环 — 记作 `(ge)`，因为这次扫查是一次性脚本，还没有东西拦住新
  的空循环测试。
- **一条守卫报的是跳过而不是测量，而排行榜有一行点了它从未计时的对手** — `load_stream_file` 的内存上
  限经 `pytest.importorskip` 要 `psutil`，而没有任何需求文件写这个包，于是每条腿都报一个看着像覆盖、
  其实不是的跳过。它被换成 stdlib `tracemalloc` 的一个标度不变量：2,000 行与 200,000 行文档走同一条
  路由必须留下同样的被跟踪足迹 — 3.12 下两处都是 3,168 字节、3.14t 下都是 3,200 — 而一条缓冲式的行走
  会涨 199,854,440 字节，于是把上限设在百分之一涨量上的断言能抓住累积读数器。排行榜里有同一个盲区：
  `tests/test_benchmark_crosslib.py` 会跳过吞不下夹具的对手，而 `yaml_edit` 那一行怪罪"较新的
  yaml_edit 发布"，实际对钉住的 0.1.3 三个夹具全都抛
  `Document::from_str: Input contains stream-level comments`，所以那一行是装好包装旁边的三个跳过。用
  一次限定范围的 `uv lock --upgrade-package` 取来 0.2.0 后它能解析三个中的两个，于是该行开始计时，而
  `test_yaml_edit_row_measures_something` 钉住它：装了就必须至少吞得下一个夹具。
- **钩子集是红的而判定是绿的，还有一条洞断言门禁读得太少** — `Test matrix (all legs)` 由 `ci.yml` 的
  作业算出，而 `toJSON(needs)` 带不来另一个 workflow 的作业，所以 `hygiene.yml` 里的
  `prek hook set on the full tree` 从来不是它读到的名字：PR #632 的第一个 head 上那个 check-run 报
  `failure` — 被 `check_changelog_coupling.py` 因变更集没有发布说明而拒 — 而同一 sha 上被要求的那个
  名字报 `success`。该作业现在跑在 `ci.yml` 里，`quality_matrix.py` 也从判定等待的那个作业读跳过清
  单。`scripts/check_hole_claims.py` 只读 `AGENTS.md` 与 `CLAUDE.md`，这就是一条过期断言在 #613
  与 #614 里、于所有门禁全绿之下活过八处的原因；范围现在是现在时 — 两份指令文件加每个镜像的
  `[Unreleased]` 块，由 `check_changelog_mirrors.py` 里新增的 `unreleased_lines` 切片，镜像切不出内
  容时退出 2。该门禁随后在 `3.15t` 腿上一次文件都没读就失败，因为 `_VERSION_HEADING_RE` 的
  `NameError` 定义在用法之上 95 行：清掉 `scripts/__pycache__` 前 20 次冷启动全红、暖启动全绿，所以
  触发条件是模块的第一次编译 — 补救是在调用之前把模块绑到局部名。
- **编辑地板为一个没有已发布调用方会跑的扫描付费，而刷新作业从第一个之后就把名字丢掉** — 三个
  `edit_*` 场景把 `line_offsets: None` 交给编辑层，而那层对 `None` 的答法是重建文档的行表
  （`scan_yaml(text).line_offsets`）— 一次完整遍历，还要决定 `has_hash`、`has_amp` 与 `is_ascii`：在
  基线的 `rustc 1.97.1` 上，这次扫描在三个循环里各花 10,104,000 条被计数指令，而绑定层是按文档记忆这
  张表的（`Splice::line_offsets`）。harness 现在在解析旁边建表、把 `Some(&offsets)` 交给三个动词、让
  `--setup-only` 把它减掉，执法镜像上第 81 次刷新读到 −24.45 %（`edit_set_nested` 48,288,771 →
  36,483,718）、−27.48 %（`edit_insert_sequence` 42,462,454 → 30,793,506）与 −27.86 %
  （`edit_delete_key` 42,392,088 → 30,583,030），超出扫描那 20.9 % 份额的部分是 `Vec` 的分配与释放流
  量。负责搬这些地板的作业丢名字：`profile` 与 `accept_drift` 用 `read -ra <<< "$VAR"` 切输入，而
  `read` 只吃一行，所以刷新 40221999337 拿到两行 `profile` 输入却记了 "attributing 1 scenario(s)"；
  两处现在都在切分前把分隔符压平。
- **fuzz 报表不再把发现扔掉，它的计数与开单器也能抵达队列** — 归档里有 `artifacts/` 与 `collected/`
  两个根；在运行 41541555317 上下载把这两个根直接解进 `--crashes`，于是把第一段路径当目标就造出两个
  名叫 `artifacts` 与 `collected` 的目标，一个 `yaml_roundtrip` 归档的三份输入被开成 issue #739
  与 #740，摘要还把每份输入数两遍。目标现在是任一根之后的那一段，摘要只读 `collected/`。报表步骤用
  `grep -c .` 计数却带着 `-eo pipefail` — 步骤自己的 `set -uo pipefail` 并不会关掉它 — 于是计数为零
  时退出 1 并结束该步：一个没有 crash 文件的归档留下只有表头的表格，下面的开单步骤从不运行，一次推送
  到 main 的发现到不了任何消费者；计数现在把零当成要报的值。第三个缺口是签名：
  `grep -m1 -hoE "panicked at [^:]+"` 停在第一个冒号，于是一次 panic 进报表时是
  `panicked at fuzz_targets/yaml_roundtrip.rs`，没有行、列或消息，定时层连续五个周六
  （2026-10-03、-10、-17、-24、-31）都是 `completed/failure`；签名现在保住位置与消息的第一行。
- **已发布文档：一个假的 `Node` 断言、格式化器删掉的一个字节、放错层级的小节** — `Node` 页面写道
  `is_valid()` 也拒绝被改过的文档，可它测的只是文档引用：一次编辑让节点过期后，它的 `repr` 翻成
  `Node(invalid)` 之后仍答 `True`，而读 `value` 抛 `YamlDocumentError`。同一个 `repr` 混淆三种起因：
  过期节点、缺失路径，以及被字典投影拒绝的路径，因为读都走 `to_dict()`：在 `0set: 1` 挨着
  `!base 0set: 2` 时每次 `Node` 读都抛 `YamlTypeError`，缺失路径是 `KeyError`；写行走 AST。四个
  locale 都带着它，现已改正。`rumdl fmt` 的 MD038 靠删字节来修"代码跨度元素内的空格"，于是一个写成反
  引号、空格、`? > #I` 的跨度在五个镜像与 ledger 里都丢了那个空格，而钩子打印 `Passed`：该前提只在
  CommonMark 渲染时剥掉的那对情形成立，只有前导空格是嵌套 YAML 标记体的缩进。`MD038` 进入
  `.rumdl.toml` 的关闭清单；2026-11-01 对 178 页跑 `rumdl check` 读到 0 条结论。本地化的 changelog
  页面把自己的小节放在版本标题同一层级，于是站点把每个都渲成它所在版本的兄弟；
  `check_changelog_mirrors.py` 现在检查这点。
- **已发布的计时表覆盖四种格式与由 Rust 支撑的读取器** — 测量表面早就在与 `json`、`orjson`、
  `tomllib`、`tomlkit` 与 `pyjson5` 竞跑，但 `.ci/perf-snapshot.json` 只记 PyYAML 与 ruamel.yaml。
  `scripts/bench_snapshot.py` 现在量一条格式轴 — YAML 的解析、序列化与往返，JSON、TOML 与 JSON5 的解
  析 — 并用 `<!-- bench: ... -->` 标记把每张表绑定到它自己的测量，而不是数列，
  `tests/test_perf_docs.py` 双向追问：量过却没发布的用例判红，点了没量东西的标记也判红。两行新数据是
  落后、也就按落后印 — `orjson` 读 1.9 KB 文档快 1.5x、`pyjson5` 读 JSON5 配置快 2.4x — 而 TOML 比
  `tomllib` 快 5.5x 到 5.9x；JSON、TOML 与 JSON5 的写仍不发布，因为它们来自引擎的 AST 而不是 Python
  对象。由 Rust 支撑的读取器在记录里哪都没量，于是 `yaml-rs` 与 `ryaml` 加入 YAML 轴、`rtoml` 加入
  TOML 轴；重测的快照（2026-10-25）说 `yaml-rs` 解析快 2.4–2.6x、往返快 1.6–1.9x，`rtoml` 读比
  `load_toml` 快 1.9x，而 pyrs-yaml 序列化比 `yaml-rs` 快 2.2–4.1x。

#### 变更

- **向只读单文档的路由要流现在被拒绝，条目之间的空行是被钉住的边界，而一次被移动的 TOML 注释点名正确
  的起因** — `parse`、`parse_file`、`safe_load`、`to_toml` 与 `YAML()` 的等价物只读一个文档，两文档
  输入回的是行走最后停在哪个：`a: 1\n---\nb: 2\n` 回来是 `b: 2`，`a: 1\n---\n` 是 `null`。现在抛
  `YamlParseError`，以 `multi-document-input` 开头并带上第二个文档的行列，指向 `parse_all_docs`、
  `safe_loads` 与 `parse_stream`。PyYAML 与 ruamel.yaml 在同样十种形状上抛，却仍接受前导 `---`、尾随
  `...`、裸 `---` 或空输入 — 守卫不可过度触发的对照。CLI 里 `-A/--all-docs` 说的是同一件事，未变；记
  作 `(hg)`。在八个位置上，并穿过编辑路由，空行一种都留不下，而值、键序、锚点与注释都留得下；
  `tests/test_roundtrip.py` 把那个不动点按今天的样子钉住，补上缺口会让这枚钉子判红。一次被移动的
  TOML `[sec] # note` 曾怪到 YAML 解析器的注释捕获模型头上，但
  `preserves_section_header_trailing_and_leading` 显出的是一个不动点：这次移动是本项目的规则 — 容器
  的行内注释停在它身体的最后一行。
- **每座桥两侧现在都有注释地板，读侧也算** — 数一遍夹具就否证了 PR #641 对写侧地板的归因：serialize
  场景跑在零个 `#` 的 YAML 上，五个写器输入零个 `{`、`[`、`]` 或 `}`，于是五个注释 helper 没有回答任
  何被量的问题，方言写器也拿到一棵零 `#` 的 AST。现在三份文档各自携带其半问的那个形状：
  `serialize_note_flow` 带一份穿过那五个答案，镜像把 12.61 % 归给这次行走；`to_jsonc_notes` 与
  `to_json5_notes` 让 `<str>::replace` 成为两者最大的自身项、占 16.08 % 与 16.01 %，在 PR #643 一次
  过拿走块的那一半之前；它们的读侧孪生在七条注释上把 `ws_extended` 排第一，占 19.86 % 与 19.79 %。对
  着零 `#` 夹具时，字节扫描与注释搬运跑在最便宜的答案上，所以 `parse_notes` 与 `stream_notes` 改读写
  侧那一行自己的文档：同样 434 字节、把注释刷成空格时，十一条注释在树路由上 +16.30 %、在事件路由上
  +19.37 %，其中 71 % 与 74 % 在被锁定的解析器里。注释离开夹具时 `ir_fixtures.rs` 按名字判红；那次刷
  新曾在 `from_json5_medium` +0.55 % 上拒绝自己，于是这个名字进了 `accept_drift`，而
  `tolerance_hint` 留在 0.5 %。
- **元数据搬运得到两条地板，数解释器的地板按 runner 镜像加键，而 TOML crate 写下它建的那棵树** — 每
  次替换都问 `merged_meta` 该搬什么，`delete_path` 问 `extend_delete_over_comments`，可没有任何地板
  抵达他们：`edit_*` 场景跑在 `MEDIUM_YAML` 上，不含 `#`、`&` 或 `!`，所以每个分支都落空。
  `edit_set_metadata` 42,838,349 对照小 26 个字符的文档上 `edit_set_nested` 的 36,483,730，那是搬运
  而不是更大的树，而 `ir_fixtures.rs` 在该夹具被装饰时判红。这一轮移动了两个它从不进入的地板 —
  `documents_short_strings` −10.46 %、`documents_long_strings` −7.00 % — 全在 glibc 的拷贝一行里，所
  以 `.ci/ir-baseline.json` 得到 `image_floors`，`scripts/ir_gate.py` 按运行所在镜像的那一行判定，且
  只对数 CPython 的场景键控：同一棵树、harness 摘要相同，`load_toml_medium` 读到 142,467,241 与
  144,904,012，一条地板兜不住的 1.71 % 差；没有对应行的镜像被拒绝（退出 2）。
  `crates/pyrs-toml/src/lib.rs` 开头写着"没有中间值树"，而 `(fw)` 把它的表插入定价为
  `load_toml_medium` 的 8.10 %、`from_toml_medium` 的 9.68 %：这次重写不在日程上，那句话现在描述它发
  的这棵 build。
- **事件流与行路由在长值形状上有了地板，而长正文在行路由上更贵** — `stream_long_strings` 用写器地板
  那八条 256 字符记录跑同一个 `Work::Stream`，所以这些数字只差在字符串的形状：276,633,813 对照
  `stream_medium` 的 255,359,778，**+8.33 %** — 门禁第一次在事件路径上量到长正文。`Work::Documents`
  经 `pyrs_json::parse_documents` 把八条一行记录读成事件，只差值的宽度、9 个字符对 256：95,815,186
  对照 328,951,144，**+243.3 %**，而同样的成对在事件路径上是 **+8.16 %**。重测移动了一个已发布比例 —
  事件成对最初写下的是 8.33 %、现在读 8.16 % — `QUALITY_MATRIX.md` §2 携带它，而
  `tests/test_ir_baseline_workflow.py` 拿文件而不是记忆核对。三十一个地板来自事件那次运行，三十三来
  自行那次。
- **长字符串形状在两条通道上都有测量，而表面覆盖全部五种格式** — 没有基准读过长值（`MEDIUM_JSON` 最
  长 21 个字符），于是向量化的扫描无人量过。`long_string_json` / `_toml` / `_yaml` 生成同一个列表、
  值换成 256 个纯字母字符，在每座桥两端读：排行榜通道四个基准（读 2.599 µs 与 3.499 µs，写 950 ns 与
  1.174 µs），执法通道四条地板，且按源字节算长文档是 `from_json_medium` 的五分之一、
  `from_toml_medium` 的八分之一。`scripts/quality_matrix.py` 把单向桥报成
  `ir-bridge-unidirectional`，于是方言桥端到端地跑 — `from_jsonc_long_strings`、
  `from_json5_long_strings`、`to_jsonc_long_strings`、`to_json5_long_strings`，每条散布 0.000 % — 并
  重测早先四条地板（`from_json_long_strings` 112,664,928 → 112,987,111、`from_toml_long_strings`
  109,254,466 → 109,535,451、`to_json_long_strings` 58,550,997 → 58,550,735、`to_toml_long_strings`
  72,438,982 → 72,438,675）。
- **harness 变大会移动变更从未进入的地板，所以名字来自一次派发的拒绝而不是预测** — 它先越过 15 条、
  再 13 条、再 18 条超过 0.005 %，而越过 0.50 % 线的名字每次增长都不同：`from_json_medium`
  78,671,707 → 79,283,878 → 79,904,977 每步 +0.78 % 越过两次，然后第三次增长把它退 0.41 %，而四个它
  从不进入的读器越过（`from_json5_medium` +1.05 %、`from_jsonc_medium` +0.72 %、`parse_inline_merge`
  +0.67 %、`from_json5_long_strings` +0.50 %）；第 74 次运行在这四个上拒绝，第 75 次命名它们并写下。
  随后的运行各需一个名字（`parse_inline_merge` +1.03 %，`stream_medium` +0.485 % 留在线内；随后
  `from_json5_long_strings` +0.82 %，`from_json5_medium` +0.479 % 与 `from_json_medium` −0.905 % 留
  在线内）；26 到 30 的增长谁也没越线，最大是 `from_toml_medium` +0.363 %。`stream_medium` 读
  +0.01 % 又 +0.63 %，只差一个变量，只有靠 `accept_drift stream_medium` 放行。`tolerance_hint` 仍是
  0.005；四十七条地板来自那些运行、由第 79 次重测，而在钉住的树上绑定通道相对镜像地板最差 +0.15 %，
  三个扫描器场景最多偏离 −21.23 %，所以 §2 的跨宿主上限被标成过期（ledger (gk)）。
- **每个 Rust 基准在自己进程里跑，两份 lockfile 都移到其范围允许的最新版本** —
  `codspeed-divan-compat` 5.0 → 6.0 为每个 Divan 基准 fork，于是一个基准留下的分配器状态不再染下一
  个，上游报告变化超过 1 % 的基准少约 70 %；`CODSPEED_PROCESS_ISOLATION=false` 可退回。没有任何决定
  合并的东西被移动，但跨过这次 bump 的 CodSpeed 趋势不是一次测量。`similar` 3.2.0 → 3.2.1 是八个
  crate 的 MSRV 1.96 地板之下唯一剩下的 Rust 步骤，九个 Python 包动了（`zensical` 0.0.71 → 0.0.72、
  `mkdocstrings` 1.0.7 → 1.0.8、`mkdocstrings-python` 2.0.9 → 2.0.10、`python-dateutil` 2.9.0.post0
  → 3.0.0、`deepmerge` 3.0.1 → 4.0.0，外加 griffelib、markdown 与两个补丁行）。`maturin` 留在
  1.15.0、`pyo3` 留在 0.29 线，因为最新的 `pytest`、`hypothesis`、`pydantic` 与 `numpy` 要 3.8 地板
  之上的解释器。实测：806 条 Rust 与 4,306 条 Python 测试、`-D warnings` 下的 clippy、整套钩子、一次
  严格的四 locale 文档构建。
- **`no_std` 宿主测试在 Windows 与 Ubuntu 上都跑，而单元测试那条腿跑它名字里写的那个配置** — 该作业
  钉在一个 runner 上，而 wheel 发向三个；与宿主无关的裸机构建仍留一个，单元测试现在摊开。那一步的注
  释说加宽 crate 列表 "换来 112 个从未在无 std 下跑过的宿主测试"，可 `cargo test -v --no-run` 显示六
  crate 的那次调用为 `pyrs-toml`、`pyrs_json` 与 `pyrs_ast` 携带 `--cfg feature="std"`，而单独的
  `-p pyrs-toml` 不带：Cargo 会把一次调用里的 feature 统一，而 `pyrs-yaml-core` 对这些兄弟的
  dev-dependency 边没写 `default-features = false`。发布的东西都没受影响，但那条断言靠的是一个量 std
  构建的步骤。三处清单边补上，`tests/test_no_std_gate.py` 现在读清单而不是信 `-p` 名单：它拒绝一条通
  向声明了 `default = ["std"]` 却没写 `default-features = false` 的兄弟的基础边，以修复前的清单作对
  照（正是那三条边），并拒绝退回单宿主的作业。无 std 的命令在 23 个目标上跑 724 条测试、Windows 上 0
  失败，而默认形状仍跑 815 条。
- **bump 机器人既不递出地板的编译器也不拼错自己的配置，两份文档写下量过什么** — 每周的
  `github-actions` 组开了 #620 抬两个 action，并把 `ci.yml` 里的 `dtolnay/rust-toolchain@1.96` 改写
  成 `@1.120` — 而那个 ref 正是 `msrv-check` 安装的编译器。五条测试判红，但没有 `ignore` 的
  `patterns: ["*"]` 保证每周再递一次；那个名字现在在 `.github/dependabot.yml` 里被排除，而
  `test_the_bump_bot_is_told_never_to_offer_the_floor_selector` 会在该排除被改掉时判红。三个
  `groups` 条目带着 `version-update:semver-patch`/`-minor`/`-major` — 那是 `ignore`/`allow` 的词表，
  因为 `groups.*.update-types` 只收 `major`/`minor`/`patch` — 于是 Dependabot 九天之后拒绝自己的文
  件，而作为一条合并门禁读不到的 check-run；五个值改用短形式，`tests/test_dependabot_config.py` 钉住
  两套词表。`QUALITY_MATRIX.md` 在八项的列表上写着 `Seven blind spots were found by measuring`；一条
  测试现在重推那个计数，自钩子集那条腿加入后读作九。三句 `docs/dev/perf.md` 把"称绑定通道嘈杂"记在
  `AGENTS.md` 名下（`grep -c noisy AGENTS.md` 是 0）；那个宽的读数是 CPython 每进程的哈希种子，现已
  钉住。

#### 性能

- **写器输出不再为中间分配付费：一次缓冲区容量、一趟注释展平** — JSON、JSONC、JSON5、TOML 与 YAML 写
  器各自从 `String::new()` 起，那七次涨到 512 字节的重新分配几乎就是一次配置大小写入的全部。在门禁钉
  住的 `rustc 1.97.1` 下数指令：`to_json_medium` 22,884,635 → 17,718,056（−22.6 %）、`to_toml_notes`
  16,468,905 → 12,071,715（−26.7 %）、`serialize_block` 14,518,712 → 10,899,301（−24.9 %）、
  `serialize_note_flow` 64,411,669 → 59,227,943（−8.1 %）— 十六个写器场景全降，最小的
  `to_toml_long_strings` −5.2 % — 而未变的树把 `to_json_medium` 的地板读到 +0.002 %。
  `edit_set_metadata` 动 −7.3 %，其余编辑场景在 −1.3 % 与持平之间，`safe_dump_*` / `to_python_*` 在
  0.28 % 内。块注释的展平原先分配两次，现在一趟写完，执法镜像上 `to_jsonc_notes` 与 `to_json5_notes`
  比 PR #642 提交的地板低 38.27 % 与 38.11 %，其余 45 个场景在 0.16 % 内（`to_json_medium`
  −0.0004 %）。经这一趟的六处行内形状在那里再买 2.5 个点，却把六个无注释写器推过 `ir_gate.py` 的容差
  （`to_jsonc_medium` +1.63 %、`to_json5_medium` +1.59 %、`to_jsonc_long_strings` +1.44 %、
  `to_json5_long_strings` +1.42 %、`to_json_medium` +1.02 %、`to_json_long_strings` +0.96 %）— 在够
  不着注释的地方 2.5 个点还不清 1 %，所以那一半被退回。没有一个输出字节改变。
- **读取器的字符串正文、TOML 的行与写器的转义扫描在 SSE2 上一次十六字节** — JSON 与 TOML 读取器逐字
  节扫字符串正文，TOML 每次调用走完整份源来拒绝裸回车；写器的转义问题在它的长值地板落地后加入
  （#600）。SSE2 在 x86_64 基线里 — 没有新依赖、没有 `std` — 字节循环是向量的尾巴，其它架构整条走老
  路。在宿主上：2 MB 带 5000 字节字符串的 JSON 2.47 → 1.97 ms，6 MB CJK 同样 6.17 → 5.27，JSON5 单引
  号 1.53 → 1.11，2 MB TOML 基本串 2.36 → 0.99、字面串 2.41 → 0.96、多行 3.37 → 1.01，0.45 MB 一百字
  节值 2.98 → 2.54；执法镜像上 `from_toml_medium` −4.02 %（125,580,997 → 120,534,055）、
  `from_jsonc_medium` −3.69 %、`from_json_medium` −3.55 %、`from_json5_medium` −1.09 %。转义问题量得
  `to_json_long_strings` 的 75.12 % 与 `to_toml_long_strings` 的 69.01 %，四条刷新后的地板读作
  58,550,735 → 22,038,711（−62.36 %）、72,438,675 → 28,254,711（−60.99 %）、60,056,647 → 23,544,666
  （−60.80 %）、60,480,707 → 23,968,721（−60.37 %）。短值是另一种形状 — 扫描占那些场景的
  25.95-26.61 % — 有一个付了钱：`to_json_medium` +0.52 %；`to_jsonc_medium` +0.48 % 与
  `to_json5_medium` +0.47 % 留在线内，`to_toml_medium` 下来 0.84 %，而 `parse_inline_merge`（一个
  YAML 场景，开发宿主上 +0.55 %）在镜像上 +0.00 %。
- **一行 JSON 一次八个字节地建索引** — `pyrs_event::LineTable` 一步一步各数一个 UTF-8 字符，既用来找
  行起点、也用来在事件之间搬游标，而在执法那条线的镜像上这些行走读掉 `documents_long_strings` 的
  51.79 % 与 `documents_short_strings` 的 21.09 %（`docs/dev/quality-ledger.md` (gl)）。一个 `u64`
  一次答八个字节：`zero_lanes` 找断点，而每个候选道要与它声称的那个字节核对，因为零道里借出的位在后
  面让一个 `0x01` 看着也像断点 — 新的属性测试在出厂前抓到这点。在一台宿主上：
  `documents_long_strings` 308,642,512 → 126,840,512（−58.84 %），`line_col` 降 82.6 %、
  `LineTable::new` 降 66.0 %，`documents_short_strings` 75,506,559 → 64,952,559（−13.98 %）。位置不
  变，地板随代码在执法镜像上一起移动：`documents_long_strings` 328,990,934 → 147,188,912
  （−55.26 %）、`documents_short_strings` 95,854,968 → 85,300,932（−11.01 %），外加两个本地 profile
  从没点名的场景，因为同一个索引服务整条 JSON 流路由 — `stream_json_medium` −14.90 % 与
  `stream_json_stop` −27.17 %。其余 38 个动 0.0051 % 或更少。
- **`load_toml` 不再建它留不住的那棵树，而 `load_json5` 走它的两个兄弟早就有的免 AST 路由** — TOML
  文本走扫描、表树、`CustomNode`、Python，于是一个普通 dict 为一棵保格式节点的模型付费；
  `pyrs_toml::read_toml` 现在把 IR 走进调用方的 `TomlSink`，其标量来自共享的 `scalar_projection`，而
  守卫是标记注册表：任何处理器装着时的读走 AST 路由。对 `rtoml` 在同一轮里，已发布那行从落后 1.92x
  与 1.91x 变成落后 1.47x 与 1.46x，约快 1.31x — 宿主在两份快照之间漂了约 20 %，所以绝对读数不是证据
  — 而在进程内对照被守卫的路由读作 1.15-1.34x 与 1.31-1.35x。`json_fast::try_load` 只接进
  `load_json` 与 `load_jsonc`，于是 `load_json5` 付整趟 `CustomNode` 往返：同一条 123 B 上 2.063 µs
  对照 `load_json` 的 0.473 µs。JSON5 是严格 JSON 的超集，所以快路径接受的是一份同值的合法文档、方言
  新加的一切都退回 AST 路径；值不变，由 712 条测试对照 `json.loads` 与 `pyjson5` 判定。对照加载器自
  己的方言路由它在 300 项上赢 6.25x、在 1,200 项上 9.34x，各 5/5 成对 — 那条比例现在由
  `tests/test_json_leaderboard.py` 把门，因为两条路由都不被指令计数定价。
- **加载路由、三个编辑动词与流停止都有指令地板** — 没有 Ir harness 进入它们：`from_toml_*` 停在树
  上、`from_json_*` 停在快读器绕开的那个节点模型上，而绑定通道只有 `bench_to_python` 与
  `bench_load_toml`。两个场景经一个 feature 门控的接缝驱动 `load_toml`，读作 `from_toml_medium` 与
  `from_toml_long_strings` 的 1.20x 与 1.29x — 那个没有任何计数数字描述过的 Python 一半；
  `load_json_medium` 与 `load_json_long_strings` 把 JSON 快路由钉在 `from_json_medium` 与
  `from_json_long_strings` 的 0.74x 与 0.86x，这条路由是替换那棵树而不是给它加东西；`load_jsonc` 与
  `load_json5` 共用该行。三个编辑场景加入，每个动词一条、跑在 `MEDIUM_YAML` 上，克隆与解析放在 setup
  里，剩下的就是动词；`move_path` 的两半被定价。`stream_json_medium` 对照 `stream_json_stop` 把停止
  放在行走的 25.09 % — `Break` 结束的是扫描，不只是交付；TOML 没有这样一行，它的 `Break` 结束发出。
  基线从 33 个场景长到 35，这些行移动了兄弟：三条 `to_python_*` +0.212 %、−0.357 % 与 −0.111 %，
  `to_python_anchors` +0.1343 %，`load_toml_long_strings` −0.2175 %，六个 JSON 读器 +0.73 %…+1.91 %
  而 diff 里没有引擎文件，`load_json_long_strings` +0.76 % 来自 runner 而不是代码 — 进了
  `accept_drift` 的名字，`tolerance_hint` 仍是 0.5 %。记作 `(fs)`、`(ft)`、`(gf)`、`(gi)`、`(gh)`。
- **门禁最宽的读数是解释器，现在被钉住了** — 三次刷新运行在同一个二进制摘要（`7adbeac719edacd6`）下
  记录绑定 harness，并把 `load_json_long_strings` 在九次抽样间散布 1.121 %，而同样字节上的引擎场景一
  致到 0.00001 % 以内。只有一个 harness 链接 CPython，而那个项是它每进程抽的哈希种子：在
  `PYTHONHASHSEED=0` 下该场景三次都读 95,136,157，七条绑定场景全部收敛（0.2811 %、0.2550 %、
  0.1249 %、0.0787 % 降到 0.0000-0.0004 %）。`scripts/ir_gate.py` 把种子与 glibc 上限一起钉住 — 就是
  关掉 1.44 % 跨宿主项的那个补救 — 而这一轮刷新把所有 42 个场景报在自己 0.0010 % 之内。地板只在种子
  先前替代码干活的地方移动：`load_json_long_strings` −0.526 %、另外五个加载器 −0.166 % 到 −0.055 %、
  `to_python_medium` +0.051 %，35 个引擎场景 0.001 % 或更少，向上没有任何一条过线。线留在 0.5 %，因
  为仪器不再需要那点余量，而 `tests/test_ir_gate_provenance.py` 会在键丢失时点名它。
- **空的块体写出它声明的 chomping，于是只差在这里的两个容器键仍是两个键** — 映射等值读 `chomping`，
  所以读者把 `? hF&a?k !tag o: |+1` 与 `? hF&a?k !tag o: |` 各留一份；写器对每个空体强制 `Clip`、两
  者都拼成 `|`，而它自己的读者以 `DuplicateKey` 拒绝这次发射（libFuzzer `yaml_roundtrip`
  crash-469125e3，最小化到 48 字节）。`effective_chomping` 现在对空值给出声明的 chomping，两个块写器
  把标头行留给调用方的终止符而不开一行体 — 两半都吃重，而一次突变说它们是两条断言：恢复 `Clip` 会让
  `container_keys_differing_only_in_chomping_survive` 带着读者自己的 "duplicate key" 判红，而把换行
  放回标头会让那条保持绿、让 `empty_block_header_reaches_a_fixed_point` 带着期望 `">+\n"` 却得
  `">+\n\n"` 判红。旧规则归一化过的空 `Keep` 种子（`>+8\r#`、`ancho: |+`；crash-89d81d99、
  crash-b5dcc38f）在第一次发射上定居，而输入归档为
  `fuzz/seeds/yaml_roundtrip/former-crash-469125e3.seed`。

</details>

### [v0.18.3] — 2026-10-22

<details markdown="1">
<summary>会读代码树的门禁 · crate 矩阵 · 注释归属 · 更宽的 wheel 矩阵</summary>

#### 新增

- **设计文档里的已落地声明要指认代码** — `docs/dev/event-stream.md` 曾在只包含 docs-only 提交
  `0a074148` 的主干上写着 "decision 6 landed"，而没有任何检查把设计文档的现在时与代码对账。
  `scripts/check_landed_claims.py` 现在要求声称 landed 或 shipped 的标题在下一行写出让这句话为真的文
  件（可选 `::标识符`）；它作为 `landed-claims` prek 钩子运行，并由
  `tests/test_landed_claims_gate.py` 把关。

- **被取消的 CI 腿会自动重跑** — `.github/workflows/ci-rerun-cancelled.yml` 在 CI 工作流于某 pull
  request 结束时通知我，并经由 `scripts/ci_rerun_gate.py` 只在「某条腿被 `cancelled`、其余腿皆绿、且
  没有 `failure`」时重跑，且不超过尝试上限以防循环。此前被 `cancel-in-progress` 取代或运行器回收而
  取消的腿，会把必需的 `Test matrix (all legs)` 结论判红——原因却与本次改动无关——要人工按下 Re-run
  才能合并。结论的 accept-set 未动：`cancelled` 仍拒绝合并，所以这是加一次重试，而非开一个例外。
  `tests/test_ci_rerun_gate.py` 双向钉住该决策，并把接线当作契约。

- **YAML 引擎现在可以面向没有标准库的目标构建** — `pyrs-yaml-core` 现在是 `#![no_std]`：
  解析器、往返序列化器、保留注释的编辑层和 splice 只依赖 `core` + `alloc`，其下的
  `granit-parser` 本来就是不带 std 构建的。`std` feature 默认仍然开启，因此 Python 绑定、CLI 和
  各基准测试保持原有 API；它负责加上三面需要操作系统而非引擎本身的表面：翻译过的错误消息目录
  （rust-i18n 持有一个进程级全局 locale）、运行时 schema 注册表（一个全局 `RwLock`）和基于
  regex 的用户 schema 语言。`no_std` 调用方改为直接构造 `pyrs_schema::types::Schema` 变体来选择
  schema。`ci.yml` 的三个 `no-std-check` 步骤现在为 `thumbv7em-none-eabi` 构建该 crate 并在宿主
  上运行其单元测试；`tests/test_no_std_gate.py` 从声明 `#![no_std]` 的 crate 根读出基础集合，
  于是「加入了 no_std 却没加入门禁」的 crate 会得到一次红灯，而不是一个无声的缺口。`no_std`
  构建把「已经见过的标量键」记在 crate 的有序节点映射里而不是 `HashSet` 里，因为 `alloc` 不带
  哈希器：`parse_medium` 的分配上限是 74 次分配，而 std 构建是 67 次——每个映射多一次——
  `tests/alloc_gate.rs` 按 feature 集分别记录这两个上限。

- **no_std 的故事改为与源码实际声明的 crate 对账** — foundation 集合是从写出 `#![no_std]` 的 crate
  根读出来的，`tests/test_no_std_gate.py` 现在把它与另外两样东西比较：`ci.yml` 里的三个
  `no-std-check` 步骤，以及现行文档里点名的 crate。`AGENTS.md` 补上了它一直缺的那个 `no_std`
  foundation 故事；而 `docs/dev/boundaries.md` 里指向它的那一段，在 YAML 引擎加入这个集合之后仍写着
  五个 crate，本该说出这件事的门禁当时只是一句注释。有日期的记录不在此列：`CHANGELOG.md`、它的四份
  镜像和 `docs/dev/quality-ledger.md` 都是历史，历史可以写五个 crate 的 foundation，因为写下它们的
  时候事实如此。

- **crate 矩阵由测量维持，而不是靠叙述** —
  `scripts/check_rust_python_decoupling.py` 从各 manifest 自身推导矩阵，并强制四条规则：恰好只有
  一个 crate 是 cdylib（扩展模块本身）；其他任何 crate 的依赖闭包都不携带 Python 运行时 crate；
  扩展 crate 不持有 Rust 测试层，并声明 `[lib] test = false`；每个 `required-features` 条目都指向
  一个已声明且非默认的 feature。该门禁自带严格的 TOML 读取器，既不需要 `cargo` 也不需要
  `tomllib`，因此可以在与本包相同的 Python 3.8 下限上运行——包括没有 cargo 的 Python 测试腿。
  `tests/test_rust_python_decoupling_gate.py` 是它的反证控制：每条规则都在合成树上被破坏，并要求
  判定随之翻转；读取器与本仓库每个 manifest 的 `tomllib` 结果逐一对照；读取器读不懂的 manifest
  必须以退出码 2 结束，而不是算作通过。`crates/pyrs-yaml/Cargo.toml` 现在声明 `test = false`，
  cargo 因此不再为扩展模块构建测试目标；`prek.toml` 里的钩子会在每次触及 manifest 或 crate 的
  改动上运行这道门禁。

#### 变更

- **事件流中的注释会说明它属于哪个节点** — `pyrs-event` 的 `StreamEventType::Comment` 原本只带一个
  标志位 `standalone`，而它同时回答了两个问题：这条注释是否独占一行（源码的事实），以及这条注释
  被归档在哪个节点上（树的事实）。想把注释放回树归档它的位置的消费方无法问出第二个问题，于是这个
  标志位现在拆成了两个。`owns_line` 保留源码自己的答案，新增的 `NoteOwner` 枚举承载树的答案：
  `FollowingLeading`、`FollowingInline`、`PrecedingLeading` 与 `PrecedingInline`，按消费方的构建
  顺序命名——`following_*` 是消费方接下来要构建的节点，`preceding_*` 是它刚构建完的那个节点。直接
  构造 `StreamEventType::Comment` 的 Rust 调用方都在这次改动里一并更新，而这些 crate 并未发布，
  所以已经交付的 API 没有移动；`parse_stream` 的 Python 调用方看到的 `style` 键
  （`standalone` / `inline`）保持不变，另外多出一个 `owner` 键，用同样的四种拼写命名。三个生产者
  都从自己归档注释的那个节点出发回答新问题，于是事件流与树不可能不一致：YAML 生产者把拖在某个节点
  之后的注释放进该节点的行内槽，把引领下一个节点的注释放进它的前置列表；TOML 生产者在表名之后才
  报告表自己的注释，因为 `[a]` 上方的注释引领的是 `a` 持有的映射而不是表名；JSON 生产者按树自己的
  规则报告每组注释的方向与槽位。由此带来两处可见变化：写在逗号之后的 TOML 注释现在报告
  `style: "inline"`，而此前报告 `standalone`，因为那条注释并没有独占一行，由 `owner` 来说明它属于
  哪里；TOML 表的注释现在排在它的键事件之后，而不是之前。JSON 生产者也不再为注释的偏移重新扫描
  源码——偏移随记录一起传递——于是事件遍历不再第二次扫描文本。这一拆分由同时向树和事件流提问的
  测试层证明：`crates/pyrs-yaml-core/tests/event_notes.rs` 对十四个文档各走两遍，断言树归档的每条
  注释都以相同的槽位、相同的方向和相同的独占行事实被报告；JSON 与 TOML 的重放套件比较整条注释，
  而不是跳过其中一个字段；一份 JSONC 文档从自己的事件折叠回来后，经过严格写入器往返得到同一段文本。

- **`rules:` 模式语言首次接受外部裁判** — `tests/test_parse_schema_spec_table.py` 就语法所接受的每个
  `type:` 名称，向本仓库并不拥有的东西提问：某条规则解析出的值，是否真是该名称所指代的 JSON Schema
  类型的实例。这一成员关系由 `jsonschema` 裁定，而它对 `boolean` 与 `integer` 的互斥——尽管 Python 的
  `bool` 是 `int` 的子类——意味着标错类型的规则是一次红跑，而非一次附议；每一行都附带一个同一值必须不
  满足的诱饵类型，使这份确认无法沦为橡皮图章。`Schema rule language` 行的 `external authority` 单元
  格由 `no` 变为 `yes`，`authority-tier:schema-rule-language` 缺口被撤销登记，
  `.ci/quality-holes.json` 重新为空。`scripts/quality_matrix.py` 把 `jsonschema` 纳入其 oracle 集
  合，于是这份认定来自测试实际调用的读取器，而非文件名后缀。

- **解耦门禁学会看守 bench 腿** — `scripts/check_rust_python_decoupling.py` 新增规则 R5：扩展 crate
  不得持有不带 `required-features` 的 `[[bench]]` 目标，因为这种目标会被每条 workspace 级 bench
  命令选中，进而重建 CI bench 腿刚刚排除的 `pyo3-build-config` 解释器探测。门禁原有四条规则；这
  第五条锁住了 bench 腿作用域化的成果，使 `crates/pyrs-yaml` 里将来若出现无条件 bench 会变成红
  跑，而非耦合的悄然回潮。`tests/test_rust_python_decoupling_gate.py` 在合成树上打破 R5 并要求
  结论翻转——反证控制现为 13 条——而干净工作副本仍通过。

- **CI 的 Rust 基准腿也不再需要解释器** — CodSpeed 的 `Rust benchmarks` 是最后一条把 Python 运行时拖
  入构建的腿，成因与测试腿相同：靠的是 workspace 的*成员身份*，而非依赖边。`cargo codspeed build` 会
  选中每个成员，于是编译了 `pyrs-yaml` cdylib 与探测解释器的 `pyo3-build-config` 构建脚本，尽管
  `yaml_bench`（在 `pyrs-yaml-core`）与 `cli_bench`（在 `pyrs-yaml-cli`）都不依赖绑定 crate ——
  `cargo tree -i pyrs-yaml` 不点名任何 workspace 成员。该腿现改用
  `cargo codspeed build --workspace --exclude pyrs-yaml`，即测试腿早已使用的排除，并删去其
  `setup-python` 步骤，因为作业里再无探测解释器之物。在本树实测：作用域化后仍选中 `yaml_bench` 与
  `cli_bench`，且 `pyo3`/`numpy` 编译数为零。`ir_gate` 目标不受影响 —— 它由 `ir-gate` 门控，从不是
  CodSpeed 基准，只在 `scripts/ir_gate.py` 需要启动 CPython 时显式构建。

- **Rust 基准测试住在它所测量的 crate 里** — divan 基准测试集从
  `crates/pyrs-yaml/benches/yaml_bench.rs` 移到了 `crates/pyrs-yaml-core/benches/yaml_bench.rs`。
  这四十七个基准测试测量的代码与此前完全相同：绑定 crate 只是重新导出了引擎的 API，因此每一处
  import 只是换了名字，别的一概没动。基准的 package 闭包丢掉了它从绑定 crate 继承来的六个
  Python 运行时 crate（`pyo3`、`pyo3-ffi`、`pyo3-macros`、`pyo3-macros-backend`、
  `pyo3-build-config` 和 `numpy`），一个也不带。基准二进制本来就与 Python 运行时无关——Linux 的
  `DT_NEEDED` 里没有 `libpython`，Windows 的导入表里没有 `python3.dll`——但那只是因为链接器剪掉了
  一个没有任何符号引用的依赖；现在依赖本身没有了，于是这份独立性是结构性的，门禁可以断言它。
  扩展模块保留着它自己对 `libpython3.12.so.1.0` 的链接，那正是一个 Python 扩展该待的地方。
  被测量的代码没有变，所以看得见的差别只有 CodSpeed 身份里的 package 段：基准条目在
  `pyrs-yaml-core` 之下重新建立基线。

- **事件流和 merge 解析器把锚点留在 crate 的节点映射里** — `#![no_std]` 的 crate 无法书写
  `std::collections::HashMap`，于是 `event_to_stream_event` 和 `resolve_anchor_name` 现在接收
  `&mut NodeMap<usize, String>`，merge 解析器的锚点表也是 `NodeMap<String, _>`：就是 AST 早已
  使用的那个 `FxHasher` 之上的 `IndexMap`，因此锚点的顺序是插入顺序，按键查找也实测更便宜。在
  指令计数门禁的 YAML 场景上，本树移动了 `parse_anchors` -2.36%、`parse_inline_merge` -2.32%，
  `parse_medium` -0.15%、`parse_small` -0.21%。已提交的下限没有改动，因此门禁在这两个场景上
  现在容忍的余量在「更快」的方向上已经过期，等在执行镜像上做一次刷新。

- **Ir 门禁打印编译器自己的诊断，而不是 cargo 的摘要** — harness 构建失败时，此前只报出
  `error: could not compile pyrs-yaml (lib) due to 1 previous error`，再无下文。门禁以
  `--message-format=json` 构建，rustc 的诊断因此被移到 stdout 上、以 `compiler-message` 记录的形式
  出现，而它只打印了 `res.stderr`；于是被藏起来的正是门禁最该解释的一类失败——
  `rust-lld: unable to find library -lpython3.12`——只能靠手工重跑 cargo 才看见。
  `build_diagnostics()` 现在汇总构建流中每一条 rendered 诊断，仅当 stdout 一条都没有时才回退到
  stderr 末尾；后者是缺工具链、坏 manifest 的情形，而不是编译错误。
  `tests/test_ir_gate_diagnostics.py` 钉住两条流与回退路径：把实现改回只读 stderr 的突变让三个
  测试中的两个变红。

- **标准 JSON Schema 面从此由官方语料裁定，并拥有独立的一行** — `Document.validate()` 把求值交给
  Python 的 `jsonschema`，本仓库只保留 AST 到实例的转换、schema 接受与 `YamlValidateError` 契约，
  因此矩阵原有的 `schema validation` 行测的是*另一个*面：`validate_against_schema()` 求值的、本仓库
  自创的规则语言。新增的 `JSON Schema validation` 行由 `tests/test_document_validate_suite.py`
  回放官方 JSON-Schema-Test-Suite（提交 `0b4a7fa`，46 个文件、1302 个用例）记为 `yes`。在
  `jsonschema` 4.26.0 上实测：1261 个用例到达语料预期的结论；`refRemote.json` 整文件按契约边界排除
  （接口不暴露解析器，只探测一个代表用例以保持排除有效），另有十个 `dynamicRef`/`vocabulary`/
  `\p{...}` 用例被记为被委托引擎的边界。同一语料直接经 `jsonschema.validate()` 回放，在全部 1302
  个用例上结论一致，因此没有任何排除可归因于转换层。排除集按已安装版本键控——Python 3.8 腿解析到
  4.23.0，需要多排除一个 `unevaluatedProperties` 用例，4.25.1 已修复——且双向断言：被命名的用例
  开始通过会失败，未被命名却开始失败同样会失败。自创语言保留其登记洞
  `authority-tier:schema-validation`，不借用该语料的信用；其出口现在写明：以
  `Document.validate()` 取代它即关闭该洞。

- **`rules:` 规则语言重新拥有自己的一行，它的盲区也重新有了名字** — 矩阵的 schema 段落在
  2026-10-21 退役校验器之后仍写着规则语言的「Rust 入口点已被删除」，但*语言*本身还在：
  `parse_schema_yaml()`、`RuleResolver` 与 `YamlTypeKind` 仍由 `pyrs-yaml-core` 导出，
  `register_schema()`、`load_schema()`、`list_schemas()` 和 `YAML(schema=...)` 仍经它读取
  `rules:` 块。它在表里根本没有一行，于是页面声称四个表面行而实际交付五个，注册表写着
  `holes: []`，同时一门任何外部读取器都无法反驳的语法只被它自己作者的测试度量。这一行现在
  是推导出来的而不是手写的：`scripts/quality_matrix.py` 把某一行的单元层与属性层限定到声明该
  表面的那个文件，于是这门语言记的是
  `crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs` 里的十一个 `#[test]`，以及唯一
  点名它的属性 `prop_schema_parse_no_panic`，而不是 YAML 引擎整个 crate 的计数。它的往返与
  parity 两格读作 `n/a`：它只解析标量类型、不拥有序列化器，而这门语法也没有第二个实现可作
  比较。authority 格读作 `no`，这是诚实的答案：语法是本仓库自创的，于是单元、属性与 fuzz
  三层都只在问引擎自己相信什么。度量推导出洞 `authority-tier:schema-rule-language`，注册表
  连同可检查的出口一并收下，`QUALITY_MATRIX.md` 在盲区表里点名它——于是注册表重新有了一条
  记录，并且新增的第二条证伪控制断言被退役的校验器符号确实不在、而幸存的语言确实还在，
  页面无法再向「已删除」的方向漂移。

- **标量路径上的 `sort_keys` 是有文档说明的空操作，而不是静默成功** — `sort_keys("$.a")` 返回 `None`
  且不改动任何内容，而 `sort_keys("$.missing")` 抛出 `YamlEditError`，包所交付的文档对此只字未提；登
  记条目记录了这一不一致，出口是「与缺失路径同样回答，或在随包发布的桩中说明这是空操作」。现在桩里写
  明了：解析到标量的路径没有键可排序，调用返回 `None`；解析不到的路径仍然拒绝。钉住该形状的测试已按
  契约命名并断言两侧，`api:sort-keys-scalar-path-noop` 从 `.ci/known-residuals.json` 移除，矩阵的
  Known residuals 表如今只列出 `api:set-comment-container-owner`。

- **无界读取器不再让 parity 列跳过测试单元** — 此前 `crosslib_parity` 在读取任何测试之前，只要
  判断出没有读取器被 marker 限界就直接返回 `yes`，于是每个测试平台都能装上的读取器家族仅凭
  import 列表就被记账：所有比较都带 `@pytest.mark.skip` 的 `test_*_crosslib.py`，以及一个测试
  都没有的文件，都读成 `yes`。现在每个模块都要过单元规则。YAML 仍读 `yes`，因为它的守卫不会
  触发——每个比较都带 `skipif(pyyaml is None)` 或 `skipif(not _ruamel_available())`，两个读取器
  都无 marker 声明，辅助函数按函数体（`return _ruamel is not None`）解析——而
  `skipif(sys.version_info < ...)` 仍然拒绝记账。要从 `safe_load`、`safe_dump` 和 `parse` 读出
  YAML 的比较，方言正则需要认识这些无后缀的默认动词。控制项把四种“没有活比较”的形态
  （`@pytest.mark.skip`、无测试、模块级 `pytestmark`、解释器门）钉为 `partial`，把死掉的可用性
  守卫钉为 `yes`；实测树不变——YAML、JSON、TOML 的 parity 仍为 `yes`，派生洞列表仍是唯一的
  `authority-tier:schema-validation`。

- **语料套件由它命名的语料记账** — 外部权威列的无读取器分支此前会给任何没有导入读取器的
  `tests/test_*_suite.py` 或 `*_spec_table.py` 记 `yes`，于是一个只有正确后缀和占位测试的文件也
  显示为 `yes`，而文档写的是语料文件按它重放的套件计数。现在该分支要求模块以字符串常量命名
  语料——三个套件交给加载器的 `Reference/` 根路径——只是出现在注释里不算加载。控制项钉住两侧：
  仅有后缀的文件、只在注释里提到语料的文件读作 `no`，真正的语料套件仍读 `yes`，且已测量的树
  不变——`test_yaml_suite.py` 是唯一走到该分支的文件，七个权威文件全部为 `yes`，四个格式行
  仍为 `authority=yes`，派生的洞列表仍只有 `authority-tier:schema-validation`。

- **外部权威列不再为一条 import 记账** — 只要 `tests/test_*_suite.py` 或 `*_spec_table.py`
  导入了不受任何标记约束的读取器，该文件就记为 `yes`，无论其中是否还有代码真正用到它。单元格
  问的是依赖而不是测试，因此紧挨着一条看起来仍在使用的 import 删掉比较，这一列仍会保持绿色。现在
  文件只有「伸手进」那个读取器才能获得读取器记账：测试中的一次调用，或在任何测试运行前接入模块级
  表的可调用对象——写入器套件用的正是这种形状。`json is None` 是探测，不是比较；无人使用的
  import 两者都不是。有界分支保留更严格的规则，因为那里的一个跳过可能正是把读取器从某条腿上移除
  的那一个；语料套件仍然不需要读取器，因为语料没有可缺席的读取器。`tests/test_yaml_suite.py`
  曾是唯一靠裸 import 读成 `yes` 的文件：`import yaml` 仅用于派生其门禁所用的 `HAS_PYYAML`
  标志，而 PyYAML 是在 `pyrs_yaml.compliance` 内部加载语料的，因此该标志现在来自那个模块，文件
  由它运行的套件记账。对照用例钉住全部四种形状——仅导入与 `json is None` 探测读作 `no`，模块级
  读取器表与语料套件读作 `yes`——实测树不变：七个权威文件全部读作 `yes`，四个格式行仍为
  `authority=yes`，派生洞列表仍只有 `authority-tier:schema-validation`。

- **最后一个对拍洞闭合：JSON5 与 JSONC 在每条腿上都有参照实现** — 被登记的条目点名了两个方言：
  它们唯一的外部读取器是 `pyjson5`（声明为 `python_version >= '3.9' and python_version < '3.14'`，
  并排除 armv7l/s390x/ppc64le），因此在本包承诺的 3.8 地板以及 3.14+ 上，两者都被拿去和「空」比较，
  而腿仍然报绿。纯 Python 的 `json5` 包（`>= 0.17.3`，`requires-python >= 3.8`，`py3-none-any`
  wheel）现已加入 test 依赖组，并被打分该方言的两个文件无条件导入；判定该方言的比较不携带任何跳过
  标记，`pyjson5` 则在其 wheel 可安装的腿上保留为第二意见。两个参照实现有两处分歧，测试明确钉
  住本引擎的读法，而不是掩盖差异：代理对在这里合并为一个字符，纯 Python 参照保留两个 UTF-16 码元；
  孤立代理在这里被拒绝，而该参照接受它。`tests/test_json5_spec_table.py` 不再用 `pyjson5` 把整个
  模块门控住，因此规范表的行在地板上不再跳过。实测：`cross_library.restricted` 为空，JSON 行的
  parity 单元读作 `yes`，`.ci/quality-holes.json` 只剩 schema 一行；external-authority 列现在按
  文件给出判定，而不是按文件名记账——退回为只依赖受界读取器的规范表会被发射为 `authority-tier`
  类别、以文件命名的洞。

- **TOML 三方一致如今在每个腿上运行，而不只在 3.11 以上** — 跨库模块曾在 3.8 到 3.10 上跳过它那十一
  条 pyrs/tomlkit/tomllib 对拍，因为 `tomllib` 随 3.11 才到，而 3.11 以下没有安装任何参考读取器。回
  移包 `tomli>=2.0` 现在以 `python_version < '3.11'` 声明在 test 组，并以 `tomllib` 这个名字导入，于
  是每个腿都跑满 24 条对拍；行里分别列出 `tomli` 与 `tomllib` 及各自的边界，`tomlkit` 仍在每个腿上判
  定该方言，结论保持 `yes`。读取器搜索也止于发行版名的末尾：`_dependency_marker("tomli")` 曾答出
  docs 组的 `tomli-w` 需求，报告的 marker 属于没有任何测试导入的包。新增控制用例把 `tomli` 钉在它自
  己的 marker 上，把 `tomli-w` 钉在无 marker 上。官方语料的编码半边在同一批腿上被同一个 `tomllib` 探
  测整块跳过，如今也在这些腿上运行；其中三条 `spec-1.1.0` 用例以一位小数秒书写，CPython < 3.11 的
  `fromisoformat` 拒绝该拼写，因此测试自己的转换器把小数补齐到六位。

- **`partial` 的对拍格是已登记的洞，不再只是脚注** — 对拍列本就从每个 `test_*_crosslib.py` 的 oracle
  导入以及 `pyproject.toml` 给它们的 marker 推出结论，而 JSON 家族读作 `partial`，因为 `orjson` 与
  `pyjson5` 都声明为 `python_version >= '3.9' and python_version < '3.14'` 并排除 armv7l/s390x/
  ppc64le。页面用散文解释这件事，登记表却保持沉默，于是「JSON 家族有跨库层」这句话可以被拿去说本包承
  诺的 3.8 腿——那里模块什么都不导入、跳过、并报告与跑完它那 43 条对拍的腿一样的绿。`parity_gaps()`
  用同一推导发出 `parity-tier:json-jsonc-json5`，`.ci/quality-holes.json` 记下它并给出消除它的出口
  （`restricted` 为空，按同一方式测量）；洞 id 的 slug 现在由标签里的字母数字推出，
  `JSON / JSONC / JSON5` 无法再拼成没人读得懂的 id。

- **装不上 JSON 读器的腿现在也判定 RFC 8259** - JSON 跨库模块曾把门槛架在整个文件上，于是本包承诺的
  3.8 地板以及 3.14+ 上，模块里 43 条对拍全部跳过，连不需要第三方读器的那些也一起消失。现在其中一条
  用标准库 `json`：`parse_constant` 拒掉标准库默认放行的三个字面量，这就得到一个不受任何依赖声明约
  束、每条腿都有的 RFC 8259 参照。在被排除腿的模拟实测里（`orjson` 与 `pyjson5` 都不可导入）：29 条
  对拍运行、44 条跳过，而此前这条腿运行 0 条、共 43 条；两个读器都装上时 73 条全跑。对拍格必须改为按
  方言记账才说得出这件事——`crosslib_parity()` 只在某个测试自己调用无界读器、且该测试无法被跳过时给那
  个方言记分，所以 `@needs_pyjson5`、模块级 `pytestmark`、散文里的提及、测试之后定义的 helper 都不
  算；洞依旧登记，只是点名的变成 `json5` 与 `jsonc`，不再是整个家族。

- **解释器划下的界，现在也成了行里报出的界** - parity 列过去只问 `pyproject.toml` 某个读器会不会在某
  条腿上缺席，而且只在模块级 import 里找读器。`tomllib` 对这两问都隐形：没有文件声明它，而
  `tests/test_toml_crosslib.py` 是在那唯一需要第二重印证的测试里用 `pytest.importorskip` 绑住它。腿
  上自己的跳过计数说出了单元格说不出来的事 - CI 地板腿跑了该模块 24 条比较中的 13 条、跳过 11 条，
  3.9 与 3.10 跳的也是这 11 条 - 于是行里只列了一个读器，而文件其实问了两个。`STDLIB_SINCE` 用从
  `requires-python` 读出的地板来界定随解释器而来的读器，`_oracle_locals()` 现在也从语法里取
  `importorskip` 绑定的名字。TOML 的判定仍是 `yes`，也仍然诚实：`tomlkit` 声明无约束、每条腿都装得
  上，所以 3.8 到 3.10 丢的是三方一致，而不是没人判定的方言。该模块现在直接 import `tomlkit` —— 没有
  marker 约束的守卫在它绿着时是死代码，一旦不绿就成了静默跳过；而 `tomllib` 一旦算作有界，它的整文件
  `pytestmark` 本会拒绝给分、凭空造出一个并不存在的洞。用突变看过：把 `tomllib` 的到场版本改成地板版
  本，两条新对照都会变红。

- **分层表格的计数现在由执行校验的那次测量直接写入** — 该行此前已被一个比较测试钉在探针上，所以过期
  数字会红而不是撒谎；缺的是*产出*正确数字的途径。这个缺口让 PR #549 的 14 条测试腿全红：新增测试文
  件推动 Python 计数，作者得自己记得重打，而两个都改这一行的未合 PR 会逼着后落地的那个重放。
  `python scripts/quality_matrix.py --write-rows` 现在用同一份测量重写这些句子，保留数字周围的散文，
  并对无法再解析的表格拒绝报告成功；`--check-rows` 改为打印漂移。本分支现场演示：重放到 `47349338`
  （含 #553 的测试）后 Python 计数 1,822 → 1,824，一条命令就写好了该行。

- **中间版本的解释器不再在计费高的运行器上重跑整套测试** — 规划器此前让地板、顶端*以及*覆盖率承载版
  本在每个操作系统上跑全套，于是 `test (macos-latest, 3.12)` 花 173-285 秒重复
  `test (ubuntu-latest, 3.12)` 已经连同阈值一起断言过的内容,而 GitHub 对 macOS 按 10 倍计费。PR #550
  绿的那次实测（47 个作业、96.8 墙钟分钟）：macOS 只占 22.3 分钟，却约为 300 计费分钟当量中的 223。
  昂贵系统上那两个中间格是唯一角色本应只在 ubuntu 上的套件腿。`scripts/ci_matrix_plan.py` 现在用
  `expensive_suite_versions` 只从声明的范围推出昂贵运行器欠下的版本（首与尾），降级格仍留在网格里当
  作 wheel smoke 而不是消失，`check()` 会拒绝把门禁角色搬到更贵操作系统的计划。套件腿从 14 条降到 12
  条；每周排程仍用套件跑完 24 格，这正是允许降级而不掩饰它的前提。

- **测试矩阵不再等待它从不安装的 wheel** — `build` 在同一个作业里编译两种产物：各测试腿安装的
  abi3 wheel，与合规报告读取的 `--find-interpreter` 发布配方。在两次各含 44 个作业的 PR 运行中
  实测：第一次的 abi3 wheel 于 +134 秒（Windows）、+135 秒（macOS）、+151 秒（ubuntu）上传，第二
  次在同一三个系统上是 +172、+192、+167 秒，而其后的发布构建按操作系统还要多花 45–346 秒，于是
  `build` 到 +485 秒与 +477 秒才结束，`test` 与 `wheel-smoke` 的 24 条腿全都在它唯一读取的产物早已
  存在之后再空等 291–471 秒才起步。发布配方现在自成一个作业 `release-wheels`，而
  `tests/test_ci_artifact_wiring.py` 靠读取每个作业自己的 `upload-artifact` 与
  `download-artifact` 输入把两个生产者钉开：一个作业同时上传两族产物，或某条腿等待一个它根本不安装
  其产物的生产者，都会被点名。拿改动前的 `ci.yml` 跑这条检查，它报出的正是那个合并的作业，所以它
  拒绝的形状就是曾经上线的形状。墙钟总时长预计从 856 秒与 846 秒降到约 500 秒——汇聚判定仍在等最慢
  的那个发布构建——而本 PR 自己的运行就是实测。runner 分钟不变（100.2 与 93.1）：没有删去任何工作，
  删掉的只是排在它前面的队。

- **`pyq` 接管 `jq` 与 `yq` 的命令行** — 凡首个参数不是 pyq 子命令的 argv，现在都按
  `jq` 1.8 或 mikefarah `yq` v4 解读（由二进制被调用的名字决定，也可用 `--as jq|yq` 指定），
  再翻译成等价的子命令。被接受的写法是拿真二进制判的，不是握手写期望：
  `crates/pyrs-yaml-cli/tests/dialect_vs_reference.rs` 在 `PATH` 上能找到 `jq-1.8.2` 与
  `yq v4.54.1` 时就去跑它们，并拒绝把工作区里的任何文件当参考（探针的第一版在 `PATH` 上撞见
  我们自己的 `yq.exe`，报了九次完美匹配，什么都没证明），随后对 24 条调用逐字节比较 stdout，
  含 `.a.b`、`.list[] | select(.x > 1)`、`del(.a.n)`、`.a.n = 9`、`-o json`、`-r`、`-N`。
  凡壳无法给出同一份文档的，一律点名拒绝并以退出码 2 结束、stdout 为空：除单个尾随
  `select` 之外的管道、jq 的一切函数（`keys`、`length`、`map`、`to_entries`）、jq 模式下的编辑
  表达式（pyq 写 YAML，jq 写 JSON）、jq 的 `--arg`/`-s`/`-c`/`-S`/`--stream`，以及 yq 的
  `-n`/`-e`/`-P`/`--split-exp`/`--front-matter`/`--xml-*`/`--lua-*`。`pyq get` 新增了
  `--no-doc`，这是探针唯一找出的行为差异：yq v4 打印多项流时不带 `---` 分隔符，于是把它设为
  方言默认。整份约定写在 `pyq --help` 里。

- **站点上每一项性能数字现在都来自一次已提交的实测** — 四种语言的性能页与功能页上的墙钟表此前靠手工
  誊抄；2026-08-17 的数字没有可复现的脚本与记录，重新测量同一批调用后，每一项绝对读数都比旧值快
  22–108 倍。`scripts/bench_snapshot.py` 对成批调用计时（九个批次、每批至少 0.1 秒、取最快一批），把
  读数连同解释器、库版本与夹具大小提交到 `.ci/perf-snapshot.json`；`--docs` 用它重写全部十二张表，
  `tests/test_perf_docs.py` 逐格对照快照，表格因此无法漂移。散文此前单独漂移：首页、往返指南、
  PyYAML 兼容、基准、对比与功能页在表格撤回旧值后仍引用 21–43×、55–177×、48–100×、123–371×、
  37× 与 85×，所以门禁现在为全部 24 个以散文形式给出加速区间的页面推导区间，并点名拒绝已撤回的
  日期与区间。页面现在写作：对 PyYAML 解析 29–34×、序列化 223–368×，对 ruamel.yaml 68–81×、
  541–876×，往返指南为 45–59× 与 102–125×。yaml-rs 权衡段落改用同一计时器实测的 2–4 倍序列化与
  约 2.5 倍 `YamlDocument` 解析差距；2–4 倍按名保留而非推导，因为它来自跨库基准而非快照。

- **Rust 基准集开始测量 TOML、JSON 与 1.1 档位** — CodSpeed 的 Rust 腿此前只跑一个文件，
  其中每个用例都是 YAML 文本输入、YAML 或 Python 输出，因此本树增长最多的两个引擎、以及决定
  纯量标量语义的 1.1 档位都没有墙钟序列。`crates/pyrs-yaml/benches/yaml_bench.rs` 新增六个
  基准：`from_toml_medium`/`to_toml_medium`、`from_json_medium`/`to_json_medium`、
  `resolve_yaml11_type_many` 与 `needs_quotes_over_scalars`。四个格式基准读取指令数门禁所用的
  `bench_inputs` 文档，并沿用 Ir 场景名，使墙钟变化与指令数变化可按同一对数字阅读。地板不动：
  CodSpeed 继续负责排名，零回退判定仍归 `.ci/ir-baseline.json`。

- **MSRV 任务安装它声称要测试的 MSRV** — `msrv-check` 安装的是
  `dtolnay/rust-toolchain@stable`，因此在当前 stable 上编译整个工作区，并把它当作最低版本：
  清单中的 `rust-version = "1.96"` 只记录地板而不强制执行，需要 1.97 的依赖会通过该任务，
  却在用户安装时失败。该步骤现在钉住全部八个 crate 声明的 `1.96`，
  `tests/test_msrv_gate.py` 双向把关——工作流的钉扎偏离清单，或清单偏离钉扎，都会失败。
  改钉之前先实测了地板本身：`cargo +1.96.0 check --workspace --locked` 与
  `cargo +1.96.0 test --workspace --locked` 在本树上均通过。

- **第二份登记表记录矩阵无法测量的残留** —
  `.ci/quality-holes.json` 只能描述 `scripts/quality_matrix.py` 可推导的缺口，因此它的空列表被读成
  "没有残留"，而实际仍有被理解、刻意搁置的行为：无法留在表头行的 TOML 表头注释、`set_comment`
  落在容器最后一个子节点上的注释，以及缺失路径会抛错而标量路径返回 `None` 的 `sort_keys`。新增的
  `.ci/known-residuals.json` 为每条残留登记复现用的节点 id、归属层与可检查的退出条件；被钉住的测试带
  `@pytest.mark.known_residual`，`tests/test_known_residuals.py` 双向失败——登记项没有存活复现，或有
  标记的复现没有登记项。修复任一条都会让其特征化测试变红，从而迫使同一改动删除该登记项。

- **aarch64 wheel 改由原生 arm64 runner 构建** — `publish.yml` 的 `linux` 与 `musllinux` aarch64
  行从 qemu 下的 `ubuntu-22.04` 迁到 `ubuntu-24.04-arm`，glibc 行也随之改为在原生环境通过 `uv`
  运行测试套件。s390x 成为唯一仍在模拟环境下运行套件的 wheel 目标；`pyq` 的 aarch64 交叉构建、
  armv7 与 ppc64le 均未改动。

- **`main` 合并后的作业开始证明它过去只是打印的结论** — trunk 上唯一为代码变更运行的作业，只用一句
  `echo` 断言"推送尖兵的同一棵树早已作为 pull request 通过完整 CI 矩阵"，既没有东西测量它，也没有任
  何路径能让它失败。`scripts/check_main_provenance.py` 现在把这句话拆成三个问题向 API 提出：这个提交
  由哪个 pull request 产生、推送的树是否属于该 pull request 的某棵提交树、承载它的提交上
  `Test matrix (all legs)` 是否报告 `success`；答案不成立即拒绝。在所测得的每个尖兵（`07f988fa`、
  `60f670d0`、`225cf373`）上，关于内容的这句断言都成立，所以缺陷在防线的形状。但防线自身第一次
  跑上 trunk 就红了一次，原因是读法而不是内容，修法与本次改动一并落地：一个提交可以同时携带该
  检查名的多条记录，API 并不按完成先后排列，而本 workflow 的判定作业只在 `pull_request` 事件下
  运行，所以一次 `workflow_dispatch` 会在真正的结论旁边留下一条 `skipped`。把最后一行当成答案，
  就是把一次"没有判定"当成拒绝。`newest_verdict` 丢弃那些什么都没判定的记录，再按作业结束时间
  排序；于是 `fddcc76e` 通过，七个祖先仍各自因为一条确实存在的结论而被拒绝。

- **每格式防线表改为派生测量，并纠正两格错读** — `scripts/quality_matrix.py` 现在会依据声明防线
  的文件（fuzz 目标、`proptest!` 块、各 `test_*_crosslib.py`，以及 `pyproject.toml` 里的依赖标
  记）计算 `QUALITY_MATRIX.md` 的格式表，并由 `tests/test_quality_matrix.py` 逐格钉住。与之对账
  后测得两处偏差：`schema validation` 被低估——`validate_schema.rs` 会先解析再校验，其
  `fuzz parse` 应为 `yes`；属性层是 27 个而不是 25 个，因为旧的计数只认 `prop_` 前缀，而
  `pyrs-event` 的两个属性用的是散文命名。parity 列的 `partial` 如今指 oracle 上声明的解释器或架
  构限制，不再是主观印象。

- **JSON5 `\xHH` 转义可以解析了 — 读取器此前缺的只有它** — `load_json5` 与 `from_json5` 接受
  `\v`、`\0` 和行续接，却拒绝 `\x` 加两个十六进制数字这一 JSON5 转义集合成员，而随包发布的
  stub 早已写着 `load_json5` "Accepts the full JSON5 grammar"。与参考实现对齐测量：pyjson5
  对 `{a: '\x41'}` 给出 `{'a': 'A'}`，pyrs 却报 "invalid escape sequence"。现在两个数字按字节
  逐个累加 — `from_str_radix` 会放过前导 `+`，那会让 `\x+1` 成为合法转义 — 严格 JSON 与
  JSONC 继续拒绝该形式。Rust 测试钉住两种引号、`\x0A` 与 `\x00`、截断与非十六进制的尾部，
  以及跨多字节字符的畸形转义；pyjson5 对齐测试钉住这一接受与两个更窄方言的拒绝。

- **JSON5 的转义键与全部行结束符，以及盯住它们的表** — 同一个 stub 承诺过却仍缺的语法有两处：
  非引号键内部的反斜杠加 u 转义（JSON5 继承的 ES5 `IdentifierName` 允许它出现在任何标识字符
  的位置），以及字符串内反斜杠紧随的 CRLF、LS 或 PS。行续接的缺口同时暴露出同一方言两个扫描器
  的不一致 — CRLF 形式在双引号里能通过，在单引号里却失败。转义字符现在按它所代表的码点直接
  采纳，不再回头用 `ID_Start`/`ID_Continue` 复核；这正是"转义的数字或点可以作键，而字面字符
  不行"的理由，两种引号也共用一条行结束符规则。找出这两处的对拍同时升级为门禁：
  `tests/test_json5_spec_table.py` 用 41 个应接受与 11 个应拒绝的语法项与 `pyjson5` 对齐。读取器
  从未接受的写法对属性层与 fuzz 层不可见 —— 两者的 oracle 只能比较已经存在的路径。

- **流式映射无法拼出块样式键，生成器也不再生成它** — 一次 2 万样例的属性层随机读到
  `{? [null]: A, ? [null]: B}` 就红了：同一个单元素序列被当作两个键，一个块样式一个流样式。在块父节
  点下这对键确实是两个键——实测 `? - null` 与 `? [null]` 重读仍是两项；但在 `{...}` 里块容器根本没有
  合法写法，写入器只能对两者都输出流记号，读取器报重复键是对的。这棵树出自生成器而非文档：
  `arb_mapping` 用不区分上下文的规则筛键。`keys_collide_in_flow` 在比较前把两键折叠成流父节点唯一允
  许的拼写，折叠的是副本，`KeyIdentity` 仍是唯一模型；一致性门禁现在也用整张表在流父节点下重问一遍，
  只守生成器必须负责的那个方向：规则放过的键对，必须能被读取器保住。

- **防线矩阵现在测量它的外部权威** — `scripts/quality_matrix.py` 派生出第六列
  `external authority`：当某方言存在 `tests/test_*_suite.py` 或 `tests/test_*_spec_table.py`
  时记为 `yes`。此前每格式表只承认 unit、property、fuzz 与 parity 四层，于是仓库里最强的几道门
  —— `test_yaml_suite.py`、`test_json_test_suite.py`、`test_toml_test_suite.py` 与
  `test_json5_spec_table.py` —— 在这张罗列防线的页面上完全隐身，而被记为功劳的 parity 只是在
  比较两个实现。`schema validation` 记 `no`，因为没有外部权威去问它的规则。
  `tests/test_quality_matrix.py` 逐格断言该列、逐行列出一个文件，所以从派生里丢掉一个后缀会让
  测试变红，而不是悄悄少掉一列。

- **全是 Unicode 空白的 JSON 字符串保住了自己的值** — 键只有 U+2028 时，`load_json5` 答
  `{None: 1}`，而 `load_json` 与 `load_jsonc` 给出字符串，`pyjson5` 也站在它们一边。中枢把解出的
  JSON 字符串存成 YAML 标量，除非 `needs_quotes` 要求否则就是平文；而 JSON 解析器对平文标量按
  Unicode 规则裁剪——`resolve_json_type` 是有意留着 `str::trim` 的，因为 JSON5 的结构空白确实这么宽——
  于是首尾是非 ASCII 空白的文本会丢掉那个字符，整段只是它的文本被解析成 null。加引号的规则原本只认识
  ASCII 空格和 NBSP，现在它认识全部 25 个 Unicode `White_Space` 码点的双端，两张表就此不再打架。这个
  判断放在字节扫描本就会走到的字符路径上：文本含非 ASCII 字节才会到那里，所以纯 ASCII 的文本一分也没
  有多付。`tests/test_json_whitespace_scalar.py` 会把这套码点在键与值两个位置过三种方言，并请
  `pyjson5` 与 `json` 确认那是字符串而不是 null。

- **JSON5 字符串可以原样容纳控制字节** — 双引号扫描器会在任何低于 0x20 的字节上停下并拒绝，那是严格
  JSON 的 RFC 8259 §7 规则，不是 JSON5 的：这个方言在引号内只禁止行终结符。于是含真实制表符的文档会
  被 `load_json5` 拒绝，而 `pyjson5` 能读出来，且本文件的单引号字符串早就把同样的字节原样复制过去 —
  同一个方言给出两个答案，取决于用哪个引号开启字符串。停止集合现在跟随方言：JSON5 接受除 CR 与 LF 之
  外的每个 C0 字节，严格 JSON 与 JSONC 继续拒绝全部 32 个，而 JSON5 写入器会把读到的转义出去，往返仍
  是一行。`tests/test_json5_spec_table.py` 把 30 个字节按两种引号都问一遍参考实现，两个严格加载器也
  被同一批字节钉在 RFC 上。

- **JSON 写入器拿到了一个不归本仓库所有的读取器** — 这个系列的写入侧只拿我们自己的读取器校验，而一个
  不动点被"同一处错两次"的实现同样满足。`tests/test_json_writer_suite.py` 现在生成 1500 棵树，分别用
  `to_json`、`to_jsonc` 与 `to_json5` 写出，再把文档交给 Python 的 `json` 与 `pyjson5` — 没见过编码
  器的陌生人 — 并要求读回的值就是写进去的那棵树。钉住之前实测：三条路径 0 次拒绝、0 次值变。它确实会
  咬：让 `write_json_string` 输出原样制表符，两条严格路径立刻变红，而 JSON5 那条保持绿。

- **按 1.1 拼成数字的 YAML 字符串，读回来就成了数字** — `safe_dump({"v": "1_000"})` 写出
  `v: 1_000`，PyYAML 与 ruamel 都交回整数 1000。`8_0`、`0b101`、`+0x1f`、遗留布尔词
  `yes`/`on`/`off`/`y`，以及只有日期的时间戳 `1990-06-23`（读回是 `datetime.date`）同样如此。
  每条路径 26 个拼法，键位和值位都漏，Python 路径与 JSON→YAML 路径都漏。决定要不要加引号的
  守卫只问 `resolve_core_type`，而那是本引擎自己的 1.2 core 读取器，这些拼法在它眼里全是字符
  串；跟着 YAML 1.1 类型库的读取器会给它们定类型，本引擎自己的 `schema="1.1"` 档位对 `y` 和
  `yes` 也会——写入器和它自己的一个档位对同一个记号各执一词。守卫现在把 1.1 那张表当作第二个
  问题来问，覆盖四族：core 没列的布尔词、数字里夹 `_`、core 看不见的基数前缀（`0b`，以及带符号
  的 `0x`/`0o`），还有 `YYYY-MM-DD`。加引号是安全的错法——带引号的标量永不被解析成类型——所以
  每一族取那张表接受的形状，而不是另写一个解析器：`a_b`、`0x`、`2026-10` 仍是 plain 标量，
  `12:00` 用不上这一层，因为冒号在下一层就拒绝 plain 输出。只有这些拼法的输出变了：`1_000`
  现在写作 `"1_000"`。

- **YAML 写入器拿到了一个不归本仓库所有的读取器** — `tests/test_yaml_writer_suite.py` 经
  `safe_dump`、`from_json` 与编辑层写出文档，再交给 PyYAML 和 ruamel.yaml，要求读回的值就是写
  进去的那个；每条路径 400 棵生成的树，每个遗留拼法都问遍键位和值位。修好之后实测：三条路径
  0 次拒绝、0 次值变，`QUALITY_MATRIX.md` 现在把这一格与官方套件的读取闸门一起记在 YAML 行上。
  编辑层受比数据路径更窄的规则约束，这个文件把两边都钉住：路径片段*指认*的是读取器解析出的那个
  键（issue #336），所以 `$.~` 与 `$.42` 必须保住各自的 plain 折叠；而在两个档位之间会丢掉类型
  的名字——`rename("$.v", "yes")` 写出的键读回来是 `true`——要加引号。日期时间对象留在门外：
  `!date` 是本地标签，SafeLoader 没有它的构造器，所以陌生读者会直接拒绝那份文档，而不是改动某
  个值，这是标签设计本身的边界。

- **TOML 写入器拿到了一个不归本仓库所有的解析器** — 写入侧三格里的最后一格。TOML 的外部层级
  （`test_toml_test_suite.py`、`test_toml_crosslib.py`）问的都是读取器，所以 `to_toml` 此前只拿我们
  自己的读取器校验。`tests/test_toml_writer_suite.py` 用 `to_toml` 与 `YamlDocument.to_toml()` 写出
  400 棵生成的树，交给 `tomlkit`，在 3.11+ 上还交给 `tomllib`。实测 1200 棵探针树里 1074 棵写出且
  0 次拒绝、0 次值变；其余 126 棵止步于同一个带类型的拒绝 `toml-cannot-represent-null`：格式没有词
  能说的值是被拒绝，不是被编个拼法。两条边界钉住而不抹平——`datetime.time` 以裸本地时间发出（那是
  1.1 的写法），超出 i64 的整数带引号发出（轮毂本来就叫它字符串）。行为未变。

- **完整 IRI 写法 `!<tag:yaml.org,2002:X>` 现在也能命中以 `!!X` 注册的插件** — YAML 1.2 §6.8 说这两
  种写法是同一个标签；加载器对手点名的五个标签（`!!str`、`!!int`、`!!float`、`!!bool`、`!!binary`）
  本来两种都认，但其余名字查表用的是渲染后的文本，于是完整写法会退回默认解析：2026-10-18 在主干
  实测，`!<tag:yaml.org,2002:widget> 7` 会忽略以 `!!widget` 注册的插件而读出整数 `7`。现在先按文档
  自己的写法查，只有落空才多一次前缀判断；不属于 YAML 命名空间的 URI 读法不变——按「无标签」解析，
  不抛错。

- **对拍列现在能认出以另一个名字声明的参考实现** — `scripts/quality_matrix.py` 读取
  `pyproject.toml` 给每个跨库参考实现加的边界，却按导入名的字面量匹配：导入写作 `ruamel.yaml`，
  声明写作 `ruamel-yaml`，于是查表回答 `undeclared`，YAML 行就把每个平台都会装的参考实现报成项目
  从未声明。查表现在按 PEP 503 归一 `-`、`_`、`.` 与大小写，且 `tests/test_quality_matrix.py` 会拒绝
  任何 `undeclared` 读数——探针跟丢某个参考实现时要失败，而不是报错它自己。

- **省略 `extends` 的 schema 现在按文档所说的默认值解析** — `extends` 可选，
  `docs/zh/guides/custom-schema.md` 写明默认是 `core`；但解析器只在键存在且值为空时才取 Core。
  整个键缺失的 schema 没有基础解析器，于是每个标量都被当成它自己的文本，判定结果两个方向都无声地
  反过来：`type: int` 拒绝 `port: 80`（报错写着 `expected int but got Str("80")`），而 `type: str`
  照单全收。钉住旧默认值的那条 Rust 用例现在拆成两条，第二条记录仍存的边界：本引擎找不到的基础
  名字依然退化为「无回退」，而是否拒绝写错的名字是关于公开错误面的另一个决定。

- **让锚点失去引用者的编辑会被拒绝，而不是写进文档** — 在同时含有 `base: &x 1` 与 `use: *x`
  的文档里，改这个锚点的名字、不带元数据地替换它的节点、或把定义移走，都会报告成功并写出一份文档：
  其中仅存的引用指向无人定义的锚点，而本库自己的读取器以 `found unknown anchor` 拒绝
  `base: &a0 1 / use: *x`。規則本就在引擎里——带元数据替换值时 `with_metadata_from` 会保留文档侧的
  锚点名——这三个入口绕开了它，于是把问题问到真正发生丢弃的位置：`set_path`（`set_many_path` 与
  `move_path` 共用）、`delete_path`，以及锚点动词本身。守卫比真正会悬空的引用集合更宽：引用绑定同名
  锚点中*最近的前置*一个，所以后面的 `&x` 本可救活的改名、以及同时装着锚点与其引用的子树删除，同样被
  拒绝。多拒绝只让调用者多做一步，少拒绝则写出谁都读不了的文件。让改名把引用一并带上是对该动词语义的
  取舍，已作为待决事项记录。

- **编辑层有了自己的 fuzz 目标** — 工作区现在有九个目标，此前八个只把*文档*交给引擎，没有任何目标驱
  动一次*修改*，而这正是往返承诺所在的层。`fuzz_targets/edit_yaml.rs` 读取 `document NUL script`，选
  取读取器真正到达的経路，并要求每次被接受的编辑都能重新解析，且把自己写入的值留在它声称写入的位置——
  删除类断言按种类分别表述，因为键是身份，而序列下标是下一项会顶上来的位置。上面的锚点缺陷在第一次種
  子回放就被触发，首轮探索几分钟后再次触发；它还报告了三种现在只记录而不断言的形状：对标量请求排序会
  回答 “ok” 却不改动任何东西，请求的注释不总留在被指定的节点上，而同时带标签与注释的 flow 成员要第二
  轮才稳定——这是尚未关闭的缺陷，已存为种子以便下一次直接拿到输入。

- **`date` 与 `datetime` 现在以所有阅读器都认识的标签写出** — `safe_dump()` 此前写出
  `!date` 与 `!timestamp`，而本地标签在本库之外没有构造函数。2026-10-18 在主干实测：
  `yaml.safe_load` 与 `ruamel.yaml` 对这样的文档都抛 `ConstructorError`，值不是被改类型，
  而是完全读不回来。现在两者都写成 YAML 唯一定义的时间戳标签 `!!timestamp`，两个阅读器都
  认识它：只有日期的文本解析为 `date`，带时刻的解析为 `datetime`；本库读取时做同样的分叉。
  `!date` 与 `!timestamp` 保持注册并继续解析，旧版写出的文档仍读回原对象，带着本地标签读入的
  文档再写出时也原样保留——只有从 Python 对象新建的值才拿到标准标签。`!time` 仍是本地扩展：
  YAML 没有为纯时刻定义标签，PyYAML 甚至无法表示它。从 TOML 读入的值仍携带中枢标签
  `!date`/`!timestamp`，由下一条在 Rust 侧一并搬走。

- **TOML 的日期与日期时间也用同一个标准标签进入中枢** — `crates/pyrs-toml/src/lib.rs` 的投影把
  仅含日期的值标为 `!date`、日期时间标为 `!timestamp`，所以只是「读 TOML、写 YAML」这样经过一道的
  文档，仍会带上上一条所说、Python 边界已不再写出的本地表记。现在两者都用 `!!timestamp`，读取时按
  文本分叉，TOML 日期仍是 `date` 而不是零点的 `datetime`。这在 `parse_stream` 可见：TOML 时间值的
  `tag` 字段现在是 `!!timestamp`。纯时刻仍保留 `!time`，`!date` 依旧可读。

- **值只有标签、没有正文的 flow 成员，其行内注释在首轮输出就固定** — `{a: !t }` 把值写成空，
  跟在标签后的注释因此没有可依附的正文：读取方把注释交给外层键值对，标签被挪到注释之后，文档要
  到第二轮输出才稳定。往返并未丢失任何东西，这正是其他防线看不见它的原因。写入器如今在注释紧随
  其后时把这个空正文写成 `''`——与注释前的空值写成 `~` 是同一条判断（libFuzzer `edit_yaml`
  crash-b23eed05）。

- **块序列元素的注释留在破折号行上** — 经编辑层写入元素映射的注释在 `-` 行上没有位置，于是写入器把
  整个映射推到独立的行；读取方随后把注释交给结束该块的键值对，下一轮输出又把行压回紧凑形式，文本需要
  两轮才稳定。现在紧凑行会带上元素自己的注释（`- a: 1  # n`）；若该行已被 YAML 唯一的行尾槽位占用，
  注释就移到破折号之上（`# n`，随后 `- a: 1  # m`）——两种写法都是不动点（libFuzzer `edit_yaml`
  crash-61474da6）。

- **空plain标量写成它实际解析出的null** — 没有文本也没有属性的标量渲染为空，而读取方不会从空行里交还
  原样的节点：`k:` 与 `-` 都会被读成 null，其自身拼写是 `~`，于是首轮输出要到第二轮才稳定（libFuzzer
  `edit_yaml` crash-63749496，由 `- >` 把样式改成 plain 触发）。写入器现在凡是正文为空的地方都写出这
  个 null 文本——嵌套值、根值、序列元素、flow 键值、flow 元素。带标签的空值不动：标签抑制解析，所以它
  的正文保留能稳定的 `''` 写法（#514）。锚点不是同一种情况：它只给节点命名、不抑制解析，所以
  `- &a3` 现在写成 `- &a3 ~`（crash-07b6a10e）。

- **TOML 写入器的方言承诺补上了属性测试层** — 1.1 唯一可见的自由是行内表里可以换行（因而可以放注
  释），而此前没有任何东西问过：一条注释都没有的树，是否被写成了 1.0 也能读的文本。
  `prop_toml_undecorated_output_loads_as_1_0`（`crates/pyrs-yaml-core/src/fmt_pbt.rs`）现在要求任意
  位置都没有 note 的生成树必须能被 `from_toml_v1_0` 重新载入；
  `an_undecorated_inline_table_writes_compact_and_loads_as_1_0` 与
  `the_1_0_reader_refuses_the_1_1_inline_table_freedom`（`crates/pyrs-toml/src/writer.rs`）则按具体
  形状钉住行内表这条分支，并钉住它的判据前提。实测：强制走多行分支起初只让具名测试变红，20 000 个生
  成用例仍判绿；因此给属性补上一条必定生成含行内表文档的策略，同一个突变现在 0.01 秒内就能让它变红。
  第三条属性说出这层原先缺的等值断言：`prop_toml_roundtrip_preserves_the_tree` 比较一轮往返前后的树
  ——值、形状与顺序；两个丢值的突变都让它变红（同两次运行里文本定点也红了，所以这是把断言直说，而非更
  强的预言机）。行为未变；计数改为 720 个 Rust 测试与 29 条属性。

- **编辑档现在也审查导出的结果** — 中枢文档正是 `doc.to_toml()`、`doc.to_json5()` 与 CLI 的
  TOML/JSON 输出所写的对象，所以「编辑后再导出」是用户真正走的路径；而此前的每一档只把这些写手交给
  *读取器*生成的树，`toml_roundtrip` 与 `json_roundtrip` 从没见过被 splice 层改写过的树。
  `edit_yaml.rs` 现在把已稳定的树依次交给每一种方言（美化写法也算一种），要求该方言自己的读取器接受
  这段文本，并且第二次输出与第一次逐字节相同。某方言拒绝这棵树算通过，因为 TOML 没有顶层标量，严格
  JSON 也拒绝非有限浮点。编辑动词按设计只服务 YAML（`crates/pyrs-toml/src/lib.rs` 就是这么写的），所
  以这是一个档问七个写手，而不是七个编辑档。同两条问题也进了 `json_roundtrip`，问的是三种美化 JSON
  写法——那一档此前从未对没人特意生成的文本问过它。

- **替换容器的 `set` 不再丢掉容器内部被引用的锚点定义** — `set_path` 处的守卫只问过被替换节点*自己*
  的锚点，而那个节点并没有锚点，它的子节点才有；`preserve_metadata: true` 恰好只复制被替换节点自身的
  属性，于是写掉这个映射就删掉了 `x` 唯一的定义，留下 `use: *x` 指向不存在的东西，本库自己的读取器随
  即拒绝（libFuzzer `edit_yaml` crash-fbd364b3）。`delete_path` 早已通过 `anchors_in` 给出更宽的子树
  答案，现在这扇门也问同一个问题，用同样的 `anchor-is-referenced` 理由拒绝，并且不触碰树。

- **只带标签的序列元素上写的注释留在它所属的行** — `- !fuzz2` 把值写成空，所以标签之后的注释没有正文
  可依附：读取器把它登记成该元素的前导注释，第二次输出就把它写成破折号上方单独一行，于是文本在第二轮
  还在移动，尽管什么都没丢（libFuzzer `edit_yaml` crash-3dd60cad）。写入器现在会在行内注释之前先收尾
  那条待定的标签行——正如它早已在注释行之前所做——该元素写成 `- !fuzz2 ~  # fuzz 47`。

- **负零按 TOML 读回的那个零写进 TOML** — 整数透传会保留纯标量自己的拼法，而 `-0` 是合法的 TOML 文
  本；但 TOML 没有负零，`ist = -0` 读回来是 `0`，于是第二次发射动了。定点必须按读取方的拼法写，所以
  带符号的零写成 `0`，而读取方确实保留的进位制拼法（`0xDEAD`、`0b1`）照旧透传（libFuzzer `edit_yaml`
  crash-917e103b）。

- **被清空的块序列保住它的破折号行** — 删掉块序列的最后一项会把 flow 标志留下，而紧凑规则否决的正是
  输出并不具备的紧凑性：空容器没有块形式，永远写在键所在的那一行上，拼成 `[]`。首轮输出是
  `-\n  t: []`，读取方答 `- t: []`，文本在第二轮还在移动，尽管什么都没丢（libFuzzer `edit_yaml`
  crash-a58caa38）。

- **拼成长整数的 JSON 字符串在过中枢时保住引号** — 对长到放不下 `i64` 的数字串，Core 解析器答
  `Str`，而 JSON 对整数字面量不设上限，于是 `"9999999999999999999"` 被投影成 plain 标量，
  `to_json` 再把它写成数字：`load_json` 手里还是字符串，走过一次中枢就变成整数。投影现在先把文本
  交给 JSON 自己的数字语法问一遍——那正是写手接下来要问它的同一问题（libFuzzer `json_roundtrip`
  crash-09341b75）。

- **`schema="1.1"` 档位解析 1.1 定义的数字家族** — `1_000` 读成整数 1000、`0b101` 读成 5、
  `+0x1f` 读成 31、`+.inf` 读成无穷、`017` 读成 15，每个都与 PyYAML 默认加载器对同一文本的回答
  一致。此前该档位只在 Core 之上增加遗留布尔，而写入端子句自 #504 起就在为「1.1 读者会定型」的
  这些写法加引号，等于在为引擎里并不存在的读者加引号。时间戳与六十进制仍是文本（无标签文本保持
  文本的裁定；冒号已在下一层强制加引号），`0o17` 在这里仍被判型而 PyYAML 读作文本，因为该档位是
  Core 之上再加，而 `0o` 本就是 Core 的前缀。

- **只有 1.1 读者会定型的字符串不再裸着写出** — `0x_1f` 对 PyYAML 是 31、对 Core 是文本，而回答
  「1.1 读者会不会定型」的子句只在十进制词元上检查下划线，于是 Python 字符串 `"0x_1f"` 裸着出去、
  读回来成了数字。这个缺口由一条新的不变量测试发现：对本档位定型的每个文本，都要求 `needs_quotes`
  同意。

- **TOML 写入器的节标题不再物化键名** — `to_toml` 原先把每个根表的标题文字单独收进一个
  `String`，写完所有顶层配对后再写出；可是节列表本来就带着键节点，标题分支完全可以像键值行那样
  直接用 `push_scalar_key` 追加。这是写入器唯一物化出来的键：`to_toml_medium` 计数指令为
  26,741,057，地板是 28,939,722，即 -7.60%，每个标题少一次分配与一次拷贝。

- **TOML 无法排在其配对之后的根表改为内联以保住位置** — TOML 不允许根表头之后还有根键值，于是
  `to_toml` 把所有节都写到所有配对之后，映射的键序就没能经受住往返：`0` 持有一个表、其后是 `a`
  的树，回来时成了 `a`、`0`。生成器在 20 000 例的属性层撞到它
  （`prop_toml_roundtrip_preserves_the_tree`）。现在这样的表按内联形式写在它自己的位置上；已解析
  文档所能携带的顺序仍保持节标题写法。

- **序列自身的注释不再抢占元素的短行** — 编辑层挂到*容器*上的注释，原先被写到结束该容器块的
  那一行上方。当容器是序列时，结束它的行是某个元素自己的短行，而 granit 会把短行下方的注释报告成
  该元素的引导注释，于是第二次写出把这段文字从序列移进了元素里（`edit_yaml` crash-08b30707：
  `?ss:` 之后是两个带标签的空元素）。序列的注释现在写到序列首行之上——读取器正是把序列自身的注释
  归档在那里，`lift_first_item_notes` 也一直依赖这个位置；映射的注释仍写在结束其块的键值对之上，
  因为读取器就在那里读回一对的注释。

- **被 #522 打破的基准现在在常规测试套件里断言契约** — `Python benchmarks` 腿在 `806811c5` 上因
  `assert '[server]' in ...` 变红，而必需的 `Test matrix (all legs)` 仍报告成功。#522 让位于后续
  根级键值对之前的根表内联输出，因此夹具的 `[server]` 表头不再是输出内容；原断言描述的是 #522
  之前的拼写，而不是文档应有的性质。断言现在位于常规套件
  （`tests/test_toml.py::TestToToml::test_benchmark_fixture_roundtrips_by_values_and_order`），
  因为基准模块在不带 `--codspeed` 时会被取消选择。重新加载输出可捕获丢失或改变的键，比较根键
  列表可捕获顺序；交换输出的两条根行会让第一个比较保持相等、第二个翻转。基准体恢复为仅包含
  被测调用。

- **指令数门禁现在属于合并判定的一部分** — `Instruction-count baseline` 作业原本位于
  `.github/workflows/codspeed.yml`，而 `Test matrix (all legs)` 只汇总 `ci.yml` 的作业，因此它的
  结果只是一份报告：#522 弄红的 `Python benchmarks` 腿登上了 trunk，必需检查却是绿的；Ir 门禁
  同样可能如此。该作业原样迁入 `ci.yml`，与其他重型腿一样由 `changes` 分类决定是否运行；
  `matrix-verdict` 现在等待它，性能回退才会拒绝合并，而不是到 trunk 上才被发现。两个 CodSpeed
  作业继续留在 `codspeed.yml` 作为参考：低于 10% 的墙钟噪声适合排名，不适合作为合并决策。

- **Ir 门禁开始拒绝被代码甩在身后的地板** — 门禁原本只读基线的一侧：超出容差的场景失败，而远高于
  当前耗用的地板始终沉默，于是引擎每快一步，门禁默许的余量就宽一分。2026-10-14 的刷新测到了这种
  积累：六个写入场景比地板低 22–46%，四个读取场景低 4.7–11.2%。`scripts/ir_gate.py` 现在两侧都读：
  低于地板 2% 以上的场景以 `FLOOR_STALE` 告警，超过 5% 则失败，失败信息指向刷新调度任务，而不是
  把地板挪到本机的 `--update`。规则生效后的首次强制镜像刷新（run 39242949404，产物
  `ir-baseline-39`）无需 `accept_drift`：`to_toml_medium` 移动 −7.12%（旧门禁接受，新门禁拒绝），
  `from_toml_medium` 移动 −3.00%；最大的上移是 `serialize_block` +0.32%，每个场景的重复测量
  离散度为 0.000%。`.ci/ir-baseline.json` 就是该产物，未作任何改动。

- **嵌套序列中结束的映射注释在一轮内稳定** — 当映射的块结束于嵌套序列内部时，挂在映射上的注释
  会被写到最后一个元素的短行上方，读取器则把它报告为那个元素的引导注释：
  `ss:\n  - !a\n  # fuzz 35\n  - !a\n` 重读为 `ss:\n  - \n    # fuzz 35\n    !a\n  - !a\n`，
  第一次输出因此不是不动点。写入器用来放置容器自身内联注释的 `pair_line_start` 字段，被块值的写入
  覆盖成了序列元素的行；现在每个键值对在返回前恢复自己的行，注释因此落在结束该映射块的键值对上方。
  注释的归属仍会移动——容器上 `set_comment` 的裁定维持不变——但文本在一轮内稳定，并由
  `a_mapping_note_over_a_nested_sequence_settles_in_one_round` 以三种形状钉住。

- **TOML 整数保留书写时的拼写** — `1_000`、`+5`、`-1_000` 以及各进制形式在经由中枢时丢失了源码
  文本：解析器在记录前清洗了 token，`to_toml(from_toml("a = 1_000"))` 因此返回 `a = 1000`。
  分隔符与显式正号都是合法的 TOML（ABNF 中的 `underscored` 与 `signed-int`），丢失发生在记录
  环节而非语法。解析器现在完整保留 token 文本；仅当 YAML Core 解析出不同的值时，投影才添加
  标签（`0x1f` 与 `+5` 保持无标签，`1_000`、`0b1101_0110`、`-1_000` 携带 `!!int`），写入器的
  字面量判定也接受它自身输出的同一套语法。带标签的整数优先于 YAML 发射器因前导 `-` 而加的
  引号；无标签的带引号 `"42"` 仍是字符串。取值不变，并与 `tomllib` 一致；由
  `TestIntegerSpellingFidelity`、`decimal_spellings_reach_the_hub_word_for_word` 与
  `a_quoted_integer_stays_a_string_unless_a_tag_says_otherwise` 钉住。

- **显式 `%YAML` 版本按需生效，输出也如实声明** — 文档头部过去只由 `version()` 报告、读取端并不
  理会，因此 `%YAML 1.1` 仍按 Core 解析，指令也在输出中丢失。现在向 `parse`、`parse_file`、
  `parse_all_docs`、`safe_load`、`safe_loads` 或 `YAML(...)` 传入
  `honor_version_directive=True` 后，`1.1` 声明会选择 YAML 1.1 档位（`flag: yes` 读作 `True`，
  `012` 读作 `10`）；`1.2`、`1.3` 与未声明的文档仍按 Core 解析，显式 `schema` 优先于声明。
  开启标志时，被采信的声明会在其 `---` 之前重新发射为 `%YAML <version>`——编辑之后与
  `reparse()` 之后同样如此——Core 读取端因此不会悄然重判文本的类型；流中后续文档的声明前会补上
  语法要求的 `...` 结束标记。默认行为不变（标志关闭、按 Core 解析、输出不含指令）；
  `schema` 的默认值改为 `None`，以区分“未指定”与“显式 Core”。由
  `TestYamlVersionNegotiation` 钉住。

- **基准测试页面现在如实描述所用的测量方式** — `codspeed.yml` 自加入起就一直以
  `mode: simulation` 运行，但 `AGENTS.md` 与四份本地化基准/特性页都写着“WallTime 模式”。现在
  页面明确说明模拟模式统计指令数，毫秒表格标注为 2026-08-17 在单台机器上记录的墙钟读数，而非
  CodSpeed 输出。中规模夹具实为 687 B，不是约 500 B；Rust 表格标注为
  `pyrs_yaml_core::bench_inputs` 上的 `cargo codspeed` 模拟腿，其 "small" 文档为 19 B，而
  Python 表格使用 95 B，同名行不可直接比较。特性矩阵中 PyYAML 的锚点/别名、显式标签与合并键
  改勾（按 6.0.3 实测），类型存根行按各包实际内容标注：PyYAML 无，ryaml 与 yaml_edit 有。
  ABI3 行改为真实地板 py3.8+。

- **`safe_dumps_async` 已进入 `pyrs_yaml.__all__`** — 该别名在 `python/pyrs_yaml/__init__.py`
  中与 `safe_dumps` 一并赋值，三种本地化语言的 API 页也已把它当成员写出，但声明的公开面只列了另外
  三个 async 兄弟：`from pyrs_yaml import safe_dumps_async` 一直可用，而公开面说这个名字不存在。
  行为没有变化，这个名字本来就可导入；`tests/test_api_doc_signatures.py` 现在对四种语言的
  `reference.md` 比较同一组 35 个名字，不再为此留例外。

- **直接加载的资格判定移入引擎 crate** — 一个文档是否落在纯值快路径之内，取决于文档本身和语法，
  而不是它最终物化成的 Python 对象，因此该判定以及钉住它的两个测试现在位于 `pyrs-yaml-core`
  （`crates/pyrs-yaml-core/src/parser/yaml/direct.rs`）。绑定 crate 保留物化这一步——它是唯一需要
  Python 对象存在的部分——它的 Rust 测试层现在是空的。这一步的出发点「Rust 测试层需要 Python
  运行时」在二进制层面并不成立：`cargo test --workspace --no-run` 构建出的 848 个产物中，只有一个
  写入了 `python*.dll`，而且它就是扩展模块本身。仍然耦合的是构建图——`pyo3`、`numpy` 及其构建期
  crate 仍在绑定 crate 的闭包里——以及绑定 crate 的 `ir_gate` 基准宿主，它启动 CPython 来统计指令
  数。行为没有变化：`safe_load` 及其同类函数回答的文档与此前完全一致。

- **引擎的 CI 腿不再装配 Python 运行时** — `cargo test --workspace` 与提到 20 000 例的属性层
  现在都传 `--exclude pyrs-yaml`，`PYO3_PYTHON` 也随之删除。在本树实测：前一条命令展开成跨 153
  个包的 250 个编译单元，其集合点名 `pyo3`、`pyo3-ffi`、`pyo3-macros`、`pyo3-macros-backend`、
  `pyo3-build-config` 与 `numpy`；同一条命令加上排除后是 139 个包、220 个单元，上述一个也不
  剩。放弃的东西是零，而这正是许可这一排除的理由：`cargo nextest run --all` 与带排除的同一条
  运行都执行 748 个测试，而扩展 crate 没有可丢的 doctest（`cargo test -p pyrs-yaml --doc` 报
  0 个）。把这两项主张捆在一起的是 `tests/test_ci_rust_leg_exclusion.py`：当跨全工作区的测试腿
  不再排除解耦门禁所点名的扩展 crate、排除了却仍设着 `PYO3_PYTHON`、改成排除别的成员，或者
  MSRV 腿也开始排除时，它都会报红——因为那条腿的职责是就包括出货 crate 在内的每一个 crate 证明
  声明的地板。公开行为没有变化：同样的 748 个 Rust 测试和同一套 Python 套件照旧运行。

#### 移除

- **自研规则校验退役；`Document.validate()` 成为唯一的 Schema 校验面** ——
  `validate_against_schema()` 与 `validate_against_registered_schema()` 已移除，Schema 定义中的
  `validate:` 段不再被读取。这两个名字求值的是一套本仓库自研的规则语言，而 `Document.validate()`
  早已委托给 Python 的 `jsonschema`；两个校验器意味着两套错误契约、两种“文档是否有效”的答案。
  调用方改用 `Document.validate(schema)`（其 `YamlValidateError` 携带参考引擎的消息），或直接调用
  `jsonschema.validate()`。CLI 的 `validate` 把 `--schema` 与 `--schema-file` 绑定为同一个参数
  （JSON Schema 文件），两者都不给时只检查语法。标量类型解析不受影响：`register_schema()`、
  `load_schema()`、`list_schemas()`、`YAML(schema=...)` 与 `rules:` 列表照旧生效。仅为退役语言存在的
  Rust 类型（`TypeSpec`、`ValidateKind`、`ValidateRule`、`SchemaValidationError`）一并删除，矩阵的
  `schema validation` 行随之离场：表格现派生四行，`.ci/quality-holes.json` 实测为 `holes: []`。
  覆盖该行的 fuzz 目标由 `validate_schema` 更名为 `parse_schema`，因为它模糊的是幸存的 `rules:`
  语法，而非 JSON Schema 求值。四种语言的指南、快速开始、教程、特性表与 API 页面均改为记录
  `Document.validate()`，发布的桩文件中不再出现这两个被移除的函数。

#### 修复

- **最新两个发布也按老规矩折叠** - `## [v0.18.1]` 与 `## [v0.18.3]` 在五份发布说明里都缺了
  `<details markdown="1">` 折叠，而自 v0.12.x 起的每个发布都有。`scripts/check_doc_metadata.py`
  只读折叠的属性，从不问它在不在。两节现已折叠，缺折叠或缺 `<summary>` 的已发布小节会直接判红。

- **wheel 测试腿装上套件真正导入的依赖** - `.ci/requirements-test.txt` 是发布工作流唯一安装的需求文
  件，而它既没有声明 `tomlkit`，也没有声明 `json5`，但有四个测试模块在模块作用域导入它们。本版本第一
  次 Publish 在还没有上传任何 wheel 时就有五条 wheel 腿在收集阶段变红，`Release` 作业被跳过，所以
  PyPI 上什么都没落下。PR 矩阵不可能看见这件事：它安装开发依赖组，而那里两者都已声明。现在两个都补上
  声明，并且 `scripts/check_wheel_requirements.py`（一个 prek 钩子，由
  `tests/test_wheel_requirements_gate.py` 把关）会把 `tests/` 里每个模块作用域的 import 与该文件对
  账，于是这类漂移在引发它的改动处暴露，而不是在打 tag 时。

- **CLI 命令面清单门禁现在真的执行了** - `tests/test_cli_binding_parity.py` 原先先用
  `getattr(pyrs_yaml.cli, "app")` 取 App，而 `python/pyrs_yaml/cli/__init__.py` 并不绑定这个属性：它
  只定义 `main()`。任何更早运行过 `from pyrs_yaml.cli.app import app` 的测试——`tests/test_cli.py` 就
  在模块作用域里这么导入——都会把这个名字绑成子模块，于是 fixture 返回的是模块，`_commands` 不存在，
  守卫便以「cyclopts 私有布局变了」为由跳过。实测：单跑该文件 22 条通过，把
  `tests/test_cli.py::test_entry_point_without_cyclopts` 排在它前面就使清单断言变 SKIPPED，而本仓库
  自有 runner 的 `test (ubuntu-latest, 3.10)` 日志里同一条也是跳过——即「注册的命令集等于文档清单」这
  条断言在任何完整会话里都没跑过。现在 fixture 只接受带命令表的包属性；缺 `_commands` 是点名已安装
  cyclopts 版本的失败，而不是与漂移无从区分的跳过；另加一条控制测试断言解析结果绝不会落在模块上，使
  这一顺序回归报红而非静默。改动前还比对过清单本身：文档 18 个名字对注册 18 个，
  `missing=[] extra=[]`，所以这次只是让守卫诚实，并未揭出面漂移。

- **pyq 的每个 fixture 目录现在都同时带进程 id 与计数器** - 方言档位的比较目录在共用路径让
  `rust-lint` 变红时就加上了这道防护，而同一 crate 里其余写入仍只按进程 id 命名：
  `crates/pyrs-yaml-cli/tests/cli.rs` 的六处、`crates/pyrs-yaml-cli/tests/compare.rs` 的 `pair()`
  辅助函数及其唯一一处手写副本，以及 `src/compat.rs` 的 compat 过滤测试。目前没有哪两个测试选中同一
  前缀或 tag，所以这是把同一条不变量推广到所有位置，而不是第二次复现出的冲突；计数器消除了对“前缀
  始终唯一”的依赖，`compare.rs` 也不再手写一份 `pair()` 的两次写入。

- **方言对拍的各条比较共用同一个 fixture 目录** -
  `crates/pyrs-yaml-cli/tests/dialect_vs_reference.rs` 用进程 id 给临时 fixture 目录命名，而 Rust 框
  架会把一个测试二进制的各用例放进同一进程的多条线程并发跑，于是每条比较都在同一处反复重写
  `doc.yaml` 与 `doc.json`，彼此正在读取它们。`fs::write` 先截断再写入，代价在 PR #556 的
  `rust-lint` 上量到了：`jq [".a", "doc.json"] answered differently`，我们这边 `(0, "")`，参照那边
  `(0, "null\n")`——两侧在同一时刻之外各自读了同一个文件。只有那个 job 会装 `jq` 与 `yq`，所以这里的
  冲突只会表现为 `rust-lint` 变红，不会表现为跳过。现在每次调用都拿到自己的目录（进程 id 加计数
  器），`each_comparison_gets_a_fixture_directory_no_other_comparison_reads` 拒绝共享路径；把 pid 单
  独命名的写法放回去，这条测试就会失败——控制项是这样验过的。

- **`no_std` 门禁只能和自己对齐** - `tests/test_no_std_gate.py` 把 `ci.yml` 里的 crate 名单与一份
  抄进测试的集合相比，于是新增了 `#![no_std]` 却没有裸机构建、或删掉属性却仍在构建的 crate，
  都能让这个 job 保持绿。集合现在从各 crate 的根文件读出来。两条对照守住它：一条在合成的源文件
  上跑读取器，把属性和引用它的句子分开；另一条拒绝空集合——空与空相配看起来就像通过。

- **权威列为空只是散文，不是度量** - `QUALITY_MATRIX.md` 写着 schema 这层没有外来的读者问它的规则，
  可 `scripts/quality_matrix.py` 对空权威列派生不出任何洞，于是 `.ci/quality-holes.json` 一直停在
  `holes: []`，也就能被读成「没有短板」。探针现在会对它发出 `authority-tier:schema-validation`，
  登记表连同能关掉它的语料一起写下来，对照测试用手工构造的行跑两个方向：为空就出标识，填上就不出。

- **`pyq` 把序列切片当成了键** - `[0:2]` 是 `jq` 与 mikefarah `yq` 写区间的方式，
  但方括号解析走到了键分支，于是 `.list[0:2]` 去查一个叫 `0:2` 的键，查不到，
  兼容壳就答 `null`。现在语法会拒绝它无法寻址的 `:`，壳也点名拒绝该构造；
  带引号的方括号（`["k:v"]`）仍然是键。在 `--as jq|yq` 下，一条都没选中的
  `select` 也不再算错误：两个真工具都不输出并以 0 退出，现在壳也一样；
  原生 `pyq get` 仍按失败报告。

- **`pyq get --json` 对 1 条匹配和多条匹配给出不同排版** — 1 条匹配走带缩进的写器，
  多条匹配却把每个值压成一行，于是 `.servers` 与 `.servers[*]` 对 `--json` 的含义
  并不一致。现在每条匹配都得到同一种排版，也正是 `jq` 对每个输出值所用的形状。
  兼容壳补齐了这份一致的另一半：在 `--as jq|yq` 下，文档里没有的路径打印 `null`
  并以 0 退出，与两个真实工具相同；原生 `pyq get` 仍然报告未命中并以 1 退出。

- **只改 CLI 也会要求一次改不动扩展的重建** - 新鲜度门禁在判断「哪些文件被编进了
  已加载模块」时枚举了 `crates/` 下的每个目录，于是独立的 `pyq` crate 被算成已发布
  的一部分，凡只改它的提交都要付一次 `maturin develop` 加一个红。crate 集合现在从
  绑定 crate 清单的 `path` 依赖传递推导，并排除 dev 依赖；两个方向都有对照：真被编入
  的源文件仍会被点名，CLI 的源文件不再如此。

- **合并不能再给同一个映射第二个空键** — `~`、`null`、空键与裸 `Null` 节点都解析为 null，
  读取器会刻意把两个空键折成一个条目（这正是 `: a` + `: b` 能解析的原因）。合并过滤器问的却是
  另一个问题：未加标签的标量按文本比较，其余按整节点比较。于是 `<<` 源会把自己的空键放进已经
  持有一个空键的映射，两个合并源也都能命名它 — 写出的文本在同一层出现两次 `~:`，重读时那一对
  连同它的值与所携带的锚点一起被丢掉。`same_emitted_key` 现在改用读取器自己的谓词，合并集合也
  按同一身份折叠，所以"先提供该键的源获胜"对 null 的每一种拼法都成立，而不只对本就相等的键
  成立。由 20,000 例的属性层发现（CI 种子 `c9ef4fd3...`，同一棵树上 `prop_mapping_order_preserved`
  与 `prop_tag_preserved` 因这一个原因失败），并由
  `a_merge_never_repeats_a_null_key_the_mapping_owns` 与从
  `crates/pyrs-yaml-core/proptest-regressions/pbt.txt` 重放的种子钉住。

- **API 页现在照桩文件声明的参数表书写** —— `docs/{zh,ja,ko}/api/reference.md` 把 `parse_file`、
  `parse_all_docs`、`safe_load`、`safe_loads` 只写了一两个参数，而发布的桩声明了五到六个；`%YAML` 版
  本协商开关 `honor_version_directive` 在用户文档里完全没出现；`reparse()` 还写着一个扩展并不使用的
  `schema` 默认值。四语种现已与桩文件一致，该参数在归属页里补齐，`tests/test_api_doc_signatures.py`
  用 `ast` 比对两侧，于是签名被写短会是一次红运行，而不是读者遇到的意外。
  `YamlDocument.flush_source()`——正文页里唯一从未被点名的公开可调用对象——现已在四语种补齐，门还断言
  桩声明的每个公开名都要出现在各语种页面里。

- **重复键折叠移走的锚点定义会被内联，而不是被抛弃** —— `: abase: &x 1` + `use: *x` 再加第二个空键
  会与第一个重复，于是折叠丢掉的条目，其*值子树*里定义着 `&x`。修补趟此前只拿到被丢条目自身的锚点，
  这个名字因此失去定义，发射出 `use: *x`——自家 reader 会拒收（found unknown anchor），而引擎承诺永不
  产出这种文本。现在折叠会登记离开语法树每个定义，存活的引用被连同锚点一起内联，输出可复读且一轮即
  不动点（libFuzzer `edit_yaml` crash-68617eaf）。由
  `a_definition_folded_away_from_inside_the_dropped_value_is_still_emitted`、Python 侧的
  `test_a_folded_away_anchor_definition_still_reaches_the_output`、新增的锚点图语料门，以及重放种子
  `fuzz/seeds/edit_yaml/former-crash-68617eaf.seed` 固定。

- **带锚点条目的注释落在整条链最外层的 header 之上** —— 块容器无法在自己的 `&a` / `!tag` 行上方
  拼出的注释，此前被写在*最近*的 header 之上；而读取端每次输出都会把首项的前导注释上移一层，于是
  注释不断上爬：对 `- &a3\n  - &a0 !fuzz2\n    - {"-(.0bb:": v6}  # fuzz 47` 给最内层条目加注释时，
  第一次输出把它写在 `&a0` 之下，第二次又移到 `&a3` 之下。写入端现在跟踪值所在间隙的最外层 header，
  并把注释写到那条 header 之上，第一次输出即不动点（libFuzzer `edit_yaml` crash-2d978c7b）。由
  `a_headed_items_note_settles_above_the_outer_sequences_header` 与重放的种子
  `fuzz/seeds/edit_yaml/former-crash-2d978c7b.seed` 固定。header 位置改为随条目写入器传递的参数，
  而不再是序列化器状态，因为状态版本并不免费：在指令门禁的 Rust harness 上对已提交的地板实测，
  状态版本让 `serialize_small` +0.92%、`serialize_block` +0.75%，参数版本只有 +0.0006% 与 +0.19%。

- **带锚点条目的注释落在它无法拆开的头部之上** — 对一个作为序列条目、且其块以只有标签的行结尾
  （`- a: !o`）的块映射调用 `set_comment_path` 时，映射自身的行内注释被写在 `&a` 头部与正文之间。
  读取器把那里的注释视为映射的*前导*注释，于是下一次输出会把它上提到头部之前：第一次输出是
  `&a0` / `# note` / `a: !o`，第二次是 `# note` / `&a0` / `a: !o`，这次编辑因此需要两轮才稳定。
  写入器现在把该注释放到头部行之上，也就是读取器交还它的位置，第一次输出即为不动点（libFuzzer
  `edit_yaml` crash-b22d0653）。由 `an_anchored_items_note_settles_above_its_header` 与重放的种子
  `fuzz/seeds/edit_yaml/former-crash-b22d0653.seed` 固定。

- **TOML 拒绝的 YAML 浮点写法会按 TOML 语法写出** — Core YAML 把 `083e6` 解析为浮点数：其指数形式
  允许整数部分带前导零，而 TOML 的 `float-int-part` 是 `dec-int`，没有这种写法。写入器的直通判定
  只检查是否含 `e`，因此导出值为 `083e6` 的文档时写出了 `lis6 = 083e6`，TOML 读取器拒绝了自己写入
  器的文本（libFuzzer `edit_yaml` crash-71feb9d9）。判定现在先校验完整的浮点语法再直通；TOML 无法
  读取的写法改为按值渲染。由 `a_yaml_float_spelling_toml_refuses_is_written_in_toml_grammar` 与
  重放的种子 `fuzz/seeds/edit_yaml/former-crash-71feb9d9.seed` 固定。

- **紧凑条目的注释落在其序列头部之上** — 作为紧凑形式 `- key: value` 的条目，若其行尾已被自身注释
  占用，条目自身的注释会向上写到达线之上。当序列同时写出 `&a` / `!tag` 头部时，这一行位于头部与
  正文之间，读取器会把那里的注释视为序列的*前导*注释，于是下一次输出把它上提到头部之前：第一轮
  写出 `&a0` / `# note` / `- k: v  # note`，第二轮是 `# note` / `&a0` / `- k: v  # note`。首个条目
  现在记录头部的行首，并把该注释写在头部之上，也就是读取器交还它的位置（libFuzzer `edit_yaml`
  crash-2fc59020）。由 `a_compact_items_note_settles_above_the_sequences_header` 与重放的种子
  `fuzz/seeds/edit_yaml/former-crash-2fc59020.seed` 固定。

</details>

### [v0.18.1] — 2026-10-17

<details markdown="1">
<summary>自由线程 3.15t wheel · Linux arm64 · 保住 merge 键的注释</summary>

#### 新增

- **面向 Python 3.15t 与 Linux arm64 的自由线程 wheel** — 发布矩阵在 Windows、macOS 与 glibc
  Linux（x86_64 与 arm64）上构建 `cp315-abi3.abi3t` wheel，3.15t 及更高版本因此可以直接安装预编译
  产物，不再从源码编译；Linux arm64 的 `cp314t` 与 `abi3t` wheel 改由原生 arm64 runner 构建，
  qemu 无法完成的目标解释器 exec 在这里是原生调用。每个 wheel 都会在它所面向的解释器上安装
  并运行完整测试套件。

#### 修复

- **以别名为值的合并键保留行内注释** — `<<: *d  # note` 过去在读取阶段丢失注释：别名没有元数据槽位，
  读取器却仍报告附挂成功。现在注释由所在键承载，块写入器正是从该槽位输出它，因此文档原样往返；
  解析合并键后，注释改挂到映射中存活条目上。

- **“只有标签”行之后的键行内注释保住归属** — 当 pair 的值自带注释时，键的注释要写在 pair 行上方；
  若正上方是一个只渲染成标签的值（`k: !:`），读取器会把注释判给该值，文档要到第二轮才稳定。写入器
  现在先用该值自己的 `~` 闭合该行，所用偏移在 pair 开始时即已捕获（libFuzzer `yaml_roundtrip`
  crash-08cf4fe4）。

</details>

### [v0.18.0] — 2026-10-17

<details markdown="1">
<summary>五格式事件流 · 显式标签语义 · 往返保真 · 更快的读取器与写入器</summary>

#### 新增

- **五格式事件流** — `parse_stream` 新增仅关键字参数
  `format="yaml"|"json"|"jsonc"|"json5"|"toml"`；事件字典保持同样的七个键，格式无法回答的字段为
  `None`，`anchor`/`alias` 仍为 YAML 专有。默认行为逐事件不变。`pyrs_json::parse_stream` 与带校验的
  `pyrs_json::from_events` 已公开，TOML 也有了无树生产者。
- **schema 语言可以命名容器** — `type: map` 与 `type: seq` 只断言节点形状、不约束内容；
  `mapping_of: seq` 表示映射的每个值都是序列；`mapping`、`object`、`sequence`、`array`、`list`
  均可作为拼写。
- **`no_std` 基础 crate** — `pyrs-ast`、`pyrs-schema`、`pyrs-json`、`pyrs-toml` 仅依赖 `alloc`
  即可构建；新增可选的 `std` feature 恢复 `std::error::Error` 与 `RandomState` 哈希器，且默认开启。
- **预编译 `pyq` 与 Linux free-threaded wheel** — 发布流程为六个平台构建 `pyq`（Linux
  manylinux2014，glibc 2.17）并附到 GitHub Release；wheel 矩阵新增 Linux free-threaded wheel。
- **CI 加入 Python 3.15** — 矩阵覆盖 3.8 到 3.15，包含 free-threaded 解释器。

#### 变更

- **标准 `!!` 标签决定加载类型** — `!!str 1.20` 是字符串 `"1.20"`（原为浮点 `1.2`，尾零丢失），
  `!!float 1` 是 `1.0`，`!!bool yes` 是 `True`，`!!binary aGk=` 是 `b'hi'`，超过 i64 的 `!!int`
  仍是整数。标签无法满足的文本抛 `YamlTypeError`；未知应用标签仍宽松处理。
- **裸 `!` 表示标量不做 schema 解读** — `! 12` 按 YAML 1.2 §6.8 加载为字符串 `"12"`。PyYAML 与
  ruamel 会隐式解析并得到 `12`；本版本遵循规范，注册在 `!` 上的插件也不再覆盖标签。这是 0.18.0
  的显式行为变更。
- **`version()` 报告文档的 `%YAML` 指令** — `%YAML 1.1` 报告 `1.1`，`%YAML 1.3` 报告 `1.3`，
  未声明的文档仍报告 `1.2`。写入器不回写该指令。
- **`bytes` 值写成 `!!binary`** — 内置插件原输出本地标签 `!binary`，PyYAML 读不了，空负载还会出错；
  加载时仍接受本地拼写。
- **TOML 二进制整数保留写法** — `b = 0b1010` 往返为 `0b1010`（原被改写成 `10`），以
  `!!int 0b1010` 承载，所有格式读到同一类型。`0b1101_0110` 仍丢分隔符，手工构造的无标签
  `0b1010` 仍读成字符串。
- **严格 JSON 拒绝 RFC 8259 无法拼写的值** — JSON 写入器对非有限浮点报错而非改写；JSON5 读取器
  拒绝其语法禁止的前导零；JSON5 写入器不再输出自家读取器拒绝的数字。
- **`granit-parser` 1.3 → 2.0** — 大版本升级无需改源码；注释、锚点与合规层全绿，`cargo tree`
  确认依赖图确实为 2.0。
- **`CustomNode` 从 208 字节降到 144 字节**（`no_std` 下 128）— 元数据改由访问器读写，罕用的
  anchor/tag 槽共享一个指针：`from_json_medium` −4.8%，`serialize_anchors` −4.0%，两个尺寸都纳入
  热路径契约。

#### 修复

- **注释、锚点与标签归属** — 注释沿自己的节点行穿过标记、标签、嵌套容器与紧凑 `- key: value`
  形式；首条注释提升到头部之上；流式集合中带锚点的成员保留名字；锚点能穿过只是看起来像注释的
  `#`、不配对引号和方括号。别名绑定它前面的锚点（键位置亦然），嵌套与别名 merge key
  会被应用而非丢弃。
- **块标量一次输出即稳定** — 空块标量保留头部写下的指示符，不再凭空增加或丢失结尾换行，
  指示符识别正确。
- **TOML 往返** — 内联表内的点分键会嵌套；根级内联表不再改写为节头；表头按各段识别，而不是按
  点连接后的文本；多行数组内的注释归其元素；内联表成员的注释会被写出；多行字符串保留首尾引号、
  转义换行与结尾分隔符。
- **写入器输出不会再被自家读取器拒绝** — `to_toml` 曾输出多处非法 TOML（现有门禁）；JSON5
  输出可被 JSON5 读取器读回；JSONC/JSON5 写入器过去丢弃的注释现在保留，实测代价为
  `to_jsonc_medium` +4.77%、`to_json5_medium` +3.78%、`to_json_medium` +0.94%。
- **重复键与空键** — 重复 JSON 键不再连带丢失注释；即使键上带注释，重复名也按名折叠；重复键按
  解析后的值拒绝；`null:` 与 `'null':` 是不同键。大量空键的文档解析为线性时间（2k 个不同键 +
  2k 个空键，4 倍输入曾增长 12.4 倍）。
- **映射键与同文本的值同义** — 键身份跟随读取器的解析结果，而不是 AST 的节点种类；解析为数字的
  裸写法也不例外；固定模型从 15 类键扩展到 18 类。
- **schema 校验先看标签** — `!!int 0b1010` 按整数 10 校验，`!!int hello` 被拒绝而不是被改写类型；
  命名路径的规则会断言路径选中的节点，而不是走过错误种类。`$` 现在是规则目标，非 ASCII
  规则路径不再使进程中止。
- **Python 契约完整交付** — 提交的 stub 能解析并在 mypy 与 ty 下通过类型检查，声明全部十个扩展
  异常类，40 个模块函数的参数名与运行对象一致；28 个 API 页面上发布的签名会对照包实测；内联
  dict schema 被接受且所有键都保留；生成的参考页渲染出此前遗漏的四个公开函数（`to_toml`、
  `validate_against_registered_schema`、`validate_against_schema`、`validate_custom_types`）。
- **边界输入** — 仅 CR 换行的文档第二轮不再移动注释；Unicode 空白、BOM 与标签后缀不再损坏文档；
  CJK 与 emoji 键的流位置按字符而非字节计数；超过 64 位的 Python 整数不再以浮点离开 dumper。
- **`pyq`** — `validate` 接受 `--input` 并遵循实际读取的格式；拒绝关闭 stdin 时测试 harness
  不再 panic。
- **防线端到端可测量** — 质量矩阵的计数来自代码树，JSON/TOML fuzz 目标断言自家写入器的输出可
  重新加载，PR 级 fuzz 为阻塞门禁，分配上限与指令数地板覆盖热路径。
- **`pyq` 发布任务的工具链步骤会收到矩阵目标** — `publish.yml` 曾写
  `targets: ${ matrix.platform.std }`；单层花括号不是 GitHub 表达式，rustup 会把 `${`、
  `matrix.platform.std` 与 `}` 当作三个字面目标名，所有 `pyq` 任务都在工具链安装处停下。
  该步骤现在传入 `${{ matrix.platform.std }}`，形式一旦回退，
  `tests/test_publish_toolchain.py` 就会失败。
- **发布 wheel 的测试环境会拿到套件用到的工具** — wheel 任务在构建完成后运行完整套件，
  环境里却既没有 `git`（Alpine 容器），也没有 `tomli-w`（所有腿），于是九个仓库门禁与
  脚本导入测试在 wheel 构建之后失败。`.ci/requirements-test.txt` 提供 `tomli-w`，容器步骤
  安装 `git`，套件主体现在放在 `.ci/pytest-wheel.sh`，由每条容器腿调用：此前它内嵌在
  `docker run ... sh -c '...'` 里，注释中的撇号（`container's`）提前结束了引用，Alpine 只
  执行了 `apk add`，其余命令落到 runner 的 toolcache Python 上。脚本还按路径直接调用 venv
  的可执行文件，因为 Alpine 的 `/bin/sh` 没有 `source`；上述任一输入消失，
  `tests/test_publish_toolchain.py` 都会失败。

#### 性能

- **读取器每字节做更少工作** — TOML 读取器每个标量只分配一次（`from_toml_medium` 148 → 97
  次分配，16,362 → 15,612 字节）；JSON 读取器的无转义字符串借用源文本（74 → 48 次分配，
  9,675 → 9,444 字节）；JSON 与 TOML 扫描器按段复制而非逐字符解码；标量解析器不再裁剪无需
  裁剪的文本。
- **写入器分配更少** — TOML 写入器直接写入缓冲区，不再为每个输出格式化临时对象（同一文档曾计数
  53.2M 指令，而 `to_json_medium` 为 21.3M）；YAML 写入器用一次字节扫描决定引号；标签输出改为
  查表，转义不再分配。
- **事件流借用标量文本** — `StreamEventType::Scalar` 不再把每个值从 granit 的 `Cow` 中复制出来；
  该改动在事件场景实测 −9.192%。
- **指令数地板，以固定的 `rustc 1.97.1` 测量** — 相对已提交基线：`from_toml_medium` −9.80%、
  `from_json_medium` −13.37%、`from_jsonc_medium` −11.28%、`from_json5_medium` −9.61%。
  发布 wheel 由测量地板所用的同一编译器构建。

</details>

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
