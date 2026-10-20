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

### 变更

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

### 修复

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

### [v0.18.1] — 2026-10-17

### 新增

- **面向 Python 3.15t 与 Linux arm64 的自由线程 wheel** — 发布矩阵在 Windows、macOS 与 glibc
  Linux（x86_64 与 arm64）上构建 `cp315-abi3.abi3t` wheel，3.15t 及更高版本因此可以直接安装预编译
  产物，不再从源码编译；Linux arm64 的 `cp314t` 与 `abi3t` wheel 改由原生 arm64 runner 构建，
  qemu 无法完成的目标解释器 exec 在这里是原生调用。每个 wheel 都会在它所面向的解释器上安装
  并运行完整测试套件。

### 修复

- **以别名为值的合并键保留行内注释** — `<<: *d  # note` 过去在读取阶段丢失注释：别名没有元数据槽位，
  读取器却仍报告附挂成功。现在注释由所在键承载，块写入器正是从该槽位输出它，因此文档原样往返；
  解析合并键后，注释改挂到映射中存活条目上。

- **“只有标签”行之后的键行内注释保住归属** — 当 pair 的值自带注释时，键的注释要写在 pair 行上方；
  若正上方是一个只渲染成标签的值（`k: !:`），读取器会把注释判给该值，文档要到第二轮才稳定。写入器
  现在先用该值自己的 `~` 闭合该行，所用偏移在 pair 开始时即已捕获（libFuzzer `yaml_roundtrip`
  crash-08cf4fe4）。

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
