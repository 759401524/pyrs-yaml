---
title: Changelog
description: pyrs-yaml 项目的完整变更日志，记录所有版本的重要变更、新增功能和性能优化。
tags:

- **键上方有注释时，该键按名字查不到** —— `doc["key"]`、`"key" in doc` 与合并展开
  都按哈希定位节点，而 `CustomNode::hash` 折入了 `NodeMeta` 的归一化注释视图，
  `CustomNode::eq` 却只比较原始 `comment` 槽：于是两个节点等值却哈希不同，`IndexMap`
  对文档里明确存在的键回答「无此键」。`NodeDecor` 的文档写明它被 `Hash` / `PartialEq`
  排除，所以越界的一方是哈希——`CustomNode::hash` 现在只折入其等值真正比较的字段
  （`comment` / `anchor` / `tag`），经由新增的 `NodeMeta::hash_custom_node_identity`；
  `NodeMeta::hash` 仍镜像 `NodeMeta::eq` 的 #117 归一化，两对关系各自自洽。首键之外的
  每个键都会中招：文档最开头的注释会被记到外层映射上，这正是单键用例从未暴露它的原因。
- docs
status: new

---

## 变更日志

本文件记录该项目的所有重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-cn/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

### [Unreleased]

#### 新增

- **防线量出了自己身上的一处空档：指令数门禁够不到 Python 绑定层** —
  `quality_matrix.py` 现在推导 `ir_gate` harness 真正链接的 crate（bench 所属 crate 及其 workspace
  依赖：`pyrs-yaml-core`、`pyrs-ast`、`pyrs-schema`、`pyrs-json`、`pyrs-toml`），再与提供 Python API 的
  crate（按目录布局定位：`crates/*/src/py/` → `pyrs-yaml`）比对。后者不在图里，于是 `safe_load` 的
  AST→Python 转换 —每个用户都走、也是 PR #292 改的那条路 — 没有可复现的性能数值：只有墙钟覆盖它，而
  本仓库自己的记录就写着 10% 以下不可信。已按 `perf-coverage:binding-layer` 登记并给出移除条件，这是
  第一次靠问"这台仪器编译了什么"而不是"它命名了什么"找到的洞。
- **新增一个代表整个测试矩阵的检查（`Test matrix (all legs)`）** — 分支保护按检查"名字"匹配，而矩阵
  每条腿都贡献一个名字：3 OS × 7 Python 共 21 条，另加 free-threaded 与 coverage。一小时内两个 PR 就
  证明了代价：#298 在 `test (windows-latest, 3.8)` 从未被采信的情况下就 rebase 合了，而那条腿上带着两个
  在本包承诺的最低 Python 上跑不动的检查器（其中一个自写下之日起就无法 import）；#299 则因为格式化器的
  修复没被 squash 进去而让 `Hygiene` 变红。两者同一个形状：不被合并决策消费的检查只是报告。fan-in 就是
  `needs: [test, test-freethreaded, coverage]` 把 `toJSON(needs)` 交给 `scripts/check_matrix_verdict.py`，
  它只把 `success` 读作绿 — `skipped`（依赖死掉后兄弟腿的样子）与 `cancelled`（`cancel-in-progress` 的
  残留）都是拒绝而非缺席。`tests/test_matrix_verdict_gate.py` 钉住这些形状与接线，所以把 `needs:` 删空
  本身就会变红。PR 内做不到的一件事：必须把这个新检查"加入"分支保护，否则洞仍留在原地。
- **指令数门禁开始度量 JSON 与 TOML 的 writer，并把门禁自身的余量也记了下来** —
  `crates/pyrs-yaml-core/benches/ir_gate.rs` 新增 `parse_inline_merge`、`to_json_medium`、
  `to_toml_medium`（12 个场景，`.ci/ir-baseline.json` 也是 12 个数），于是 `to_json()` 与
  `to_toml()` 暴露的路径和 YAML 一样被门禁住。量它们的时候也把门禁量了一遍：对今天的树重跑已提交的
  九个场景，移动从 −2.35%（`parse_anchors`）到 +1.61%（`serialize_anchors`），而新增三个的复测精度是
  ±0.0005% — 这不是噪声，是取基线之后累积的漂移。漂移的方向才危险：2% 余量有五分之四已被
  `serialize_anchors` 花掉，`parse_anchors` 上 2.35% 以内的退化会完全看不出来。对策两条。
  `ir_gate.py --update --only <name>` 现在合并进已提交的文件，并拒绝编造没量过的数（旧行为是把九个
  数的文件覆盖成一个数）；`ir-baseline.yml` 在执行门禁的 `ubuntu-24.04` 镜像上重新度量全部数值，
  打印 diff 并作为 artifact 上传，由人读过再提交。该 job 的数值其实已经提交（见 Fixed）：十二个 runner 度量的值、
  一份 `generated_by` 溯源，以及按实测镜像间一致定出的 0.5% 容差。
- **JSON 与 TOML 的 writer 有了"落定文本"oracle（`fuzz/fuzz_targets/json_roundtrip.rs`、
  `fuzz/fuzz_targets/toml_roundtrip.rs`）** — #296 交付它时任何发布说明里都没有条目，所以在此补记，
  也正是下面的耦合门禁要抓的那类失败。两个 `parse_*` 目标本来已调用每个 writer，但把重解析结果绑给
  `let _ =` 后丢弃，于是它们断言的是"读取器接受自己的 writer"，而不是"writer 的文本已落定"——
  本引擎所有注释搬迁类缺陷都住在这个缺口里。新目标逐方言断言 `once == twice`，绝不跨方言
  （`to_jsonc_text` 会输出注释，`to_json5_text` 会输出严格读取器必须拒绝的十六进制数与裸键），
  `crates/pyrs-json/tests/roundtrip_corpus.rs` 与 `crates/pyrs-toml/tests/roundtrip_corpus.rs`
  在每次 `cargo nextest` 确定性重放已提交的 30 个种子——JSON 33 轮、TOML 30 轮，各自在文件里声明下限，
  因此不再驱动 writer 的语料会失败而不是空过。实测：没有不落定的 writer。
- **动了产品的变更集也必须动发布说明（`scripts/check_changelog_coupling.py`）** — 针对 pull request
  文件清单的两条规则：diff 触及 `crates/`、`python/pyrs_yaml/`、`fuzz/`、`scripts/`、`tests/` 或随包
  发布的 manifest 时必须触及 changelog；触及五份镜像之一就必须五份全触（`AGENTS.md` 的"不许提交部分更新"
  此前没有执行手段）。`check_changelog_mirrors.py` 两类都看不见：它比对版本头，而版本头只在发版时移动，
  且 `prek.toml` 的 `files:` 模式在没有 changelog 的 diff 上根本不会触发钩子。采用前用 `main` 最近 40 个
  commit 校准——8 个会变红，且都属于本仓库自己的说明已经写过的类别；把 `.github/workflows/**` 或
  `prek.toml` 算进来只会多算两次依赖版本升级，故排除。它跑在 `hygiene.yml` 的 pull request 粒度而非
  commit 粒度，因为拆分后的 PR 中某一个 commit 不带说明是正当的。判别力由突变证明：撤掉耦合规则恰好
  3 个测试变红，撤掉完整性规则恰好 2 个，把工作流触发项加回去恰好让 2 个校准哨兵变红
  （`tests/test_changelog_coupling_gate.py`，29 个测试）。
- **属性档新增 20,000 用例的阻塞作业（`ci.yml: property-tier`）** —
  此前所有属性测试都跑在 proptest 默认的 256 用例上，因为没有任何 workflow 设置
  `PROPTEST_CASES`；`scripts/quality_matrix.py` 把这一点测了出来，并登记为防线台账里最后的盲区。
  号称"在 20,000 用例失败"的三个 writer 不动点性质里，有两个从未真正失败：在该用例数下它们耗尽了
  proptest 默认的 1024 次全局拒绝预算，以 `Test aborted: Too many global rejects` 中止——把预算调回
  1024 后三个性质都死在 `fmt_pbt.rs:93`，这是实测。方言性质的预算现为 250,000，第三个失败才是真实缺陷
  （见 Fixed）。成本实测：在本树上 `PROPTEST_CASES=20000 cargo test --workspace --locked` 用 63 秒跑完
  core 的 305 个测试并以 0 退出。完成判据：把 `property-tier:default-case-count` 从
  `.ci/quality-holes.json` 删除、让台账为空——`tests/test_quality_matrix.py` 同时从反方向强制这一点。
- **卫生类钩子从此在 CI 里跑,行尾策略也有了能强制的形式** —— 新增 `Hygiene` 工作流,在每个
  pull request、每次推送到 `main`、以及每周,都对整棵树跑 `prek run --all-files`。它存在的原因是
  同仓使用的 `jj` 从不执行 Git 钩子,于是 `prek.toml` 里的十七个钩子只是本地自觉;十五个被跟踪
  文件已经带着 CRLF 行尾进入 `main` —— 12,263 行,其中五个是 changelog 镜像 —— 十几行编辑被放大成
  2,500 行 diff,而每个门禁都说绿灯。`.gitattributes` 声明策略,`scripts/check_line_endings.py`
  强制这条绝对规则,并在 `prek.toml` 里注册为 `line-endings-lf` 钩子。内置的 `mixed-line-ending`
  并*不*实现这条规则:注入一个全 CRLF 的文件它仍然 `Passed`,因为它只检测混合行尾。这十五个文件里
  十三个在此归一;剩下两个刻意保留,因为它们的 CRLF 位于原始字符串内部,正是已提交 Ir 基准所测量的
  输入(`crates/pyrs-yaml-core/src/bench_inputs.rs`、`crates/pyrs-yaml-core/benches/ir_gate.rs`),
  归一它们属于数据变更,应与基准重生成一并做。同一个作业也跑 `cargo fmt --check`,而此前没有任何
  工作流跑过它。 CI 的 clippy 也从 `cargo clippy -- -D warnings` 改为本仓库声明的 `--all --all-targets`
  范围——改动前先实测:更宽的命令在整棵树上本来就是干净的。
- **质量防线第一次被度量,而度量本身成了门禁** —— `QUALITY_MATRIX.md` 记录单元、属性、fuzz
  三档各自能到达与到不了的地方;`scripts/quality_matrix.py` 不抄写这些数字,而是从声明防线的文件
  (`.github/workflows/*.yml`、`prek.toml`、`fuzz/Cargo.toml`、`scripts/check_*.py`、Ir 基准
  bench 与 `.ci/ir-baseline.json`)重新推导。`tests/test_quality_matrix.py` 把推导出的盲区与
  `.ci/quality-holes.json` 台账比对,两个方向都会失败:新盲区不写下退出条件就不许落地,已经堵上的
  盲区留在台账里同样不允许。首轮实测浮出的结论,没有一条来自推断:CI 里没有任何任务跑钩子集,也没有
  任何任务跑 `cargo fmt --check`,CI 的 clippy 不看测试与基准,属性测试每次只用 proptest 默认例数,
  JSON 与 TOML 两个引擎只有解析向 fuzz 目标,写手从未被 fuzz。这个门禁自身的判别力用四次注入验证,
  每次恰好让为它写的那条测试变红。
- **CI 真正守得住的指令数门禁** —— `CodSpeed` 工作流新增
  `Instruction-count baseline` 作业，用*计数指令*（`callgrind` Ir）测量引擎热路径，
  相对 `.ci/ir-baseline.json` 上升超过百分之二即失败——该容差按两种 Linux 镜像之间实测
  的漂移校准（WSL 生成的基线在 GitHub runner 上最高 +1.45%）。之所以要它：divan 套件上报的
  wall-time 比较在十个点以内并不可复现——连续三次推送每次都比上一次*少做*工作，却被判
  −7.7%、−10.5%、−9.8%（同一组基准）。Ir 在同一二进制上复现精度约 ±0.001%，因此这条线
  是有意义的；整个门禁只花约六秒。用 `python scripts/ir_gate.py` 检查，用 `--update`
  谨慎地重置基线。它的场景通过 `pyrs_yaml_core::bench_inputs` 读取与 divan 套件相同的
  文档，两套测量因此不可能各自漂移。
- **注释存活现在是一道门禁，而不是一种指望（`crates/pyrs-yaml-core/tests/note_survival.rs`）**
  —— 往返档的预言机是文本幂等，而一份稳定但少一条注释的文档恰好能通过它；这个盲区正是五起
  静默丢注释能藏在绿光后面的原因。现在有一个确定性测试在每次 `cargo nextest` 重放已提交的
  YAML 种子语料，要求读取端记录的每一条注释都出现在发射结果里，**并且**每个输入都要一轮落定
  —— 今天有 37 枚带注释的种子真正承载这条断言，而随着崩溃变成种子，语料会自动把它扩大。
  它的边界写在文件里，并且用突变证明而不是靠论述：让读取端在并没有挂上注释时报“已处理”，会让
  那些钉住形状的测试变红，而这道门禁仍保持绿 —— 因为在摄取阶段丢掉的注释根本到不了门禁所测的
  AST。改成去数源文本里的 `#` 反而会把正确输出判红：语料里就有 `!###0 …` 这样一个后缀全由
  `#` 字符组成的标签。会把正确输出弄红的门禁不如没有，因此读取端那一半仍由 fuzz 档与逐形状
  钉选守住，两半都不声称能覆盖对方。
- **CI 每周定期模糊测试** — `.github/workflows/fuzz.yml` 每周六运行全部
  四个 libFuzzer 目标（也可手动触发，`fuzz/` 变更时自动触发），用精心挑选
  的 `fuzz/seeds/`（历史崩溃输入 + 人工写制的形态种子）为每次临时语料库
  播种，失败时上传崩溃产物以喂养「崩溃 → 回归测试 → 种子 → 修复」流水线。
  机器生成的语料库仍永不进入 git。
- **引擎的 `cargo-fuzz` 模糊测试套件（`fuzz/`）** — 四个覆盖引导的 libFuzzer
  目标：`parse_yaml`（单文档 + 流）、`yaml_roundtrip`（解析 → 序列化 → 重解析
  与序列化幂等）、`parse_json`（三方言 × 三 writer 全交叉重解析）、`parse_toml`
  （1.0/1.1 及 writer 重解析）。仅目标代码入库；语料与 crash 产物为每次
  会话本地生成、保持不跟踪（crash 发现以回归测试钉住，而非 corpus 文件）。
  套件在首跑一分钟内即证明价值——见下方注释扫描器修复。
- **`pyrs-ast` / `pyrs-schema` 支持 `no_std`** — 所有格式引擎赖以奠基的两个基础
  crate 现在仅靠 `alloc` 即可构建：`indexmap` 与 `thiserror` 关闭默认 `std`
  feature，新增的 opt-in `std` feature 重新启用 `std::error::Error` 实现与
  `RandomState` 哈希器。`std` 仍默认开启，所以现有所有使用方继续拿到与以往
  完全一致的 `IndexMap<K, V, RandomState>` 节点映射类型；`no_std` 用户用
  `default-features = false` 退出，获得固定种子的哈希器。Proptest 节点策略移到
  新的 `test-strategy` feature 之后，普通构建不再承担属性测试成本。CI 任务
  `no-std-check` 为裸机目标交叉编译，确保这一性质不退化。
- **`pyrs-json` / `pyrs-toml` 也支持 `no_std`** — 两个原生格式引擎仅靠 `alloc`
  即可构建：`std::sync::Arc` 迁至 `alloc`，`String`/`Vec`/`format!` 预导入改由
  `#[macro_use] extern crate alloc` 显式提供，解析期键值存储复用 `pyrs-ast` 的
  `NodeMap` 哈希器别名，`canonical_float` 的整数值判定改为 core-only
  （`f64::trunc` 是 std 固有方法）。`no-std-check` 作业现覆盖全部四个 crate 的
  交叉编译。
- **`pyq` 随每个发布提供预编译二进制** — `publish.yml` 新增 `pyq` 任务，为六个
  平台构建原生 CLI 并把压缩包附到 GitHub Release，用户不再需要 Rust 工具链即可
  获得独立二进制。
- **类型桩漂移门禁（`scripts/check_stub_drift.py`）** — 已提交的
  `python/pyrs_yaml/pyrs_yaml.pyi` 是机器生成物，随每个 wheel 分发；但 CI 此前只
  断言它存在且被追踪（`release-guard`），因此绑定签名一变，公开的类型契约就可能
  静默落后。`validate.yml` 新增 `stub-drift` 作业，用声明的再生成路径
  （`uv run maturin generate-stubs`）重生成桩，内容有任何差异即失败。两处归一化让
  被追踪的桩保持完全派生而非手工打补丁：prek 钩子在提交时剥掉的行尾空白，以及一
  处声明式保真正例——maturin 1.14.1 对两个 `__next__` 返回丢掉了绑定实际返回的
  `Option`。每条声明的正例都会断言预期命中数，因此签名变化或上游修复会明确失败，
  而不是被静默改写。`mise run stubs` 现在也走同一条管线写入。

#### 变更

- **`pyq validate` 接受 `--input` 并按真实格式解析** — 此前它硬编码 YAML 解析器且
  无任何可改途径，指向 `pyproject.toml`、`package.json` 等非 YAML 配置一律被拒绝。
  现在它接受 `--input auto|yaml|json|jsonc|json5|toml` 并经共享加载器路由。
- **`pyq` 预编译二进制改在 manylinux2014 容器中构建** — Linux 目标经 `cross`
  在官方 CentOS 7 镜像内原生编译，钉定 glibc 2.17 下限，零手拼交叉工具链地
  覆盖 CentOS 7 / Ubuntu 16.04 / 18.04 / Debian 8 / 9。更早的 zigbuild 方案
  把 x86_64 钉得低一个小版本（2.16，实测 `getauxval` 地板），但 `publish.yml`
  的首次真实运行（该工作流从不在普通 PR 上执行）暴露出整个 zig 栈并不完整
  （`cargo zigbuild: no such command`、armv7 腿被主机默认 `-fuse-ld=lld` 误链、
  冒烟测试执行了 runner 跑不了的二进制）；容器方案用一个社区标准工具替换了
  三种失败模式，且下限比 runner 自带 glibc 低两个大版本。
- **本地化文字纯度门禁（`scripts/check_cjk_localisation.py`）** —— 现在机器检查 `ja` / `ko` /
  `zh` 的每一个页面是否只用自己这套书写系统：非 `ja` 页不得出现假名，非 `ko` 页不得出现谚文，
  `ja` 页不得出现仅简体中文使用的汉字，而 `ko` 正文一个汉字都不许出现。韩语那条规则原本是一份
  手工挑选的十三个简体独有码点清单，而这份清单正是门禁失明之处：日语新字体 `経` 与繁体 `內` 都
  不在其上，于是韩语变更日志带着十五行韩中混杂的散文发了出去——像 `热点 样本` 那样的词组混在
  韩语助词之间——而仓库里每个检查器都打印 OK。改判“任何汉字”之后，规则不再需要同步任何清单。
  技术文本按码位豁免（围栏代码块、行内代码、链接目标），因为同一批韩语页面本就应当在 YAML
  样例里展示 `title: 文档标题`，并把 `{ é: 1, 名: 2 }` 列为解析器输入；若按行豁免，这两处都会
  被判定违规。扫描范围从变更日志的 `[Unreleased]` 块扩大到每个语言的每个页面，且先做了测量：
  `zh` 与 `ja` 各自 41 个页面零违规，`ko` 的发现全部集中在 `changelog.md`，而那十五行在同一次
  改动里一并修好。门禁现在除 prek 钩子之外也会在 CI（`Validate` → `script-purity`）中运行，
  并有了针对自身的测试（`tests/test_cjk_localisation_gate.py`），覆盖它必须抓住的每种形状与
  必须容忍的每种形状——以突变证明：关掉 `ko` 的汉字规则恰好让那四个韩语用例变红，其余不动。
  本条的韩语译文同样写得通过它所描述的那条规则。
- **PR 档模糊测试现已阻断（`fuzz.yml`）** — PR 上的步骤级
  `continue-on-error` 只是棘轮而非漏洞，它存在的唯一理由是 main 仍带着这道档会
  正确标红的漂移（crash-f44eca1d，#256/#258/#261/#262 修复其前序之后）。2026-10-04
  在当前树上复核：四个目标重放已提交种子语料全部干净（5/59/6/5 枚种子，钉住的
  nightly-2026-08-15、cargo-fuzz 0.13.2、60 秒发现窗口），于是新崩溃会让引入它的
  PR 直接失败，而不再等到下个周末才暴露。2026-10-05 在合并键覆盖规则落地后重新复核：
  75 枚种子重放四个目标全部干净，但新跑的 60 秒发现窗口仍会触及注释重定位这一族
  （`crash-a916de77`，48 字节）—— 已记入 `ROADMAP.md`，未播种也未修复，因为播种就是
  承诺该输入能过。账本里列为下一个的 key-metadata 缺口
  （crash-86a9ae7b）已用单输入重放确认关闭；核查过程中发现的漂移
  `yaml_roundtrip` crash-ac5d9043 作为当前未关闭项写入 `ROADMAP.md`，未播种也未修复。

#### 修复

- **Ir 基线再生成作业原本要用与基线记录不同的编译器来量** — 该作业第一次真跑（能触发
  `workflow_dispatch` 的只有 `main`）用的是 `rust-toolchain@stable`：runner 的 stable 是 rustc
  1.99.0，而 `.ci/ir-baseline.json` 是在 1.97.1 上产出的，于是代码未动、重生成的文件就把
  `serialize_medium` 挪了 +6.7%、`serialize_small` +5.8%。若把那份 artifact 提交，一次编译器退化就此
  变成基线，门禁再也不会察觉。现在作业按执行门禁那个作业同样的方式解析 toolchain（从基线里读版本号），
  只把"移动 pin 本身就是变更"这一种情形交给 `toolchain` 输入，仍是显式动作。
  `tests/test_ir_baseline_workflow.py` 钉住：只能手动触发、绝不自动 push、不许退回 `@stable`。
- **三个检查器在本包支持的最低版本 Python 3.8 上跑不起来** —
  `scripts/check_changelog_mirrors.py`（`-> set[str]`）与 `scripts/check_stub_drift.py`
  （`-> tuple[...]`）在 import 时求值签名注解，缺 `from __future__ import annotations` 就会被 3.8 拒绝；
  `scripts/check_changelog_coupling.py` 调了 3.9 才有的 `str.removeprefix`。之所以没人发现，是因为跑这些
  脚本的 job 全是 3.12/3.14；把它暴露出来的是 pytest 矩阵的 3.8 那条腿，而触发点是"只读检查器"的
  coupling 门禁自己的测试文件（它 import 并调用了它们）。那条腿上的实测：import 时
  `TypeError: 'type' object is not subscriptable`，首次调用时
  `AttributeError: 'str' object has no attribute 'removeprefix'`。现在这一类被守住了：
  `tests/test_scripts_import_on_supported_python.py` 用运行测试的解释器 import `scripts/` 下每个文件，
  静态检查内建泛型注解必须带 future import，并断言最低版本取自 `pyproject.toml` 而非记忆 — 它第一次跑就
  揪出 `check_stub_drift.py` 这第三处。局限也写在该文件里：函数体内的新 API 调用 import 看不见，所以门禁的
  行为测试必须继续真正执行那些函数。
- **Ir 基线改由执行门禁的环境生成，容差从 2% 收到 0.5%** — 同一个 commit 在两个 GitHub runner
  镜像上度量，十二个场景最多相差 0.0018%（`parse_anchors`：341M 中 6,061 条指令），所以
  `.ci/ir-baseline.json` 现在装的是 runner 自己的数值并带 `generated_by` 溯源；`scripts/ir_gate.py`
  在运行机器与记录不符时会打印提示（WSL 运行就打印了）。旧的 2% 是围着"WSL 度量的
  `serialize_block_scalars` 与 runner 差 1.45%"设计的，两个解释都被试过且都不成立：说
  `.gitattributes` 正规化了 `BLOCK_SCALAR_YAML` 里的 CR —— 加 `-text` 后 runner 的数只变了
  16,020,906 中的 28 条，而这份 fixture 存进去的字节本身就不稳定：`git show` 对 `main` 的副本报 0 个
  CR、对加了 `-text` 的分支报 98 个，Windows 的 checkout 又会把它们插回来 — 这才是要 `-text` 与新的
  字节相等性测试堵上的可复现性缺口；说镜像之间漂移 —— 两个镜像一致到 0.0018%。于是这道差被记为未解决，
  真正改变的是门禁不再拿来源不明的数去比。`-text` 只为字节稳定性保留。
- **TOML 多行内联表中非末位成员的注释，写到读取器报告它的位置** — writer 原先把它放在分隔逗号之后
  （`b = 1, # n`）。`#` 一直到行尾，TOML 无法把逗号留在注释里，于是读取器把那条评论改记为*下一个*键的
  行首注释；第二次输出就会移动它，文本因此永不落定。这一形状无法由合法 TOML 文本产生（所以只有生成器能
  发现它），只会经由转换路径出现——那些路径交给 writer 一个源码无法表达的 AST。现在注释输出在成员之后的
  独立一行（`b = 1,` / `# n`），正是解析器报告它的地方。归因：撤回该规则恰好让
  `writer::tests::a_same_line_note_on_a_non_last_member_is_emitted_on_its_own_line`（0.45 秒档）与
  20,000 用例下的 `fmt_pbt::prop_toml_writer_is_fixed_point` 变红，其余不动；而在 256 用例并移除已
  持久化的 shrink 用例后整套测试全绿——是这个高档位堵住了这个洞。
- **键下面的空容器也不再需要 dump 两次** —— `#287` 让两个写手都把 `{}` 与 `[]` 内联到 dash
  行，但从 Python 对象发射*映射值*的那条路径仍然缺这同一个判断：`safe_dump({"a": {}})` 产出
  `"a:\n  {}\n"`，而 AST 写手产出 `"a: {}\n"`。两种文本都能读回同一份数据，这就是往返测试
  一直看不见它的原因；区别只在于其中一个 dump 一轮就不再移动。它后面还藏着一个形状——序列项只要
  含一个空容器就整个失去紧凑 dash 写法，于是 `safe_dump([{"a": {}, "b": 1}])` 产出
  `"- \n  a:\n    {}\n  b: 1\n"`，而写手本应输出 `"- a: {}\n  b: 1\n"`。两处都已修正。发现
  它们的缺口是新增的：`tests/test_route_parity.py` 把等价数据同时喂给两个 YAML 写手——解析后树上
  的节点写手，以及面向 Python 对象的 `direct_dump` 快路径；二者按设计互为镜像而不共享代码——并要求
  每个形状逐字节一致，同时把有意保留的差异（引号风格、流式风格、块标量、锚点、标签）作为差异钉住而
  不是抹平。防线度量里登记的 `route-parity:node-writer-vs-direct-dump` 就是靠这张表在登记次日关闭的。
- **自己参与合并的模板现在会把继承来的键继续传下去** —— 昨天发布的合并修复还有第二个位点：锚点体在任何合并解析
  之前就被快照，所以 `use: {<<: *m}` 引用 `mid: &m {<<: *b, y: 2}` 时读的是过期副本，看到一个以为目标
  已经拥有的 `<<`，就把它跳过了。`use` 返回 `{y: 2, z: 3}`，而 PyYAML 返回 `{x: 1, y: 2, z: 3}`——继承来的
  `x` 没了，三层链则一次丢两个键。输出文本始终稳定，所以往返断言看不见它；只有对象视图能作证。现在每个锚点体在
  被读取的地方先解析，覆盖关系也因此落在正确的层级：链自己的键胜过它继承的，文档自己的键胜过整条链——每层一步，
  与两个参考库一致。
- **合并源内部的合并键现在会被应用，而不是被丢弃** —— `<<: {<<: {x: 1}}` 此前会把两层 `<<` 都当作数据
  留下（`{'<<': {'<<': {'x': 1}}}`），而 PyYAML 与 ruamel 对同一份文档读出的是 `{'x': 1}`；
  `<<: {<<: {x: 1, y: 1}, y: 2}` 直接丢掉 `x`，块式写法 `<<:` 下的 `- <<:` 同样丢。收集器会跳过目标
  “已经拥有”的源键，判断方式是整节点比较，而此刻目标自己的 `<<` 条目还在表里，于是嵌套的合并被丢弃而
  非执行。又因为该比较把键的元数据也算进去，一条*注释*就能改变结果：三行写法中只要中间那行的 `<<:` 带注释，第一次
  dump 就消费一层、再读它自己的输出时又消费一层，文本每轮都在动——这正是 fuzz 档报告的漂移。而那些“稳定但
  合并没做完”的文档从来不会让任何测试变红：文本等价的预言机看不见一次未被执行的合并。现在一次解析就会先把
  源解析完，同一文档的每种写法含义一致，且源自己的键仍然覆盖嵌套合并带进来的键（`<<: {<<: {x: 1}, x: 9}`
  就是 `x: 9`，与两个参考库一致）。
- **序列式键首项上的注释不再每轮爬一层** —— 写手把它放在键体缩进下、`-` 的上面，可是再读时这个位置的
  注释会被报到*序列*上，于是下一次输出又把它提升到 `?` 标记的上一行，文档要到第二轮才收敛：
  `?` + `-` + `#?` + ` ? ` 第一次输出把注释写成键体内独立的一行，第二次输出才把它抬到标记的上面。
  标记行现在才是读写手一致的位置，一次输出即到达不动点，注释也留在文档里。来自 fuzz 积压清单
  （`crash-1445c91a` 与 `crash-f1643b2d`，两个输入最小化后是同样的 10 字节）。
- **容器的注释不再借用“只有标签、尚未写完”的那一行** —— 在注释**行**跟随之前先闭合该行
  （`k: ! ~`）自上个版本起已是规则；留下的漏洞是容器自己的行内注释，它被写手追加到块结束的那
  一行上。对 `:\t!-<CR>... #-o` 这就得到 `~: !-   # -o`：文本是稳定的，可重读时注释已归给**键**，
  容器其实悄悄失去了它。现在注释打头它自己的 pair——先 `# -o`，再 `~: !- `——既让第一次发射就是
  不动点，又把注释留在解析器安放的那个节点上；此前这两条性质是相互牺牲的。在 `crash-7eb273bc`
  （24 字节）与 `crash-9733643a`（27 字节，各自化简到 13 字节）上实测：旧拼法需要第二轮，还会
  把注释挪到值上。两条既有 pin 随之外形改变（`crash-11ced252` 的未引号键、`crash-22cb5f67` 的
  引号键），且它们现在都额外断言“一轮之后映射仍然拥有这条注释”。这些形状的发射文本有变化；没有
  任何文档变得不可解析，且语料库里“一轮收敛”的断言覆盖全部 69 个 `yaml_roundtrip` 种子。
- **序列项里的空容器不再需要 dump 两次** —— `{}` 与 `[]` 没有块式写法，可两个写手都把它们放到
  `-` 下面另起一行，于是再读时成了*流式*节点，下一次 dump 又把它内联回来：`safe_dump([{}])` 产出
  `"- \n  {}\n"`，再 dump 它则得到 `"- {}\n"`。数据从来没错，是文本一直在动——而这正是 fuzz 档所断言的
  不变量。落在 dash 那一行才收敛，两个实现同时改：处理解析/编辑后树的 `Serializer::write_sequence_item`，
  以及面向 Python 对象的 `direct_dump` 快路径；二者按设计互为镜像而不共享代码。现在 `safe_dump([{}])`
  就是 `"- {}\n"`，`[]` 与嵌套情形同样如此，且对它们各自再 dump 一次即是不动点。由
  `pbt::tests::prop_mapping_order_preserved` 在 Linux 种子上以 CI 常规例数发现。
- **闭合只有标签的行之后，注释行仍守住自己那一列** —— 为了让注释留在它所属的节点上，写手会
  闭合那条只有标签的行，而它用**输出中的绝对偏移**记住这一行。写出简单键的行内注释时，文本被
  插入到该偏移之前，而偏移从未随之移动：于是闭合判断从错误的位置量起距离，认定挂起的行已不再
  紧邻，就把值留在了未完成状态；下一轮随即把后面的注释行读成该值自己的前导注释，注释便从第 0
  列滑进值的缩进里。在 `crash-5561902a`（88 字节，化简到 15：`b: ! #&` / `#e` / `? #!`）上实测：
  第一次发射现在写出 `! ~`，注释仍挂在它被解析时所归属的键上，而且第一次发射就是不动点。对已经写出的输出做插入时，
  位于插入点之后的偏移现在都会随之平移。
- **紧凑 `- key:` 行上的注释不再消失** —— `write_sequence_item` 的紧凑 dash 分支自己组装
  `key: value` 这一行，却只抄写了正文，因此 `write_mapping_pair` 所尊重的那几个注释槽位一次也
  没有走到。实测：`- a: !   # n` 在**第一次**发射时就丢了注释（输出 `- a: ! `）；CI 输入
  `crash-55c199ef`（25 字节）则晚一轮才丢，因为它的首次发射把注释放在写手会读的节点上，重读时
  又交给了键。无正文的值与带引号的键都是无辜的——直接位于文档之下的 `a: !   # n` 本来就守得住
  注释——所以这是一个根因，而非同一外观的别种现象。如今条目自身的注释栈写到 dash 之上，后一个
  pair 的注释栈写在 pair 缩进处，键的行内注释跟着它自己的 pair 行；原本正确的项外形不变。逐个
  撤掉这三处写入，恰好只有守卫它的那一条测试变红，因此没有任何槽位被记上了它并不具备的保护。
  同一次采样里的 `crash-5561902a`（88 字节）是另一个根因，仍保持开放。
- **键的行尾注释不再迁到值的行上** —— 当值为了容纳自己的前导注释而必须下移到独立行时，
  属于*键*的那条注释仍被追加到「最后结束的那一行」，也就是值的行。而重读时，只带 tag
  的标量行尾的注释会作为**前导**注释归到值上，于是注释每轮换一次主人，发射永不动点：
  `b: ! # &` 加 `#~` 先得到 `b:\n  # ~\n  !   # &`，重读又得到
  `b:\n  # ~\n  # &\n  ! `（libFuzzer `yaml_roundtrip` crash-1b01ac3f，93 字节最小化到
  11）。现在键的注释留在 `key:` 行——那既是读者会上报它的位置，也是单轮即达的不动点。
  代价可忽略：指令数门禁只动了 +0.05%。
- **合并不再重复映射已经拥有的键** —— 一个带着注释的未标记 `y` 与被合并进来的 `y` 是两个
  不同的 `IndexMap` 键，于是两者都活到了发射里，`to_yaml` 在同一层把 `y:` 打印了两次：这段
  文本被我们自己的解析器拒绝，破坏了引擎“绝不发射不可解析文本”的契约（`crash-3495cc86`，
  72 字节，最小化到 19 字节）。`prepend_merged_pairs` 长期声称它的输入“已由调用方按现有键
  过滤”；实际上没有任何过滤。现在展开会丢弃被映射自身覆盖的项 —— 身份取未标记标量键的
  *发射值*，与 `push_node` 重键检测用的是同一条规则，两者对“哪些键可以共存”永远不会有分歧
  —— 而被丢掉的项的注释会被重新安家，不会跟着一起消失（`former-crash-3495cc86.seed`，由
  `a_merge_never_repeats_a_key_the_mapping_owns` 与语料门禁共同钉住）。撤掉该覆盖只会让
  这两条变红，283 条里其余全绿。
- **只带属性的根节点旁边的注释不再被丢弃** —— `!x # note`、`&a # note` 以及 69 字节的 `!###0`
  注释墙一直在丢文本，而任何预言机都看不见它：一份稳定但少一条注释的文档仍然是一份完全稳定的
  文档。两种到达顺序，两个缺陷。granit 把这条注释投递在 `Scalar` 事件*之前*，此时
  `attach_inline_comment` 根本没有可挂的候选节点，却仍然回答“已处理”，于是调用方永远不会把它
  往后传；而在 `!m` CR `...` SP `# -o` 里，注释到达于 `DocumentEnd` *之后*，把它反向绑到已完成的
  根节点上当行内注释，产出的是 `!m   # -o` —— 这个拼法自己重读时会把注释报在节点之前并归为
  leading 注释，于是往返永远落不下来。现在只有真正完成绑定才可报告成功；跟在非容器文档结束之后
  的注释会被往后传；文档收尾时仍挂在 pending 槽里的注释会以 leading 注释的身份骑在根节点上，
  而不是被丢掉（11 字节 `former-crash-7918272c.seed`、69 字节 `former-crash-ce106ccc.seed`，由
  `a_note_beside_a_property_only_root_survives_and_settles` 钉住：五个形状加两枚种子，断言精确
  发射、文本存活与一步不动点）。把诚实的返回值撤掉只让这一条测试变红。容器根节点有意保留它的
  行内归属 —— `a: 1` + `# trailing note` 会从最后一行值上读回来，那正是钉住 `flush_trailing_comment`
  的两条测试所守的东西。
- **纯文本的 `#` 不再把注释从它该待的行上挤走** —— 写入器会把容器自己的行内注释贴在刚写完的那一行上，
  但它先要问“这一行是不是已经有 `#`”，而这个问题是按原始字节扫出来的。带引号的标量里含 `#`
  （`"+#": !-`）就会被答成“有”，于是注释被降级成单独一行——而值下面的裸注释行重读时会交给*后一个*
  节点当 leading 注释，第二轮就把它挪进了值块内部。现在这个扫描尊重引号与 YAML 的空格规则
  （`line_has_comment_marker`），注释便贴在键值对那一行，一次发射即是不动点（`former-crash-22cb5f67.seed`，
  15 字节，由 `a_quoted_hash_key_settles_the_containers_note_at_once` 与 `comment_marker_scan_respects_quoting`
  钉住）。撤掉新的扫描恰好只让那条端到端测试变红。
- **带标签的容器现在把首个条目标记脊柱上的每一叠注释都上提** —— 把注释从头行下方清出去的那个提升，
  原先只读第一个键*自己*的 leading 注释叠，于是骑在体内标记上的那一叠留在头行下面，被读取端在下一轮上提。
  现在这个提升取走整条脊柱（`former-crash-e6551c75.seed`，60 字节，以及 43 字节的 `former-crash-8f7085b0.seed`，
  由 `every_spine_note_clears_a_tagged_containers_header_line` 一起钉住）。台账曾以这条遍历会把注释从
  *嵌套*标记里拽出来、破坏已钉住的紧凑键形状为理由推迟它；把该遍历撤回并重跑后，变红的恰好只有一条测试，
  同族其余形状两种写法都通过，可见当时的影响范围是推断出来的而非测量出来的。
- **`pyrs-toml` 重新能在裸机目标上构建** —— 堆叠注释的那批改动把一个 `#![no_std]` crate 的解析器与写入器
  里放进了 `std::mem::take`；宿主上的每次构建都放过它，`no-std-check` 作业不会。现在六处调用改用
  `core::mem::take`，而 `cargo build --locked --no-default-features --target thumbv7em-none-eabi -p pyrs-ast
  -p pyrs-schema -p pyrs-json -p pyrs-toml` 在本地是绿的，那正是那个作业跑的命令。
- **带标签的容器不再把注释梅拆到自己的头行两侧** —— 把块容器首个条目的注释上提到它打印的
  anchor/tag 头行之上的那个提升，在容器自带注释时拒绝运行；因为在当时一个节点只有*单个*
  leading 槽，第二叠写在那里会把第一叠覆盖掉。那个槽现在是 `Vec`，于是这份拒绝不再保护文本，
  只是多花一轮：`# a` + `!5b4?` + `?` + `# b` + `k: v` 把 `# b` 写在头行之下，必须重读才被
  上提。先测量再动手 —— 这一族每个形状的不动点都是*所有*注释梅按源顺序位于头行之上 ——
  然后守卫直接移除而不是调参，并把原先钉住旧位置的特征测试重新推导：
  `notes_stack_above_a_tagged_containers_own_note` 现在断言顺序、两条注释都存活、一步不动点；
  `both_note_stacks_land_above_a_tag_header_in_one_round` 重放发现窗口给出的 32 字节载体
  `former-crash-fbc8f2ae.seed`。把守卫放回去恰好只让这两个测试变红。
- **标记脊柱上的每一叠注释都上提到标记行，不只是第一叠** —— `hoist_marker_note` 沿单行开启的
  `?` 标记链行走，原先在遇到的第一叠 leading 注释处停下。一条链可以带多叠，而 granit 把它们
  都报告在标记自己那一层：crash-f8525a9e 的脊柱深三层标记，`#` 在中间那个映射上、`!!"#~` 在
  最里层 `~` 键上，于是只上提一叠就把另一叠留在深一级的位置，发射要到第二轮才收敛。现在这个
  行走累积所有叠，先外层后内层 —— 这同时关闭了 99 字节的 crash-c9031de4，它的注释也在同一条
  脊柱上（`former-crash-f8525a9e.seed`、`former-crash-c9031de4.seed`，由
  `every_note_on_the_marker_spine_lifts_to_the_marker_line` 钉住）。这条输入的第一个假设 ——
  上提漏掉了存在 legacy `comment` 槽里的那条注释 —— 被检验并**否证**（发射一字未变）；
  `take_leading_notes` 仍改用归一化的 `leading_comments()` 视图，因为只读两种存放约定中的一种
  正是 `standalone_slice()` 要防止的分叉，但它只以这条理由自证，不记为本次修复的功劳。
- **容器自己的行内注释现在写在能容纳尾注的行上** —— 写入器会把块容器上非 standalone 的
  `comment` 打印成块下方一行裸注释，但读取器从不从空行报告行内注释：重读时那段文本被交给
  结束该块的那个节点，成为它的*前置*注释，于是第一次发射从来不是不动点。
  `:<TAB>!-<CR>... #-o` 先写出 `~: !- \n# -o\n`，要到第二轮才得到
  `~:\n  # -o\n  !- \n`（libFuzzer `yaml_roundtrip` crash-11ced252，13 字节）。现在注释借用
  块刚写完的那一行 —— `~: !-   # -o`，一步就稳定，也正是 granit 回读时报告它的位置。
  这个槽位的归属是在写行时记录的，而不是从输出文本猜的，因此三种情形拒绝借用：块标量的
  正文行（追加进去会变成内容）、折行的续行、以及已经带着注释的行。
  `a_containers_inline_note_after_a_text_less_value_settles_at_once` 钉住允许的一侧，
  `a_block_scalar_body_never_borrows_the_containers_note` 钉住拒绝的一侧；突变检查（把追加退回）
  只让前者变红，其余 274 个测试仍为绿。经由 TOML 中枢这也收紧了一条已记录的边界：
  `[sec] # note` 不再逃到文档开头，而是留在它自己的表里，作为该表最后一条 `key = value` 的
  行尾注释，同样一轮到不动点，所以 `TestSectionHeaderCommentBoundary` 被改写为钉住“存活 +
  留在表内 + 这一步稳定性”。
- **标记行现在执行它的两次注释上提** —— 显式键可以在键节点上带一条注释，*同时*留下第二条挂在
  键体首个条目上，而读取器把两条都报告在标记自己那一层。写入器原先用 `if`/`else if` 在两次
  上提之间二选一，所以只要键自有注释，体内那条就被写在深一级的位置，重读时又爬一级；发射要
  到第二轮才收敛（libFuzzer `yaml_roundtrip` crash-456176be，40 字节：`?` + `### standab:` +
  `?` + `# ! y%% yam2:#l: tr` + `~: ~`，其树把第一条注释放在键映射上、第二条放在内层
  `~` 键上）。现在两次上提按源顺序叠加在 `?` 之上，一次发射就到不动点 —— 由读取已播种子的
  `a_marker_carries_both_its_own_note_and_its_bodys_first_note` 钉住，归因方式相同：把它们拆回
  二选一只会让这一个测试变红。两条症状相同的输入 crash-f8525a9e 与 crash-c9031de4 在修复后仍
  是红的，所以它们是另一种几何（注释挂在嵌套*标记*上而非标量键上），保持未关闭。
- **注释文本里的 `&` 不再能把名字让给真正的锚点** —— granit 只交回数字 `anchor_id`，所以
  显示名要靠从节点内容向左扫回来还原；而那个还原只在“紧邻 `&` 的前一个 token 就是注释
  开启符”时才拒绝。注释正文可以包含任何东西：`bg: &b` 后面跟 `# !! &?`，重读就变成了
  `bg: &?`，因为 `&?` 前面是 `!!` 而不是紧贴着的 `#`。锚点被改名就会静默悬空所有指向它的
  别名 —— 与 #265、crash-04fddeb8 同一数据丢失类，也是本周期第三次“守护写得比它代表的规则
  更窄”。拒绝现在问 YAML 自己问的那个问题：这一行更靠前的位置是否开启了注释？藏在标量里
  的 `#` 仍然不算开启；唯一可能误拒的形状（同一行更早处有带引号的 `#`，而后又出现锚点）没有
  可达形式，因为节点属性总在值之前。由 `anchor_name_before_ignores_ampersand_anywhere_in_comment_text`
  （谓词的两个方向）与 `anchor_keeps_its_name_across_a_comment_line_holding_an_ampersand`
  钉住，后者从新种子 `fuzz/seeds/yaml_roundtrip/former-crash-68adf94c.seed` 读字节，并断言
  锚点记号、注释文本与一步不动点；该 artifact 重放 CRASH→CLEAN，文件里原有的其余守护仍成立。
- **键上方的每一行独立注释都会保留，而不是只留最后一行** —— AST 把 leading 注释放在单槽里
  （`NodeDecor.leading_comment: Option<Comment>`），而 YAML receiver、JSONC parser 与 merge
  阶段都会*覆盖*它，所以一叠注释行只剩一行：`# alpha` + `# beta` + `key: 1` 回写成
  `# beta` + `key: 1`。下游谁也看不见它 —— 丢失后的文本照样稳定，而往返档判据只问“再序列化后
  是否稳定”，所以那是那一档无法表达的第一类缺陷。`NodeDecor.leading_comments` 现在是有序列表；
  `NodeMeta::standalone_slice()` 是唯一的规范化读取（有列表就读列表，否则把旧的
  `comment(standalone = true)` 写法当作一元片——返回片而不是 `Vec`，因为 `NodeMeta::eq` /
  `Hash` 会在每个 mapping 的每次 `IndexMap` 探测上运行）。`leading_comment()` 与 Python 的
  `Node.leading_comment` 仍报第一条，既有行为不变；`Node.leading_comments` 是新的完整视图。
  顺这条线又找到三处同形状的静默丢失，每处都已修复、加测并播种：只含注释的文档会把注释全丢
  （`#&l<TAB><TAB>:` → `null`，因为没有节点时 `DocumentEnd` 不会触发）、null 键折叠会把被折叠
  那条的注释一并删掉、被消费的合并键（或它合并进来的那个映射）会把注释带走 —— 最后这处正是合并键
  身份修复刚刚暴露出来的洞。现在注释一律重新安放，绝不丢弃。TOML spoke 的 pending 槽是同一型覆盖（`# a` + `# b` + `k = 1`
  只留一条），空集合的单个行内槽也会丢掉一叠注释的其余部分（`# d1` + `# d2` → `{}  # d1`）；
  现在两者都全量保留。这也改了一条已记录边界：表格头行的注释仍然不能留在那一行，但不再消失，
  而是被搬到文档开头，且经 hub 走一轮 TOML 就已稳定。只有“注释存活”判据还等剩下的归属工作
  收尾，不会先把 CI 弄红。
- **合并键按它本身被识别，而不是按它携带什么** —— 合并阶段用整节点相等在 pair 表里查
  `<<`，所以键节点上挂了注释的 `<<` 对它完全隐形：`<<: #*` + `y:` 解成
  `{'<<': {'y': None}}`，而同一份文档写成 `<<:` + `y: ~  # *` 却解成 `{'y': None}`。
  正因为写入器会把注释在这两个位置之间搬来搬去，一次往返就改变了文档的含义，配对在下一
  轮直接消失（libFuzzer `yaml_roundtrip` crash-69931a77，`cargo fuzz tmin` 最小化到 10
  字节；crash-0a6fe677、crash-2d3dab18、crash-f88c2382 同时关闭 —— 把整节点相等放回去，
  四份输入一起变红，这才是归因而非同一条断言）。现在按 YAML 自己的解析方式匹配键 ——
  无 tag 的 plain `<<` —— 并按位置而不是按值定位该条目；这顺带修好了尾遍历在本体克隆与
  自有键相等时把克隆交回来的情形。风格与 tag 仍然决定身份：引号 `"<<"` 和带 tag 的
  `!x <<` 照旧是普通键，由 `a_quoted_or_tagged_merge_lookalike_stays_an_ordinary_key`
  与 `TestMergeKeyIdentityIgnoresMetadata` 钉住。已播种为
  `fuzz/seeds/yaml_roundtrip/former-crash-{69931a77,0a6fe677,2d3dab18,f88c2382}.seed`。
- **容器自己 tag 行之下的注释，不再和 tag 行互换位置** —— granit 会把写在 anchor/tag
  header 行*下方*的独立注释报成那个带 tag 节点的 leading 注释，所以写入器留在那里的
  注释下一轮就跑到 header *上方*，一次发射永远到不了不动点（libFuzzer
  `yaml_roundtrip` crash-77a8039b，28 字节：`!5b4?` 接 `# yrrrrrrrrrrrr%3c` 接 `~: ~`；
  实测要到第 2 轮才停下）。这类注释现在上提到 header 之上 —— reader 会把它交回的那一行
  —— 同时从正文副本里取走，免得写两遍；带 tag 的块序列首项同样上提。上提在 reader 槽位
  用尽处收口：自带注释的容器在 header 上方已占一行，再来一行会落进同一个 leading 单槽，
  所以那个形状保持原位置，不把漂移换成丢文本
  （`a_note_is_not_stacked_above_a_tagged_containers_own_note`）—— 并作为独立开放项记录，
  因为该输入今天确实实测会丢一条注释。已播种为
  `fuzz/seeds/yaml_roundtrip/former-crash-77a8039b.seed`。
- **尾部注释不再跨过自身节点吞掉的换行被错挂** —— granit 给块级集合的 span 会*越过*
  结束本行的那个换行，于是 span 末尾其实已经落在下一行。“这条注释是否在更后的行上”
  的检查只扫描候选结束字节与注释之间的缝隙，在 `?\n` 里看不到 `\n`，就把注释挂到了更
  深的节点上；写入器把它写进那个块里，重读时又交给浅一层的条目，所有权每往一轮就升一
  层（libFuzzer `yaml_roundtrip` crash-0e1c4378，最小化到 10 字节
  `b:<LF> ?<LF>? #i`）。现在该检查先把 span 吞掉的空白退回去，再找换行。两侧都有断言
  守住边界：crash-105de752（47 字节）随同一改动变 CLEAN，而归因是*测出来的* —— 关掉
  这个回退，它会与 crash-0e1c4378 一起重新变红，所以是同一根因而非同一条失败断言；
  `a_note_on_a_multi_line_nodes_last_line_still_trails_it` 则钉住反方向，因为跨多行节点
  最后一行上的注释仍必须属于那个节点。两份输入都已提交为
  `fuzz/seeds/yaml_roundtrip/former-crash-{0e1c4378,105de752}.seed`。
  方法记录，比修复本身更值钱：这条规则的第一版把 receiver 的“字符序号 → 字节偏移”表
  当成了行表来读，而纯 ASCII 输入下这张表根本不存在，于是它返回 `None`，什么也没改变。
  它读起来是对的，实际什么都没做；只有那条仍然发红的断言说了真话。
- **mapping 会折叠它的 null 键，且只折叠真正的 null 键** —— `~` 键与空键是同一个键，
  但 `IndexMap` 比较的是整个节点，两种拼写在元数据上不同，于是两条都留下、都渲染成
  `~:`，而 reader 回读时又把它们折叠——这类文档每往一轮就少一行（libFuzzer
  `yaml_roundtrip` crash-00e31785，最小化到 9 字节 `: &b #*\r:`）。现在入站阶段就按
  重读会得到的结果折叠。该折叠最初反而删了数据：`is_null_key` 只看标量文本，于是引号
  包起来的 `"NULL"` / `""` 键和带 tag 的 `!a null` 键也算成 null，`{"": None,
  "NULL": None}` 在 JSON5 与 TOML 往返中丢了空键（`tests/test_property_dialects.py`），
  proptest 又报 `!a null:` + `!A null:` 为假重复。两个谓词现在按 YAML 自己的问题发问：
  隐式类型解析只适用于无 tag 的 plain 标量。
- **空正文的块标量不再声明 chomping 指示符** —— granit 重读一个没有正文可依附的
  header 时会报回*默认* chomping，所以给空标量写 `|+` / `>+` 会在下一轮漂成 `|` /
  `>`，`to_yaml` 到不了不动点（libFuzzer `yaml_roundtrip` crash-89d81d99，5 字节
  `>+8<CR>#`；crash-b5dcc38f，55 字节，`ancho: |+`）。写入器出于完全相同的原因已经
  不再写*缩进*指示符，现在对 chomping 指示符也照样丢弃；并不丢信息——空正文没有尾部
  换行可供保留或剥除，AST 仍保存解析到的那个值。
- **跟在 mapping 键后的注释不再被静默丢弃** —— granit 会把夹在简单键与其 `:` 之间的
  注释报在*键*节点上，但一旦这对条目写成 `key: value`，那个位置就没有写法可言，YAML
  写入器直接丢了注释（`? a # note` + `: b` 只会输出 `a: b`）。现在它写到值后面 —— reader
  能报回这条注释的唯一槽位 —— 信息不再消失，且该行在那里就是不动点；值自带注释时仍归值，
  因为一行只有一个尾部槽。往返门禁看不见这类问题（文本本就稳定，只是少了条注释），所以由
  `a_note_trailing_a_key_survives` 与 `test_from_jsonc_keeps_comments_as_yaml_notes` 钉住。
  顺带发现仍有三处宣称 `from_jsonc` “会剥离注释”（自 #112/#115 起已不适用的说法）：
  绑定的 `from_jsonc` / `from_json5` 文档与生成的桩文件；改完 docstring 后，新的桩漂移门禁
  立刻报出陈旧的 `.pyi`，并按声明路径重生成而非手改。
- **紧跟在显式键标记行后的注释，写到 reader 实际报告的位置** —— granit 会把这类注释
  挂到比落点浅一层的节点上，所以缩进写法每往一轮就升一列，`to_yaml` 到不了不动点
  （libFuzzer `yaml_roundtrip` crash-ac5d9043，`cargo fuzz tmin` 最小化到 8 字节
  `? ? ? #~`；原输入里的 `&##` 锚点无关）。写入器现在把标记行的注释上提到自己那一
  层；granit 在独立一行上读到的注释仍原地不动 —— 那个几何本就往返得上，而且有断言
  钉住它，免得修复越界。
- **注释行不再把它的 `&` 让给锚点名** —— granit 只报告数字 `anchor_id`，所以显示名要靠
  从节点自身内容向左扫描、找最近边界 `&` 来还原（#265 收紧的是 tag 那条路）。夹在锚点与
  内容之间的独立注释从未被排除，而 `&` 是合法的注释文本：`chi&&&: &~:` 后面跟一行
  `# &l`，重读时锚点被改名成 `&l` —— 真名 `~:` 消失，指向它的别名全部静悄悄地悬空。这是与
  #265 同级的数据丢失，只是来自另一个可合法含 `&` 的记号。现在若某个 `&` 所在行已经开启了
  注释，就同 tag 内那样拒作候选；注释本身照旧保留。（libFuzzer `yaml_roundtrip`
  crash-04fddeb8。）
- **含流式指示符的 tag 后缀不再摧毁文档** — granit 交给 reader 的是*已解码*的后缀，
  所以源码里的 `!a%2cb` 到 AST 中是 `a,b`。tag 发射过去只重新编码 RFC 3986 禁止的字符，
  而 `,` `[` `]` `!` 都是合法的 URI 字符——但它们恰恰是 granit 的 `is_tag_char` 拒绝、
  使后缀扫描停下的位置，而 flow level 0 之后扫描器要求空白或换行，于是 `to_yaml` 的输出
  被我们自己的解析器直接拒绝（"while scanning a tag, did not find expected whitespace or
  line break"；libFuzzer `yaml_roundtrip` crash-e92ce66f，43 字节，与 `!5%2cy7` 同一根因）。
  写入字符集现在取自 reader 而非 URI 语法：简写 tag 对这四个做百分号编码，而 verbatim
  `!<uri>` 保留原样，因为那里的 `is_uri_char` 接受它们——`!<tag:yaml.org,2002:str>`
  仍逐字节往返。
- **Unicode 空白不再冒充 YAML 空白** — YAML 流水线上有六处用了
  `char::is_whitespace()` / `str::trim()`，那是 Unicode 定义，也会匹配 NBSP（U+00A0）、
  U+0085、U+2028/U+2029，而 YAML 从不把它们当作分隔符（granit 的空白集只有 SP 与 TAB）。
  后果是静默的而不是表面：只含一个 NBSP 的文档被“空文档”快速路径吞掉，重读成 `null`
  而不是标量（libFuzzer `yaml_roundtrip` crash-512814，5 字节：BOM 加一个 NBSP）；
  `resolve_core_type` 与 `resolve_yaml11_type` 把内容剪掉了，于是 `<NBSP>42` 解成*整数*
  42、`<NBSP>yes` 解成 `true`，而跨行的 NBSP 标量解成 `Null`，使 writer 跳过加引号、
  裸写换行，回读时空行塌缩——发射永远到不了不动点（crash-b44481b2，7 字节）；被折行
  排版的普通标量在换行处丢了 NBSP；`anchor_name_before` 把 `&a<NBSP>b` 截成 `&a`，
  让引用全名的别名静悄悄地悬空；注释文本两端的 NBSP 也被吃掉。六处现在统一用 reader 自己的
  集合 `pyrs_schema::is_yaml_blank`。JSON 家族解析器故意保留 Unicode 空白——JSON5 确实
  把它们当作结构性空白。
- **毗邻含 `&` 的 tag URI 的锚点不再丢名字** — granit 不报告锚点名，
  `anchor_name_before` 便从节点向左扫描最近的边界 `&` 来还原显示名。但 `&` 是
  合法的 URI 字符，而 `-`（`- &a v` 需要它）又在边界集里，于是对 writer 自己采用的
  `&anchor !tag` 顺序而言，最右侧符合条件的 `&` 其实落在 tag *内部*：`&F !-&l`
  被重读成锚点 `l`。名字每一轮都在变，发射永远到不了不动点；而锚点一旦被改名，
  引用它的 `*F` 别名就全部悬空——这是数据丢失级别的问题，不只是格式漂移。现在若某个
  `&` 左侧以空白分隔的那一段以 `!` 开头，就当作 tag 内容跳过（libFuzzer
  `yaml_roundtrip` crash-f44eca1d，36 字节最小化到 12 字节）。
- **双引号标量内的 BOM 会被转义而不是原样写出** — 转义器的兜底分支只测
  `is_control() || is_yaml_noncharacter()`，而 U+FEFF 两者都不满足（它是 `Cf`
  格式字符，非字符掩码也排除它），于是一路落到原样输出，把 BOM 直接写进引号内
  ——而 YAML 在这里本来有转义写法。解析器在输入阶段就会拒绝文档中间的 BOM，但
  edit API 直达 writer（`set("$.key","a<BOM>b")`），产生了无法再解析的输出；现在
  输出 `"a\ufeffb"` 并可原样读回。追 crash-2d14c6f6 时发现；注释与锚点位置根本没
  有转义语法，另由入站过滤处理。
- **注释或锚点内的字节序标记不再摧毁文档** — U+FEFF 被*限定*只能作为流自身的开头
  BOM，不得出现在文档内部。granit 会把它放在已解码的注释文本里，而我们自己的
  `anchor_name_before` 文本扫描器也会把它扫进锚点名。这两个位置都是裸发射（`# note`、
  `&name`）且没有任何转义语法，所以重新发出 BOM 会使输出被我们自己的解析器直接
  拒绝（"a BOM must not appear inside a document"；libFuzzer `yaml_roundtrip`
  crash-2d14c6f6，55 字节）。现在注释与锚点文本在入站时就被过滤为文档内字符，因此
  AST 是唯一的已安全形式，所有写入点由构造保证正确——与空注释（#248）和滞留注释
  （#256/#258）的“记录可重读形式”同一规则。注释保留其可读文本（`# a<FEFF>b` -> `# ab`）；
  若已无可读内容则整条丢弃，而不是发出无法解析的文本。
- **标签后缀写出时重新编码，使解码后的标签仍可解析** — granit 交给读端的是*已解码*的
  标签后缀，源码里的 `!y5%7c` 到达时已成 `y5|`。writer 把这段已解码文本原样发出，而 `|`
  不是标签中合法的字符，于是输出根本无法再解析（"while scanning a tag, did not find
  expected whitespace or line break")，往返立刻断裂（libFuzzer `yaml_roundtrip`
  crash-b91536ce，7 字节 `!y5%7c `）。现在标签发射会对标签 URI 字符集之外的字符做
  百分号编码（包含 `%` 本身，故字面百分号不会成为新转义的起点），还原成可读且每轮一
  致的拼写；无需转义的标签仍原样输出。
- **块条目破折号行上的注释绑定到它所注释的条目** — 行尾注释（`Placement::Right`）
  此前总是挂到最近创建的节点上，即使它位于更晚的行。在 `- :\u{feff}:\n- #e` 中，第二个
  条目破折号行上的注释落到了*第一个*条目的值上，于是 writer 把它溢出到第一个条目的块内；
  重读时又把它绑到第二个条目，导致所有权每轮翻转（libFuzzer `yaml_roundtrip` crash-aee06aca）。
  `attach_inline_comment` 现在比较注释所在行与回绑候选节点：同一行的注释仍按行内绑定，块标量
  头行之上的注释也仍会绑定（granit 把节点 span 落在其*内容*，比头行更晚），而更晚行的注释则
  前移作为下一个节点的前导注释。锚点、块标量与空容器槽位行为不变。
- **块容器的尾随注释可完整往返** — writer 无法把行内注释（`meta.comment`，
  `standalone = false`）挂在*块*映射或序列的同一行上（末项之后已无行可挂），
  于是把它另起一行写在末尾。重读时 granit 把这种形态报告为后面没有节点的独立
  注释，接收器又把它滞留在 pending 槽位里丢掉，导致第二次序列化丢失该注释
  （`&"\n-\r... #-o` -> `&" \n- ~\n# -o\n` -> `&" \n- ~\n`；libFuzzer `yaml_roundtrip`
  crash-96fa252c）。现在 `DocumentEnd` 时仍滞留下的注释会被回写到已完成文档的根
  节点——正是 writer 当初读取它的那个槽位——使往返既稳定又保留注释。
- **块头部探测锚定到标量的字节 span** — 头部再探测（#250）按解析器行号从
  内容行向上扫描并取第一个 `|`/`>`，于是键内的 `|`（带引号 `"k:yam  |1": |` 或普通
  `k:yam  |1: |2`）或内容行上的 sigil 被误当头部，解析出错误的缩进指示符并致 `|`↔`|1`
  轮次漂移（crash-cad17b2b，延伸自 crash-bdf3f15f）。现改为锚定标量自身的源码字节 span，
  只读其内容上方的那一物理行，取尾部符合块头部文法（至多一个缩进数字与一个 chomping
  符号，其后仅空格或 `#` 注释直到行末）的第一个 sigil。对所有形态均由构造保证正确、对
  granit 的 `\r` 行号位移免疫（它只把 `\n` 计为换行），并从块标量热路径移除向上逐行
  重扫的二次方开销。正常头部不受影响。
- **以文档指示符开头的普通标量也被引号包裹**（补全 #249）— 以 `... `/`---` 开头的值
  （如 `... k`）裸发在行首会被当作文档标记加非法尾随内容而无法重解析（crash-08f05e25）；
  现将该前缀形态一并引号化。
- **按宽度折行不再破坏长普通标量的空格** — 超过换行 `width` 的普通标量会在空格处
  折行，而折出的换行重解析会还原成单个空格。在 2 个及以上连续空格（或制表符）旁
  折行会留下行尾空格，重读成不同数量的空格，值因此逐轮漂移（libFuzzer
  `yaml_roundtrip` crash-9ee754bf）。`write_plain_scalar` 现在对含多空格串或制表符的
  值不再折行（发成一条无损长行）；仅含单空格的值仍照常折行且稳定，值始终精确。
- **块标量的缩进指示符改为在其内容行之上探测** — `detect_block_header` 从块首个
  内容行向上扫描，却从该行本身开始，于是含 `|`/`>` 的内容行可能被当成头部解析。
  granit 只把 `\n` 计为换行，故源里的 `\r` 会把 `key: |2` 与一个含 `|` 的内容行
  挤在同一逻辑行；重新发出时拆成 `\n`，使扫描命中的行发生偏移，`|2` 与 `|` 每轮
  互换（libFuzzer `yaml_roundtrip` crash-bdf3f15f）。现只在严格浅于块内容的行上
  定位头部，内容永不被误当头部；载荷性指示符（内容比声明更深）仍保留，值不变。
- **等于文档指示符的普通标量现被引号包裹** — granit 把带前导空格的 `...` 读作字符串
  `"..."`，而写入器曾把它裸发；行首的 `...` 是文档结束标记，于是该值重读成 null
  （`...` -> `null`）并每轮漂移（libFuzzer `yaml_roundtrip` crash-41acfbbe）。
  `needs_double_quoted` 现在把恰好等于 `...` 或 `---` 的值视为需要引号（`---` 早已被
  其前导 `-` 捕获）；其他普通标量不受影响。
- **无内容的注释不再被存储或发出** — granit 会把裸 `#` / `#` 作为一个空
  `Event::Comment` 报出，但重读时又不会读回它，于是写入器发出的一行 `#` 被下一次
  解析丢弃：一个多出来的 `#` 每序列化一轮就漂移（libFuzzer `yaml_roundtrip`
  crash-0de6be17）。AST 与流接收器现在都跳过 trim 后为空的注释，一个无话可说的
  注释既不被记录也不被发出；非空注释不受影响。
- **空的块标量不再携带多余的缩进指示符** — 空的 `|`/`>` 正文没有可供测量缩进
  的内容，granit 在重读时会丢弃显式指示符；而 `detect_block_header` 曾把 `|2` 的
  `2` 读进 AST，写入器又把它发回，于是 `|2` 每序列化一轮就漂移成 `|`（libFuzzer
  `yaml_roundtrip` crash-d4ea8a23）。两个块写入器现在在值为空时都省略缩进指示符，
  使空形态幂等；非空块标量的指示符保持不变。
- **映射键现在在发出时保留其锂点、标签与空键引号** — `write_scalar_for_key` 写出键的标量
  token，却丢弃了键节点的锂点与标签（值标量会发出的那些属性），并把空的普通键裸发，
  于是像 `&f& !&&f&&&  `（空字符串上的锂点与标签）这样的键被发成 `:` 并重读为 null `~`
  标量——锂点与标签丢失，往返从 `: ~` 漂移成 `~: ~`（libFuzzer `yaml_roundtrip`
  crash-62bcff6f）。键现在与值一样携带其锂点/标签，空键则加引号（`""`）以重读为空字符串
  而非 null。复杂键（`? `）本已正确——它们经由会发出属性的节点写入器。
- **锂点名按节点从 granit 自身锂点位置就地还原，不再整篇预扫描** — granit 不输出锂点的
  `&name` 文本，故解析器此前靠在原文上跑一个手写的引号/转义/注释状态机（`extract_anchors`）
  来还原名字，再用计数器（`anchor_name_idx`）把第 N 个扫到的名字配给第 N 个带锚点的事件。
  这一按位置的配对一旦状态机误判某个字节类——裸撇号（`bas'e`）、内嵌 `&`（`sbb&e`）、
  单引号反斜杠——便立刻错位，而每一类此前都各自是一个修复、一个 libFuzzer `yaml_roundtrip`
  崩溃，且都会连带错标其后所有锚点。预扫描已移除：granit 事件标出节点被锚定（`anchor_id != 0`）
  并给出其精确源 span，故名字现于该 span 处按 granit 扫描器同款的极大 `is_anchor_char` 游程
  就地读回（`anchor_name_before`），以 granit 权威 id 为键。还原是位置隔离的：某个不可读字节类
  只影响该节点，绝不会再挪动另一锚点的名字——整族漂移由构造终结，而非逐形状打补丁。它还从
  每次解析省掉一整趟文档扫描。锚名内含 BOM 的 emit 可表示性缺口是另一独立根因，另行跟踪。
- **反斜杠不再在锚点扫描中转义单引号标量的闭合撇号** — `extract_anchors` 以
  前在单引号内也运行转义态机。YAML 单引号标量没有转义处理器（只有 `''`），
  所以键闭合 `'` 前的 `\`（如反斜杠结尾的单引号键 `'a\'`）被读作转义了那个
  `'`，引号永不闭合，其后每个 `&锚点` 被隐藏——值的锚点在再解析时消失，往返
  发生漂移（libFuzzer `yaml_roundtrip` crash-12f01ee0）。现在转义仅适用于双
  引号内；无锚、单/双引号文档的扫描与 granit 读取完全一致。
- **嵌入普通标量内的 `&` 不再被读作锚点** — `extract_anchors` 会采集引号外的
  每个 `&`，包括嵌在普通标量里的（裸键 `sbb&e` 中的 `&`）。granit 只在节点
  可开始处开启锚点，那个幻影 `&e` 名被塞入有序 `anchor_names` 列表，错位了
  `register_anchor` 中基于索引的 id→name 配对：后续真锚点被错标（`&b` 重发为
  `&e:`），往返发生漂移（libFuzzer `yaml_roundtrip` crash-83cc68c6）。现在锚点
  提取对 `&` 施加与引号状态机相同的节点边界门控（行首或 `\t:,[]{}-` 之后），
  故 `sbb&e` 仍是普通键。无锚与正确锚定的文档扫描结果不变。
- **重复键按值而非整个节点拒绝** — AST 的 `IndexMap` 以整个 `CustomNode`
  为键，因此文本相同但后随注释/风格/锚点不同的两个标量键（`key # a` 与
  `key # b`）仍被视为互异：解析时不报重复，而序列化器丢弃键装饰并发出两行
  完全相同的 `key:`，导致我方 parser 在再解析时拒绝（libFuzzer
  `yaml_roundtrip` crash-3b0a7d1d——输出的文档无法解析）。重复键检测现按标量
  键的值识别，与 `to_yaml` 发出的同一身份，故此类输入在首次解析即被拒绝。
  `<<` 合并键仍豁免：YAML 允许一个映射重复它。
- **空块容器作为映射值时内联序列化** — 空的 `Mapping`/`Sequence` 没有块形式，
  但块式的空值被发出为 `key:` 并将 `{}`/`[]` 放在下一个缩进处。重新读取会得
  到一个*流式*集合，于是 `flow_style` 翻转，下一轮把它内联了——`key:\n  {}` 与
  `key: {}` 每轮序列化都漂移（libFuzzer `yaml_roundtrip` crash-d0e84310）。现在
  空容器总是内联发出（`key: {}`），包括带锚点/标签的值（`key: &a {}`）；对其
  跳过换行预发，故报头永不重复。
- **普通键中的裸撇号吞掉了后续所有锚点** — `extract_anchors` 运行一个引号
  状态机以跳过被引号包裹的 `&`，但它对任意 `'`/`"` 都翻转状态，即便它嵌在
  普通标量里（如裸键 `bas'e` 或 `a'` 中的 `'`）。那个幻影引号会持续到文档
  结尾，导致预扫描不返回任何锚名、`register_anchor` 给每个节点都返回 `None`，
  锚点静默地从输出中消失，往返发生漂移（libFuzzer `yaml_roundtrip`
  crash-68da2420）。现在引号的“开启”受 token 边界门控（行首或 `\t:,[]{}-` 之后），
  与 granit 一致；普通标量内的引号是字面内容，而真带引号的标量仍会遮蔽其 `&`。
- **字面块标量在首行为空白时强制缩进指示符** — AST 将 `|`/`|N` 正文去缩进
  存储，井丢弃源头的显式指示符；因此首行以空白开头、后续行更浅的值
  （` 1|l\n:t\n`）在无指示符重新发出时，会让 granit 把更深的首行当作块缩进，
  将更浅的那行读成降级缩进——输出不再可重新解析（libFuzzer `yaml_roundtrip`
  crash-e432d4b8）。字面写入器现在镜像折叠写入器，止好在该情形下强制缩进
  指示符，从而跳过自动探测、使前导空白保留为内容。首行非空白的文档仍按字节
  相同输出。
- **折叠写入器保留了 more-indented 行之后的 break** — granit 的折叠规则维护一个
  `leading_blank` 标志：more-indented 行（以空格或制表符开头的续行）不仅自留其
  前导 break，还会置位该标志使其后那行的 break 同样不被折叠。写入端的连段规则
  此前只认前半段：它按上一行来判断抑制，导致 more-indented 行后接普通行时连段被
  多补一个换行，每序列化一轮就多吞一个空行（libFuzzer `yaml_roundtrip`
  crash-b7a2285e）。规则现改为按刚写出的那一行判断，触碰任一侧 more-indented 邻居
  的 r 换行连段恰好发出 r 个物理换行；以完整值保真（再解析保留标量值）钉住，
  而非仅字节幂等。
- **折叠标量如今能读回自身换行** — granit 的 folded 读取无论行首还是文本行之间，
  都把 k 个空行读回为恰好 k 个换行；而按行切分的写入器每个连段少写一个空行，
  折叠值因此每轮少一个换行（libFuzzer `yaml_roundtrip` crash-490c4beb：
  4 → 3 → 2 → …；crash-6288e5be 为前导空行同型漂移）。写入器现已感知折叠语义：
  r 个换行的连段占 r 个空行（前导计入头换行；more-indented 续行会自留其
  break，故少一空行），任意连长由构造封闭（内部、前导与 more-indented 的
  1 至 5 连段均已验证稳定）。
- **块标量输出如今在再解析下封闭** — 两种 granit 读取形状此前不被序列化器
  匹配：尾部含空行的 `Clip` 块标量值只能在 `Keep` 指示符下往返（Clip 读取
  会剥掉尾部空行——它是任何文档位置都能读回同值的唯一头形式；libFuzzer
  `yaml_roundtrip` crash-c18cb1fd），故输出时提升之；块标量的行内注释现在
  搭载在头行（`y: |  # c`）而非独立成行（此前会被吸收为块内容；
  crash-cfb3fa83）。两条规则都是纯输出侧规范化：此前稳定的所有文档仍
  逐字节一致地序列化。
- **锚点名语法对齐 granit，一次性根治整个漂移族** —
  `extract_anchors`/`scan_anchor_name` 长出了两个 granit 扫描器并不有的手写分支：
  引号锚形式（`&"a b"` 含空格）与值指示规则（空格/EOL 前的 `:` 终止名字）。granit
  把名字读作一段极大 `is_anchor_char` 游程（`:`/`#`/`"`/`&` 都是普通名字字符，
  仅在空白/换行/流指示符处终止——granit 自己的 issue14 测试）。两套语法每一次分歧
  都会错位 id↔名 配对、破坏往返；下方四条（#215/#218/#227/#228）都是同一因的
  症状。现在扫描器与 granit 完全一致（极大游程 + 原子跳过整个锚 token，名字内的
  `"`/`#` 不再扰乱引用/注释状态），`write_anchor_tag` 裸发 `&name`——闭包由构造
  成立，四个逐形状的补丁被吸收；本就无法往返的引号锚被删除。
- **引号锚点名曾吞掉换行** — `scan_anchor_name` 的引号分支把缓冲区中任意靠后
  的 `"` 当作闭合引号，于是 `&"X-<CR>:&"X-` 越过回车把名字读成 `X-\r:&`。序列化器
  原样发出它，重新解析时每轮多裹一层（一个不断增长的 libFuzzer `yaml_roundtrip`
  非幂等，11 字节）。granit 在 CR/LF 处结束锚点 token，因此越过行终止符的闭合引号
  不再算作引号锚点——名字保持单行、再发出也稳定。
- **嵌套自引用合并锚点撑爆原生栈** — 用 `&b` 锚定的映射，其主体重新引用
  `*b`（直接或间接通过第二个 `&b`）时，会在路径环守卫已弹出的情况下进入
  `resolve_mapping_merges` 的尾递归，导致每轮遍历都重新展开锚点的一份新克隆、
  递归深度无界增长（libFuzzer `parse_yaml`，58 字节 `bas: &b … <<: *b …`）。
  如今尾遍历只递归进入映射*自身*的子节点（并入的克隆已在展开循环中受守卫解析），
  并以 `MAX_MERGE_DEPTH` 预算把任何残留失控转为优雅停止，与解析器容器深度和
  序列化器 `max_depth` 守卫保持一致。
- **以 `:` 结尾的锚点名曾被以不稳定形式发出** — `write_anchor_tag` 把每个
  锚点都写成裸的 `&name` 标记。当解析出的锚点名以 `:` 结尾（经由未闭合的引号
  锚点如 `&"X-::…:` 得到）时，末尾的 `:` 与随后发出的空格合并成值指示符、在
  重新扫描时被丢弃，于是每轮序列化都少一个字符——一个 42 字节的 libFuzzer
  `yaml_roundtrip` 发现，`fmt(fmt(x)) != fmt(x)`。现在不安全的名字（以 `:` 结尾、
  含空白或流指示符）会以带引号的 `&"name"` 锚点发出，原始扫描器读到闭合引号
  为止，从而跨轮保留确切字节。
- **原始锚点扫描器会发明文本无法保留的锚点** — `extract_anchors` 把后接
  空格/行尾的 `:` 收进锚点名（`&&&&:` → `&&&:`），从注释文本里扫锚点，
  并在已接受锚点名内部重扫重叠的 `&`（`&&&&` 产生幽灵锚点 `&&&`、`&&`、
  `&`），使后续所有 id→名称配对错位。每种缺陷都会让序列化文档重解析成
  不同的锚名——libFuzzer 发现的 12 字节输入 `&&&&:<LF>#&&&:&` 每轮漂移
  一个字符。现在：值指示冒号处截断名字、跳过注释文本、已接受锚点 token
  不再重复扫描。
- **双引号标量被解码了两次** — granit 交付的双引号值已完成转义解码，
  但两个 receiver 又对其跑了一遍 `unescape_double_quoted`：`a: "\\n"`
  （字面的两个字符 `\` `n`）被静默压缩成换行符，且每序列化/重解析一轮
  就少一个反斜杠（libFuzzer `yaml_roundtrip`：`!-# \\f"<TAB>0:!`）。
  两处调用点现在都是直接透传，stream/AST 单元测试钉住单次解码契约。
- **未闭合引号的锚点名吞掉了行内剩余内容** — 面对 `&"X-<CR>:`，
  `extract_anchors` 的 quoted 扫描因始终等不到闭合引号而一路收集到行尾，
  把裸 CR 和冒号收进锚点名；序列化器原样输出 `&X-\r:`，而 granit 在空白处
  截断锚名，重解析得到 `X-` —— `fuzz/yaml_roundtrip` 用 6 字节输入打破了
  序列化幂等（`fmt(fmt(x)) == fmt(x)`）。未闭合的 `"` 现在恰好停在 granit
  unquoted 锚点 token 停止的那个字符；真正的 `&"quoted anchor"` 名字
  （含空格）保持不变。
- **JSON 注释扫描器可能在多字节字符中间 panic** — `ws()` 的行注释与未闭合
  块注释扫描逐字节推进 `pos`，后继多字节字符（如 U+FEFF）会把 pos 留在字符
  内部，下一个 `&text[pos..]` 切片以 "not a char boundary" panic（由
  `fuzz/parse_json` 在约 25 秒内发现：`\r\r{aMNaN/*0\u{feff}`）。行注释现在
  按完整码点前进，未闭合块注释回退到其 `/` 处，所有失败路径重新变为类型化错误。
- **Linux 免线程（`cp314t`）wheel 随 Release 发布** — wheel 矩阵此前只为
  Windows 和 macOS 构建免线程产物，Linux 上的无 GIL 解释器用户无从安装：
  带 GIL 的 `cp38-abi3` wheel 与 `Py_GIL_DISABLED` 构建 ABI 不兼容，而
  `abi3t` wheel 要到 CPython 3.15 才生效。`linux` 作业现在为 x86_64
  使用镜像自带的免线程解释器构建 manylinux cp314t wheel，并在 `3.14t` venv
  中冒烟测试通过后才附加到 Release（aarch64 暂不纳入：非 abi3 wheel 构建
  必须实际执行目标解释器，而该执行在 qemu-user 下失败）。
- **`pyq` 发布作业现在真正产出 Linux 构件** — 跨架构腿为 `cross` 的模拟
  容器注册 qemu binfmt 处理程序，其冒烟测试在 manylinux 镜像*内部*执行刚
  构建的二进制：主机有 qemu 翻译器却没有外族 `/lib/ld-linux-*.so` 加载器，
  直接在主机 exec aarch64/armv7 二进制会在 `main` 之前死亡。现在每条腿在
  上传压缩包前都能构建并自验证。
- **块标量保留显式缩进指示器** — 写作 `key: |2` 的正文在序列化时 `2` 被静默丢弃，
  于是当正文首行比后续行缩进更深时（正是 `4RWC.yaml` 的形状：首行 6、后续 4），输出的
  重解析结果与输入不同。无指示器时读取器按正文首行自动探测缩进，所以丢失它的是语义
  变更而非外观问题。指示器现在随 AST 保存并按规范顺序（`c-b-block-header`：chomping
  在前、缩进在后，故 strip 形式为 `|-2`）重新输出，且写入器以承载 header 的那一行为
  基准衡量正文，而非父节点的列。

#### 性能

- **tag 发射改用查表，转义不再分配内存** —— 把 tag 编码对齐到 reader 自己的字符类之后，
  逐字节归属测试就落到了序列化热路径上，于是它变成一张编译期的 128 项成员表（字母数字判定
  折叠进同一次查表），`%XX` 转义也改由十六进制数字表拼出，而 `format!` 会为**每一个**被转义的
  字节新分配一个 `String`。在同一进程内交替跑 best-of-6×40 批、以序列化器真实遇到的后缀语料测量：
  每 11 后缀扫一遍从 10.53 ns 降到 2.55 ns（4.1 倍）。两处相关替换同样更快：schema 解析器的边缘
  裁剪 0.47 → 0.20 ns（2.3 倍，YAML 空白集只是五次比较，而 Unicode `trim` 要查字符属性表），
  锚点名字符判定在加上 ASCII 快速路径后 1.61 → 1.14 ns（1.4 倍）。
- **大量 null 键的文档以线性时间解析** —— 上面那条折叠起初每遇到一个 null 键就重扫
  整个 mapping。在全 null 文档上看不出来（被折叠的条目就坐在 0 号槽），但在真正要紧的
  形状上是二次方：2k 个不同键后接 2k 个 null 键，输入变大 4 倍时耗时变大 12.4 倍
  （8k + 8k 要 99 毫秒）。现在 mapping 记住自己 null 键的槽位，扫描只作为正确性兜底：
  同一输入 11.0 毫秒、增长 4.14 倍 —— 与不同键文档的斜率（4.02 倍）一致。
- **本机跨进程的 divan 表无法判定 10% 以内的问题** —— 同一二进制逐轮相差可达 ±38%
  （`parse_medium` 一批读成 +77%、另一批读成 −20%），因此上面的谓词数字取自同进程 A/B，
  端到端结论交给每个触及 `crates/**` 的 PR 都会跑的 CodSpeed 门禁。`cargo nextest run --all`
  保持 449/449，全部已提交 fuzz 种子重放字节不变。

### [v0.17.0] — 2026-10-01

#### 新增

- **pyq CLI 对齐参数** — `pyq fmt` 新增 `--indent N`（块缩进，默认 2）、
  `--width N`（纯量软换行列宽，0 为关闭）、`--sort-keys`（序列化器级
  全文键排序）与 `-i/--inplace`（就地改写文件），暴露
  `pyrs-yaml-core::SerializeOptions` 全部旋钮并对齐 Python CLI 的
  `fmt --indent`。`pyq to-json` 新增互斥的 `--jsonc` / `--json5` 方言
  输出，接入 `pyrs-json` 的注释保留与 JSON5 写法序列化器
  （`to_jsonc_text*`、`to_json5_text*`）。
- **pyq JSONC/JSON5 输入方言** — `Format` 枚举新增 `--input jsonc|json5`
  （自动识别同样支持 `.jsonc` / `.json5` 扩展名），经由 `pyrs-json`
  原生方言解析器，注释与 JSON5 写法挂在 AST 上；配合 `to-json --jsonc`
  一条命令即可完成保留注释的 JSONC→JSONC 往返。`--all-docs` 对单文档
  方言报出稳定错误信息。
- **`pyq diff` / `pyq merge`** — 原生 CLI 新增语义文档对比与右侧优先的
  深度合并（yq `*+` 形态）。`diff` 遍历两棵 AST，比较解析后的值、结构
  与标签（注释/引号/排版永不出现），输出 `-`/`+`/`~` 路径行，相同退 0、
  有差异退 1。`merge` 递归叠加映射、追加倍列（`--replace-arrays` 整体
  替换），输出往返保留的 YAML；两命令均可通过 `--input`/扩展名识别读取
  任意支持的输入方言。

#### 变更

- **GitHub Release 改由 `publish.yml` 自动创建** — 过去每次发布后都需人工
  执行 `gh release create`，既多一个容易遗忘的步骤，也让已发布版本与 tag
  多一个漂移点。现在 `release` job 在 `uv publish` 成功后自动执行
  `gh release create`，复用同一个 `refs/tags/` 条件，自动生成 release notes
  并附上构建出的 wheel —— notes 与产物均来自发布到 PyPI 的那个 tag。
  `workflow_dispatch` 运行行为不变（既不发布 PyPI 也不建 Release），
  与原有行为一致。
- **`README.md` / `README.zh-CN.md` 补齐原生 `pyq` CLI 文档** — 两份 README
  在 Python CLI 小节旁新增 `pyq` 小节：从源码安装方式、三条实际可跑的示例，
  以及完整命令清单，并指向 pyq 指南获取细节。

#### 修复

- **`pyrs-json` 模块文档** — 旧文案仍声称注释读入即丢、不再回写；
  自 #122 起注释挂在 AST 注释槽位上，并由 JSONC/JSON5 序列化器还原。

### [v0.16.0] — 2026-10-01

#### 新增

- **JSONC 块注释热点基准** — 目标 §测试覆盖 5 将「block-comment」
  列为必需热点样本；之前仅行内 `//` 注释入基。新语料驱动
  `test_load_jsonc_block_comments`：50 对 pair + header/footer，
  每 pair 一个独立 `/* item N */` 以及一个尾追 `value /* trailing */`，
  块扫描回归从此在 CodSpeed 上显形。
- **YAML 的 PyYAML + ruamel.yaml 跨库对拍** — 目标 §测试覆盖 3
  将两库点名作为 oracle；之前仅在 `test_benchmark_crosslib.py`
  用于基准与特性支持 printout，从未做**正确性**断言。
  `tests/test_yaml_crosslib.py` 20 个规范文档 × 5 个对拍面 +
  2 个文档化分歧（重复键严格、YAML 1.1 传统 bool schema 域）= 122
  测试。可选依赖 `skipif` 自动降级。
- **`load_toml` tomlkit 跨库对拍** — 目标 §测试覆盖 3 点名 tomlkit
  作为 oracle；之前 tomlkit 仅在 benchmark 出现。`tests/test_toml_crosslib.py`
  新增 24 个测试，覆盖 11 种规范构造的三方对齐（pyrs / tomlkit /
  tomllib），将 `>i64` 拒收钉为规范严格（TOML v1.0 §Integers：
  64 位有符号），并断言 `-2^63` 边界（PR #174 修复）。可选依赖，
  `skipif` 自动降级。
- **orjson 作为 `load_json` 的严格 JSON oracle** — 目标 §测试
  覆盖 3 要求与 orjson 逐位对比；之前 orjson 仅用于基准。
  16 个规范文档断言逐字节对齐，12 个非规范形式（注释、尾逗号、
  单引号、裸 `NaN`/`Infinity`/`-Infinity`、十六进制、前导 0、
  `+.5`、`5.`）两侧一同拒收。orjson 拒绝 stdlib `json.loads` 在
  `allow_nan=True` 下默认的裸字面量，严格度上高于 stdlib，
  作为 RFC 8259 oracle 比 json.loads 更接近标准。可选依赖，
  `skipif` 自动降级。
- **CLI ↔ Binding 对等 gate（`tests/test_cli_binding_parity.py`）** —
  Pillar 1「CLI 与 Python Binding 两端均需具备同等功能」从文档声明
  升级为可执行契约：CLI 注册命令需与 18 命令固定清单对齐（过滤 cyclopts
  的 `--help`/`-h`/`--version` 伪命令）；每个 `to-X` / `from-X` 动词必须
  有对应的 `YamlDocument.to_X` / `from_X` / `load_X`；断言 `load_*` 家族
  （json/jsonc/json5/toml）四兄弟齐全；编辑/validate/compliance 动词都
  映射到实时 Python API。两侧任何一者漂移现在都会破 CI。
- **`load_json` 属性测试 + CodSpeed 基准** — Hypothesis
  （`test_load_json_matches_stdlib_json` 与
  `test_load_json_matches_load_jsonc_on_strict_domain`）为每个生成的
  规范文档固定严格 loader 与 `json.loads` 对齐，并断言两个 loader 在
  strict 域逐字节等价；快速路径越权或回退漂移将作为属性失败暴露。
  三个 CodSpeed wall-time 基准（`test_load_json_large` / `_floats` /
  `_escapes`）镜像 `load_jsonc` 样本，将严格 binding 层纳入回归追踪。
- **`load_json`（严格）——补齐 `load_*` 家族对称** — binding 已有 `load_jsonc` /
  `load_json5` / `load_toml`，唯独缺严格 RFC 8259 对应物。`pyrs_yaml.load_json(s)`
  在规范输入上与 `json.loads` 逐位对齐，并对 JSONC/JSON5 扩展（`//`、`/* */`、
  尾逗号、单引号、裸 `Infinity`/`NaN`、`0x…`）抛出类型化 `YamlParseError`。快速
  路径与 `load_jsonc` 共用 `json_fast::try_load`（任何非规范字节即 bail，零语法
  越权风险）；被拒对象走 STRICT `from_json` AST 解析。这就此补完下方 CLI ↔
  Binding 对等声明中的最后缺口：CLI 每种格式都有对应的 `load_*` 兄弟——支柱 1
  完成。已从 `pyrs_yaml.__init__` 重新导出并入 `__all__`；`.pyi` 通过
  `maturin generate-stubs` 重生成。
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
- **文本级重解析门控（`prop_output_always_parses`）** — Rust proptest 套件现在断言
  每个生成的 AST 序列化后都能被解析器重新接受。此前的 AST 对 AST round-trip 属性
  会静默跳过序列化文本无法重解析的形态（`try_roundtrip` 返回 `None`），使整类
  defect 不可见；新门控首轮就抓到六个真实的 serializer bug（见下方修复条目），
  每个都已由针对性 Rust 单元测试和 Python 回归类
  （`tests/test_roundtrip_bugs.py` 的 `TestNestedBlockScalarIndent`）钉住。
- **toml-test 一致性测试框架** — `tests/test_toml_test_suite.py` 以与 `test_yaml_suite.py`
  运行 YAML 套件相同的方式运行官方 [toml-test](https://github.com/toml-lang/toml-test) 语料：
  未跟踪的本地工件、缺失时 `skipif`、实测下限阈值，并用类型标签适配器做解码比对。
- **TOML 时间类型正确解码** — 仅日期与仅时间的值现携带不同的 `!date`/`!time` 标签（日期时间仍用
  `!timestamp`），从而走 `date`/`time.fromisoformat` 而不再崩溃。toml-test 发现裸时间（`07:32:00`）、
  省略秒的时间（`13:37`）以及小写分隔符的日期时间（`1987-07-05t17:45:00z`）在有效 TOML 上抛
  `ValueError`；`!time` 现补齐省略的秒，`!timestamp` 规范小写 `t`/`z`。
- **TOML 控制字符严格性** — 基本、字面与多行字符串内现在拒绝原始 C0 控 制码（NUL、FF、DLE、US 等）与 DEL（U+007F），仅保留制表符（及多行形式中的换行）。
  toml-test 的 `invalid/control` 语料暴露了 13 份被误承受的文档；注释体与 裸 CR 检查列为后续项。
- **TOML 数字字面量严格性** — 前导零十进制（`01`、`-01`）、给进位前缀整数加符号（`+0x1F`、`-0b101` — `signed-int` 只支持十进制），以及尾随/双下划线（`1_`、`1__0`）现均被拒绝。toml-test 的 `invalid/integer` 与 `invalid/float` 暴露 23 份误承受（总数 71 -> 48）；此前“进位整数可带符号”属违反规范。
- **TOML 内联表键冲突严格性** — 内联表现拒绝与已定义路径相等、扩展或被其遮蔽的点号键（`{ a = 1, a.b = 2 }`、`{ a.b = 1, a.b.c = 2 }`）；同属相路径（`{ a.b = 1, a.c = 2 }`）仍合法。toml-test `invalid/inline-table` 的 duplicate-key/overwrite 暴露此类（误接受总数 48 -> 39）。
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
- **JSON5 Unicode 标识符键** — 无引号对象键不再仅限 ASCII，现在接受完整的
  Unicode `ID_Start` / `ID_Continue` 集合，因此 `from_json5` / `load_json5`
  可解析 `{ é: 1, 名: 2, हिन्दी: 3 }`。基于 `unicode-ident` 表（rustc
  自身的词法分析器所用的 crate）实现，逐文字精确符合规范，含标识符中间的结合
  字符。仅在 JSON5 模式下启用，因此严格的 `from_json` 与 `from_jsonc` 仍要求
  为此类键加引号。`\uXXXX` 转义中的孤立 UTF-16 代理项仍被拒绝（Rust `String`
  无法无损表示）。新增依赖 `unicode-ident`。
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
- **JSON5 字符串转义 `\v` 与 `\0`** — `from_json5` 现在接受
  垂直制表符（`\v`）与 NUL（`\0`），双引号与单引号字符串均可；
  严格 JSON / JSONC 仍拒绝。与 #120（数值）、#124（数值语义）
  一起补齐 JSON5 语法。
- **load_json5 的 JSON5 数值语义** — `load_json5` 现在把 JSON5
  独有的数值形式（`0x1F`→31、`+7`→7、`5.`→5.0、`Infinity`/`NaN`）
  解析为真正的数字（新增 `Schema::Json5`），严格 JSON/JSONC 加载
  器不变，`to_json5_text` 仍按源文拼写输出。
- **JSON5/JSONC 开放到公开 API** — `pyrs_yaml.from_json5` /
  `load_json5`，以及 `YamlDocument.to_jsonc()` / `to_json5()`
  （走原生引擎，注释与 JSON5 风格不丢）。同时修正一个可达性
  缺口：`from_jsonc` / `load_jsonc` 以前未从 `pyrs_yaml` 包重导出，
  `pyrs_yaml.from_jsonc(...)` 会报 `AttributeError`；现已列入
  `__all__`。`to_jsonc`/`to_json5` 新增 `emit_root_leading`，保留文档级
  独立注释。`test_benchmark_api.py` 补充了 JSON 家族基准。
- **JSON5 writer（`to_json5_text` / `to_json5_text_pretty`）** —
  契约 B 第 2 步。把 AST 序列化回 JSON5，还原解析器保留的
  单引号字符串与 `0x…`/`.5`/`+7`/`Infinity`/`NaN` 数字形式，
  并输出 `//` 注释。key 总是加引号（无损）。内部共用一个
  `Mode`（Json/Jsonc/Json5）；严格与 JSONC 输出不变。
- **JSON5 数值形式解析** — `from_json5`（新增
  `allow_json5_numbers` 轴）现在接受十六进制（`0xDECAF`）、前/后小数点
  （`.5`、`5.`）、前导 `+`（`+7`）、前导零（`07`）以及裸 `Infinity` /
  `NaN` / `-Infinity`，每项都保留原文本供后续 JSON5 writer 使
  用。STRICT / JSONC 仍默认关闭该轴，像以前一样拒绝。同时修正了
  `from_jsonc` 陈旧的“注释被丢弃”文档（#112/#115 后注释已保真）。
- **TOML inline table 内部注释保真** — PR #119 捕捉 inline
  table 内部的 `# ...` 注释（成员上方独立行 → leading，值同行
  后面 → trailing）并贯串 IR，使它们能往返而不丢失。无装饰
  的 inline table 保持紧凑单行形式；嵌套在数组内的有装饰项
  会提升为多行。同时修复了 #114 遗留 bug：`skip_all_blank` 将
  独立注释自身的终止换行误判为空行。
- **YAML receiver 将独立行注释写入 `decor.leading_comment`**
  — PR #117b 把最后一个引擎（granit-parser receiver）迁到 #114 /
  #115 确立的新槽。scalar / mapping / sequence 的独立行注释现在落在
  `NodeMeta::decor.leading_comment`，不再写旧的
  `comment(standalone = true)`。手建 fixture 因 #117 的规范化保持
  相等；`CustomNode::remove_comment` 同时清空**两槽**，保证
  Python `Node.remove_comment()` 在 YAML 文档上行为不变。
- **跨槽独立行注释规范化 + Python `Node.leading_comment`**
  — `NodeMeta::eq` / `Hash` 现在把“独立行注释”视为单一概念，无论
  它存于新的 `leading_comment` 槽（TOML / JSON 引擎使用）还是旧的
  `comment(standalone = true)` 槽（YAML receiver 与手建 fixture 仍
  在用）。setter / remover 对两槽原子操作，YAML serializer 读规范化
  视图，`to_yaml(toml_ast)` 不再丢头注。Python
  `Node.leading_comment` getter / setter / remover 镜像
  `Node.comment`，TOML / JSONC 来文档的独立行注释首次对 Python 调用方可见。
- **TOML 1.1.0 语法** — `from_toml` 解析到 TOML v1.1.0（2025-12-18
  发布）。四项新增：**(A1)** inline table 可跳行 + 允许尾逗号；
  **(A2)** 基本字符串中 `\xHH` 字节转义（0x00..=0xFF）；**(A3)**
  `\e` = U+001B；**(A4)** time / date-time 秒可选（`t = 14:15`、
  `dt = 2010-02-03 14:15`）。`TomlDialect::V1_0` 与 `from_toml_v1_0`
  保留为严格 1.0.0 逃生舱；1.0.0 文档两种方言下解析结果一致。
  同时修复 `space_time_sep` 检测中的索引 off-by-one（导致无 `T`
  分隔的 date-time 在 1.0 模式下也未能识别）。
- **JSON 双槽注释保真** — JSONC 解析器现在把独立行 `// ...` 注释
  写入 #114 引入的 `leading_comment` 新槽，同行行尾 `// trailing`
  仍留在 `comment`。对象成员与数组元素因此可同时拥有两个注释，
  这是 #112 单槽模型不能表达的形状。`to_jsonc_text_pretty` 优先
  读新槽，回退到 `comment` 十 `standalone = true` 保持手建 fixture
  兼容。严格 JSON (`to_json_text`) 行为不变，仍不写 `//`。
- **TOML 空行与双槽注释保真** — `NodeMeta` 新增
  `leading_comment: Option<Comment>` 与 `blank_before: bool`（均从结构
  化 `Hash` / `PartialEq` 中排除），让 section 头部或 AOT 元素同时携带
  上方的独立注释与 `]` 后的行尾注释，互不隐盖。`to_toml` 现在会
  重现源文中的空行分隔（`a = 1\n\nb = 2` 字节稳定往返）；文档开头
  第一对KV不写前导空行。手建 / YAML 来源的节点仍通过 writer 的
  fallback 读取保持兼容渲染。
- **JSON5 方言** — `pyrs_yaml_core::json::from_json5(text)` 与
  `from_json_with_options(text, JsonParseOptions)` 接受 JSON5 全部四
  个轴：尾逗号、单引号字符串、无引号标识符 key 以及行/块
  注释。每个轴可以单独开关；`STRICT`、`JSONC`、`JSON5` 常量作为
  预设提供。
- **JSONC/JSON5 绑定与 CLI** — `pyrs_yaml.from_jsonc(str)` 输出 YAML
  文本；`pyrs_yaml.load_jsonc(str)` 直接返回 Python dict / list。
  `pyq from-json` 新增 `--jsonc` 与 `--json5` 旗标，`tsconfig.json` 与
  `settings.json` 能直接进入 verb 流水线。
- **JSONC 注释保真** — `from_jsonc` 现在会把采到的 `// 行` 与
  `/* 块 */` 注释挂到 AST 的 `NodeMeta::comment`（独立部分在 key
  节点，行尾部分在 value 节点），与 PR #109 建立的 TOML 模型对齐。
  配套的 `to_jsonc_text(node)` 与 `to_jsonc_text_pretty(node, indent)`
  按原位置写回；块注释输出时均一为 `//`（AST 仅存主体文本）。严格
  writer `to_json_text` / `to_json_text_pretty` 保持字节一致，使消费者
  可以选择性地启用保真。
- **pyq 多文档编辑** — `-A/--all-docs` 现覆盖全部编辑命令
  （set/delete/rename/move/append/insert/sort-keys）及 `to-json -A`
  （JSON 数组，与 Python 对等）。每个文档针对流中自己的文本段做 splice
  （`MultiDocEditor` + `DirtyUnit::shifted`）：未触碰的文档与所有 `---`
  分隔行逐字节保持原样；路径未命中的文档跳过（与 Python try/skip 语义
  一致，全部未命中仍报错退出）；布局异常的文档单独回退，不连带邻居。
- **JSONC 解析** — `pyrs_yaml_core::json::from_jsonc(text)` 与
  `from_json_with_options(text, JsonParseOptions)` 接受任意空白位置的
  `// 行` 与 `/* 块 */` 注释（即 TypeScript `tsconfig.json` 与 VS Code
  `settings.json` 使用的方言）。注释仅剥离，不保留。尾逗号及其他 JSON5
  独有形式仍拒绝，接受语言保持为 RFC 8259 的严格超集。`from_json` 默认
  行为不变（仍为严格模式）。
- **TOML 注释保真** — 解析器现在会采集独立注释（`# ...` 单独一行，
  位于键值对或 section 头部之上）与行尾注释（`key = value # ...` /
  `[name] # ...`），并通过 `NodeMeta::comment` 挂载到共享 AST（独立部
  分挂在 key 节点，行尾部分挂在 value 节点）。`to_toml(from_toml(src))`
  按原位置重新写回，`pyq edit` 与 `YamlDocument.set()` 不再剥离 TOML
  往返中的注释。空白行分隔仍采用 writer 默认样式（见设计文档）。
- **TOML 数字源码保真** — `to_toml(from_toml(src))` 现在保留十六进制
  (`0xDEADBEEF`) 与八进制 (`0o755`) 整数的源拼写，以及带指数的浮点
  (`1e10`、`-3.14e-2`)。下划线分隔符、显式 `+` 号、带负号的 radix
  形式 (`-0x1F`) 与二进制 (`0b101`) 仍归一为十进制，因为 YAML Core
  schema 无法重新读取它们，从而保证共享 AST 与 YAML 流水线的互操作
  性。注释保真与 JSONC 支持按设计文档后续 PR 落地。
- **pyq 功能补齐** — CLI 追平 Python CLI 功能面：`rename`/`move`/`append`/`insert`
  splice 编辑、`validate`（解析检查，或用 `--schema rules.yaml` 按 schema 语言规则
  校验）、`frontmatter`（`--body-out` 分离正文），以及 `get`/`fmt`/`to-json` 的
  `-A/--all-docs` 多文档流。接线过程揪出核心引擎 bug：`move_path` 只返回目标
  INSERT 单元，splice 文本会残留被移动子树的副本（文档回退全重序列化时不可见）；
  现返回两个单元，bindings 经批量 splice 通道依次应用。
- **`pyq` 过滤动词** — 匹配流上的结构化 jq 风格后处理：
  `--select 'PATH OP LITERAL'`、`--sort-by PATH` / `--desc`、`--unique`、
  `--first` / `--last`、`--skip N` / `--take N`、`--join SEP`，在 `get` 与
  `from-*` 上按固定管线 `select -> sort -> unique -> slice` 后接 `join`
  作用。有意选择旗标而非表达式语言：谓词仅一次微语法解析（约 40 行），
  类型不匹配一律 `false`（与 jq 全序的已知差异，已入文档），启动保持瞬时。
- **`pyq completion`** — 输出 bash、zsh、fish、PowerShell 的 shell 补全
  脚本（`pyq completion bash > ...`），由 `clap_complete` 驱动（已批准
  添加到 CLI crate 的依赖；仅存在于 `pyrs-yaml-cli` 二进制内，不影响
  Python 分发）。
- **`pyq sort-keys`** — 对任意路径（`$` 为根）的映射键排序，可原地回写
  或输出到 stdout，与 `set`/`delete` 共用核心 plan/splice 引擎，补齐与
  Python CLI `sort-keys` 的对等性。
- **命令行工具** — 新增 `pyrs-yaml` 命令（通过 `pip install "pyrs-yaml[cli]"` 安装，
  需 Python 3.10+），在终端中暴露库的核心能力：`fmt`（往返重新格式化，保留注释/
  锚点/顺序）、`get`（JSONPath 查询，支持 `--format yaml|json|text`）、`set` /
  `delete` / `rename`（基于路径的编辑，支持 `--inplace`、`--string`、
  `--create-missing`）、`validate`（对 CI 友好的退出码）以及 `to-json` / `from-json` 转换。所有命令通过 `-` 读取 stdin，默认输出到
  stdout。实现为纯 Python（`python/pyrs_yaml/cli/`），基于
  [Cyclopts](https://github.com/BrianPugh/cyclopts) 作为可选 extra，基础安装保持零额外依赖并继续支持 Python 3.8。
  基础安装保持零额外依赖并继续支持 Python 3.8。
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
  （`python/pyrs_yaml/settings.py`）是 `pydantic_settings.YamlConfigSettingsSource`
  的即插即用替代，使用 pyrs-yaml（YAML 1.2 核心 schema）而非 PyYAML 解析。采用惰性
  导出，`import pyrs_yaml` 不依赖 pydantic-settings；通过
  `pip install "pyrs-yaml[settings]"` 安装（Python 3.10+）。`dump_pydantic` 与
  `parse_as` 也已改为相同的模块级 `__getattr__` 惰性导出模式。
- **`pyq` — Rust 原生 CLI crate** — `crates/pyrs-yaml-cli`（workspace 成员，
  基于 clap）将 `pyrs-yaml-core` 直接接入 jq/yq 风格命令行，运行时无需
  Python：`fmt`（保留注释的往返格式化）、`get <path>`（JSONPath-lite，支持
  `--json`/`--raw`）、`set <path> <value>` 与 `delete <path>`（yq 风格编辑，
  支持 `--create-missing` 与 `-i/--inplace` 文件回写，输出经往返序列化器，
  保留注释与值自身风格）、`to-json`（保序）、`to-toml`，以及导入命令 `from-json` /
  `from-toml` / `from-ini`；输入格式按扩展名识别（`--input` 可覆盖），`-` 或
  省略时读 stdin，失败时以 core 的稳定错误文本非零退出。
- **TOML 与 INI 交换格式** — 轮毂-辐射式多格式支持，YAML 仍是唯一可编辑
  表示：`from_toml`/`to_toml` 在 TOML 文本 ⇄ YAML 文本间转换（Rust
  `toml_edit`）；`load_toml` 将 TOML 直接读为 Python 值（datetime 经内建
  `!timestamp` 插件；TOML 字符串不会被重新解析）；`load_ini` 经标准库
  configparser 读取 INI（严格模式，只读）。TOML 输出对不可表达结构报
  稳定错误；往返编辑按设计仅 YAML 支持。

#### 变更

- **granit-parser 1.1 → 1.3** — 将 YAML 事件解析器从 1.1.0 升级到 1.3.0。
  这是 1.x 版本线内语义化版本兼容的次级升级：1.2.0 为特殊文档的限制新增了可选的
  `Options` 字段，1.2.1 按 YAML 规范收紧了若干解析结果，1.3.0 为 `Input` trait
  新增了两个带默认实现的方法（`fetch_block_scalar_line` 与
  `take_quoted_scalar_ascii_chunk`），让扫描器更快地跳过块级与引号标量字节。
  本项目通过 `Parser::new_from_str` 使用解析器，只实现 `EventReceiver` /
  `SpannedEventReceiver`，从不实现 `Input`，因此无需改动源码——新增的 trait
  方法解析到其默认实现。全套测试通过：`cargo nextest run --all`（359）、
  `pytest`（1436 + 43 numpy）、纯 Rust `--no-default-features` 构建，且 YAML
  测试套件合规门控保持不变。
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
- **`YamlDocument.validate()` 缓存编译后的 validator** — schema（JSON 文本或
  dict）首次校验成功后即缓存编译好的 `jsonschema` validator；后续调用跳过 schema
  解析、meta-schema 检查与 validator 构建。dict schema 按对象身份键控并辅以深拷贝
  快照守卫：原地修改会在下次使用时通过 `==` 检出并透明重编译。缓存路径抛出
  `exceptions.best_match(validator.iter_errors(instance))`，与
  `jsonschema.validate()` 语义完全一致。WSL 实测：`document_validate` −98%。
- **解析/序列化内核结构化去重** — mapping 与 sequence 渲染共享单一
  `write_container_node` 骨架（输出字节级一致，`serialize_*` 中位数 −5~11%）；
  单/多文档解析入口共享同一 `load_ast` 错误契约；schema 解析链共享
  `bool_word`/`numeric_tail`，YAML 1.1 不再逐标量重复 core 的 null/bool 检查；
  锚点注册（`register_anchor`）与独立/行内注释分类（`is_standalone_placement`）
  在 AST 与流 receiver 间单源化。仓库重复代码率 3.38% → 2.60%。

#### 修复

- **`\u` / `\x` 转义后紧跟多字节字符时解析器 panic** — 定宽转义读取器
  按字节偏移切 `&self.text[pos..pos+width]`；JSON `\u` 或 TOML
  `\xHH`/`\uXXXX`/`\UXXXX` 后跟多字节字符时切片落在字符中间而 abort（#153
  非 ASCII 切片崩溃的同族）。现改为字节切片 + UTF-8 校验，畸形转义干净报错。
  由方言 fuzz 发现，两处解析器均有确定性 Rust 回归测试固定。
- **字面 `<<` 键（非 merge 值）被静默丢弃** —
  `load(safe_dump({"<<": None}))` 返回 `{}` 丢了键。merge 解析器把任意 `<<`
  都当 merge 消费，即使值是 Null/标量。按 YAML，`<<` 仅当值为映射别名/内联映射/
  其序列时才是 merge；Null/纯标量 `<<`，以及不含别名且无内容可合并的 `<<`（`<<: []`、`<<: [1, 2]`、
`<<: {}`）现保留为普通键并往返保真。Alias/映射/序列
  路径（含 #166 自引用守卫）不变，yaml-test-suite 仍 405/406。由往返属性 fuzz
  非确定性地暴露（正是 #163/#165/#166 类缺陷），并新增确定性 Rust 回归测试锁定。
- **TOML 深嵌套耗尽原生栈并 abort 进程** — TOML 解析器此前无嵌套预算
  （JSON 有 `DEFAULT_MAX_DEPTH`、YAML 有 `parse` `max_depth`），
  `parse_value` → `parse_array`/`parse_inline_table` 无界递归。深嵌套数组/内联表
  直接崩掉解释器（已验证：退出码 `0xC00000FD` STACK_OVERFLOW）——即
  TOML 版的 #166 YAML merge 栈溢出。解析器现追踪 `depth`，超过 1000
  返回类型化 `ParseError::MaxDepthExceeded`，与 JSON 对称。由进程内 Python
  边界测试 + subprocess 崩溃金丝雀 + 大栈 Rust 单元测试共同守护。
- **方言 writer/parser 丢失或错置文档级注释** — 三个定点属性抓到的 defect：
  (a) JSONC/JSON5 值前的文件首 `// note` 被误判为行内注释（空白扫描启发式中
  偏移 0 前无换行）并被首个对象成员占取，而非落在 writer
  `emit_root_leading` 所标注的根容器上——空 `{}` 或根标量时彻底丢失；(b)
  JSON 家族与 TOML writer 逐字输出注释体而 parser 存的是 trim 后的文本，未 trim 的
  注释会在多轮 pass 间振荡尾随空白——writer 现在输出时也 trim，首次拼写即
  稳定；(c) 纯注释 TOML 文档（`# note` 后无 key 消费）重解析时丢注释、空根
  序列化为 `""`——遗留的独立注释现在挂到空根表上。至此五格式的 leading
  注释均达到逐位稳定的定点。由
  `jsonc_file_leading_comment_stays_on_the_root`、
  `jsonc_comment_text_is_written_trimmed`、
  `comment_only_document_keeps_its_note_on_the_root` 钉住。
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
- **TOML 拒绝合法的最小 i64 整数** — `from_toml`/`load_toml` 在
  `-9223372036854775808`（`i64::MIN`）上失败：带符号路径先按无符号绝对值解析，
  取负号前就溢出。现在符号与数字一并解析（`i64::from_str` 向负方向累加），带符号
  浮点保留指数拼写，旧的取负通道已删除。由新的 Python 侧 Hypothesis 方言模糊测试
  （`tests/test_property_dialects.py`，以 stdlib `json`/`tomllib`/`pyjson5` 为预言机
  做类型严格相等比较；同时钉住两类 AST 歧义拼写——JSON5 裸 `Infinity`/`NaN` 字面量
  与超 i64 数字串）发现。Rust 回归：
  `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`。
- **错误缩进的流序列续行再次被拒绝** — 将 YAML 解析器升级到 granit-parser 1.3
  （见*变更*）后，开始静默地*接受*这样的输入：多行流集合的续行缩进不比其所在块键
  更深（yaml-test-suite `9C9N`：`flow: [a,` 后接列 0 的 `b,`），使严格性从
  `405/406` 回退到 `404/406`——由于 suite 的 ≥95% 阈值门，它对 CI 不可见，因而
  带着“绿”混过。现在 AST receiver 里一个解析后的 in-tree 守卫会跟踪所在块的缩进，
  并拒绝缩进不足的流续行，恢复 `405/406`。守卫只用解析器已算好的 span，因此正确
  缩进的多行流不受影响。`9C9N` 现被固化为逐例硬门（字面输入、无 `skipif`）写进
  `tests/test_yaml_suite.py`，另加一个 Rust 单测
  （`parser::tests::flow_continuation_under_indented_is_rejected`）。
- **自引用合并键不再溢出原生栈** — 展开后指回自身锚点（`a: &a` 内含
  `b: {<<: *a}`）的 `<<` 会在 `resolve_merge_keys` 中无限展开，耗尽原生栈并
  拖垮整个解释器进程（Windows 退出码 `0xC00000FD`，即段错误）。循环 guard
  此前只在*收集*合并对时生效，从不在*遍历*展开结果时生效，因此递归重入
  从未被拦截。现在锚点 guard 与别名展开一样按路径作用域：某锚点名在其展开被
  遍历期间始终留在递归路径上，解析回该路径上已存在的祖先的合并会终止为空展开
  而非递归。无环 AST 无法承载 PyYAML 的循环 dict，因此自引用合并现在收敛到
  `{}` 而不再崩溃。同批修复 4 个相关合并语义缺陷：null/标量/序列合并源不再
  残留为字面 `<<` 键；直接作为合并值的内联映射（`<<: {x: 1}`）现在会被合并；
  合并序列中的非别名元素（`<<: [*a, {y: 2}]`）会保留其内联映射。由 6 个 Rust
  与 9 个 Python 回归测试覆盖（`merge::tests`、
  `tests/test_gaps.py::TestSelfReferentialMerge166`）。
  由 [@bourumir-wyngs](https://github.com/bourumir-wyngs) 在 #166 报告。
- **NumPy 序列化不再在不持有 GIL 时读取 Python 内存** — ndarray 写入器曾通过
  `unsafe { as_slice() }` 借用数组数据缓冲区，并在 `py.detach` **内部**（即已释放
  GIL 之后）遍历该借用切片。无论来源为何，`&[T]` 都是 `Send`，因此借用检查器无法
  拦截；但这块内存归 Python 所有，其他线程可并发 resize 或写入，构成不健全的数据
  竞态 / UB，仅在并发下暴露。现改为在**仍持有 GIL** 时把缓冲区快照为 Rust 自有内存
  （`slice.to_vec()`），仅标量→节点转换离线程执行。这是绑定层**唯一**一处
  `unsafe` 缓冲区借用；其余全部 `py.detach` 站点已审查，仅触及 Rust 自有状态
  （AST、源文本、`BufWriter<File>`）。回归覆盖见
  `tests/test_numpy.py::TestNumpyConcurrency`。由
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) 在 #165 中报告。
- **重复的别名引用不再解析为 `None`** — `to_dict()` 在一个**全局** visited 锚点集合
  后展开别名，且从不清理，导致任一锚点只有**第一次**引用产出值，之后全部静默降级
  为 `None`：

    ```yaml
    a: &x 1
    b: *x      # 1
    c: *x      # 原为 None，现为 1
    ```

    影响面比“第二次引用”更宽：同一容器内的两个兄弟引用也会互相污染
    （`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}` 中 `b` 有值而 `c` 为 `None`）。现将该
    guard 限定在当前递归路径上——仅在一次展开期间压入，随后弹出——因此重复引用与
    兄弟引用各自获得完整构建的值，而真正的环路仍会终止。`<<` 合并解析与 AST 本身经
    审查不受影响。`tests/test_direct_load.py` 新增 6 个与 PyYAML 对齐的用例锁定该
    行为，另有两个此前将错误输出当作预期的测试被重写。由
    [@bourumir-wyngs](https://github.com/bourumir-wyngs) 在 #163 中报告。
- **首层值为嵌套容器时文档头注释不再丢失** — 解析器曾为所有进行中的
  容器共用单一注释槽，嵌套容器的 start 会提前抹掉尚未落位的 standalone
  头注释（在 parse 层即丢弃；`to_dict` 不可见，`dump` 致命）。现改为
  每容器独立槽的栈式管理。
- **splice 编辑不再重复前导注释** — 重生成的区域文本携带 pair/item 自身
  的 standalone 注释时，被替换区域未覆盖旧注释行，两条注释并存；plan
  现将区域回扩至注释行（`pyq set`/`delete` 与 bindings splice 路径共享
  此修复）。
- **`!timestamp` 在所有受支持 Python 上接受结尾 `Z`** —
  `datetime.fromisoformat` 仅从 3.11 起识别 UTC-`Z` 后缀；插件现将
  `...Z` 归一为 `+00:00`，修复 3.8–3.10 上 YAML `!timestamp` 标量及
  `load_toml` / `from_toml` 引入的 TOML datetime 报 `Invalid isoformat
  string` 的问题。

#### 性能

- **事件流→Python 对象的直接物化** — `safe_load`、`safe_loads`、
  `YAML().safe_load*` 现在单次遍历 granit 事件流直接构建 Python 对象，
  不再先建完整 AST 再在 `convert.rs` 中二次遍历；schema 解析、原文映射
  键与重复键报错语义完全一致。带锚点/tag/merge/多文档的输入经零成本
  预否决回退 AST 管线。WSL 实测：标量密集 `safe_load` −21~25%，
  家族整体 −13~18%，回退形态不变。
- **锚点提取字节门控** — `extract_anchors` 先做一次 `&` 字节包含检查，无锚点文档
  直接返回空，整体跳过逐字符引号状态机。Rust 侧 `parse_*` 基准中位数提升 11–18%，
  扫描本身从 1.5µs 降至 38ns。
- **流事件字典键驻留** — `parse_stream`/`load_stream` 每事件的固定键复用
  `pyo3::intern!` 常驻字符串，消除每键一次的 Python 字符串分配。WSL 实测：
  `parse_stream` −34%、`parse_stream_multidoc` −39%、`load_stream` −22%。
- **分解微基准** — 新增 `granit_events_*` 基准，将 granit 纯事件管道成本与 AST
  构建分离（仅基准）。
- **多文档解析免除逐文档深拷贝** — `on_document_end` 改为将完成文档的所有权移动
  进集合而非深拷贝（下一文档会重建 result，克隆是纯开销）。WSL 实测：
  `parse_all_docs` −9.7%、`safe_loads`（多文档）−9.5%、`YAML().safe_loads` −6.7%。
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

### [v0.15.0] — 2026-08-19

#### 新增

- **Node 元数据 setter/getter** — 新增 `Node.comment` / `Node.anchor` / `Node.tag` 只读属性和 `set_comment` / `set_anchor` / `set_tag`（及 `remove_*` 系列）。编辑别名或不存在路径会报错；内联标量值和序列项上的独立注释现在输出到独立的缩进行（修复 `child:\n  # c\n  val` 与 `- a\n# c\n- b` 既有的 round-trip 缺陷）。
- **Verbatim 标签** — `set_tag("!<tag:yaml.org,2002:str>")` 现在生成 verbatim 标签（空 handle），且从源码解析的 verbatim 标签在 round-trip 中保留：`Tag` 的 `Display` 对空 handle 标签以 `!<...>` 包裹输出，`parse_tag` 识别 `!<...>` 形式，流事件通过 `Display` 序列化标签。
- **Schema 文件 IO 与列表** — `load_schema(name, path)` 从文件读取 schema 定义并注册，`list_schemas()` 返回所有已注册的 schema 名称（内置 `failsafe`/`json`/`core`/`yaml1.1` + 自定义）。
- **Node style/format setter/getter** — 新增 `Node.scalar_style` / `Node.flow_style` / `Node.chomping` 只读属性和 `set_scalar_style` / `set_flow_style` / `set_chomping` 方法。ScalarStyle/Chomping 现在 derive `Copy`。非标量节点返回 `None` / no-op，别名和缺失路径报错。
- **Schema 结构化校验** — 在 schema 定义的 `validate` 段添加结构检查（路径限定标量类型、`sequence_of`/`mapping_of` 容器、`required`）。`validate_against_schema(data, schema_yaml)` 列出所有失败项并抛 `YamlValidateError`。
- **`Node.copy()`** — 将子树深度复制为与文档分离的独立 Python 值（dict/list/scalar），可用于通过 `set_value()` 粘贴。
- **深度编辑 API** — `doc.set_many({path: value})` 在单次 splice 突发中设置多个路径（支持通配符 `[*]` 和深度扫描 `..`）；`doc.sort_keys()` 原地排序映射键；`Node.move(new_path)` 移动子树；`Node.path` / `Node.find_first()` / `Node.value_eq()` 新增路径访问、首通配符查找、值比较。
- **0.14+ 新功能的属性测试** — `validate_node` / schema 解析 / style round-trip 的 Rust proptest，`set_many` 通配符 / metadata 编辑 / `sort_keys` 的 Python hypothesis 测试。`hypothesis` 移至 `test` 组，确保 CI 运行属性测试。
- **序列化器修复** — 空 flow 容器（`key: {}` / `key: []`）上的独立注释不再产生无效 YAML（降级为行内）。

#### 变更

- **NumPy 在自由线程 (cp314t) wheel 上重新启用** — 从 cp314t 构建参数移除 `--no-default-features`；rust-numpy 0.29 支持自由线程 Python，`numpy.ndarray` 序列化现可在自由线程 wheel 上使用（运行时自动检测 NumPy 是否安装）。

#### 文档

- **修正全部语言（en/zh/ja/ko）文档中的过时引用** — `saphyr-parser` → `granit-parser`，YAML 合规率 98.1% → 99.75%（405/406 套件用例），ABI3 支持 3.9–3.13 → 3.8–3.15（py3.9+ → py3.8+），并更新基准测试表为当前 CodSpeed CI 数据（解析快 21–43 倍、序列化快 55–177 倍于 PyYAML）。Rust 侧基准测试章节从 Criterion 迁移到 divan（`benches/yaml_bench.rs` → `crates/pyrs-yaml/benches/yaml_bench.rs`）。

### [v0.14.1] — 2026-08-15

#### 修复

- **含反斜杠+控制字符/非字符的单引号标量** — 此类值改用双引号输出；单引号无法转义控制字符/非字符。
- **非字符与 BOM 引用** — `needs_quotes` / `needs_double_quoted` 现对 U+FFFE/U+FFFF/平面末尾非字符及 U+FEFF（BOM）要求引用。
- **双引号转义宽度** — U+FFFF 以上的码点现以 8 位 `\Uxxxxxxxx` 形式转义（4 位 `\u` 仅限 BMP）。
- **折叠 plain 标量续行缩进** — 续行缩进改由值起始列推导，使嵌套序列/映射项续行缩进超过父块缩进。
- **多字节折叠边界** — `wrap_plain_scalar` 对折叠切片做 char boundary 向下取整，避免 4 字节 UTF-8 跨边界时 panic。
- **publish 测试依赖含 `hypothesis`** — `.ci/requirements-test.txt` 固定 `hypothesis>=6.113.0`，使发布工作流能运行属性测试。

#### Added

- **`scripts/fuzz_panics.py`** — 本地大规模 Hypothesis fuzz 脚本，含恶意策略覆盖 dump/parse/edit/幂等。

### [v0.14.0] — 2026-08-14

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

- **带引号标量恒为字符串** — 隐式类型解析仅作用于纯标量（YAML 1.2）。`safe_load('"true"')` 返回字符串 `"true"`（而非 `True`）。序列化器保持文档（`to_yaml`）路径下的负数正确往返。
- **单引号/双引号单字符键可往返** — 值为单个 `'` 或 `"` 的映射键以引号标量输出，不再产生无法解析的 YAML。
- **空集合输出 `{}`/`[]`** — 空映射/序列序列化后不再是解析为 `None` 的空文档。

#### 变更

- **`get()` 仅接受字面键** — `YamlDocument.get()` 不再将含 `.`/`[` 的键视为 JSONPath；所有键都按顶层映射键处理（与 `__getitem__`/`__setitem__` 一致）。路径访问请使用 `find()`/`node()`。

### [v0.13.0] — 2026-08-10

#### 变更

- **Rust MSRV 提升至 1.96，edition 升级为 2024** — 两个 crate 均声明
  `rust-version = "1.96"` 和 `edition = "2024"`；CI 将 `build`/`test-freethreaded`
  任务固定在 Rust 1.96 以生成确定性 wheel；新增 `msrv-check` 任务在 MSRV
  上运行 `cargo check`/`cargo test` 防止静默漂移（`rust-lint` 仍使用 `stable`）。
  版本基线高于 PyO3 0.29 自身的基线（rustc 1.83），目的是获得 std API 的前瞻性
  支持（如 `assert_matches!`，1.96 稳定），无需代码迁移。
  `TAG_REGISTRY`（标签处理器存储）重构为 `std::sync::LazyLock`，
  移除了 `Mutex<Option<...>>` 间接层。

#### Performance

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer**
  — Python→YAML 序列化无需中间 `CustomNode` AST。
  单次 `direct_dump` 替换旧的两次传递 `pyobject_to_node` + `to_yaml`。
  `safe_dump` 提速 7 倍（28ns→4ns），`from_dict` 提速 6 倍（35ns→6ns）。(#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking**
  — 当输入不含 `&` 字符时，跳过 `collect_anchors` 和锚点解析，
  使用更简单的 `node_to_pyobject_simple` 路径。(#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — 非数字/
  非布尔首字节立即返回 `Str`，避免常见情况下的 schema 解析开销。(#59)
- **迁移到 granit-parser** — 用 granit-parser 1.0.1 替换 saphyr-parser，
  借助原生 `Event::Comment` 输出消除了全文 `scan_yaml()` 预扫描。
  parse_small -18%、parse_large -21%、roundtrip_large -18%。

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
- **只由换行组成的块标量值不再退化成空串** — `>+8\r\r#` 读作一个值为单个换行、chomping
  为 `Keep`、显式缩进为 8 的折叠标量。写手保留了缩进指示符，发射出 `>+8\n\n`；它重读时值
  不变但变成 `Clip` 且无指示符，于是下一轮发射 `>\n\n`，值就成了 `""`——Clip 会剥掉尾部换行，
  而该值仅有的那一个换行无处安放。让*空* body 保持幂等的那两条规则（丢弃不可恢复的缩进指示符；
  写出能重读回同一值的 chomping）都以“空”为条件，而“全是换行”的 body 并不为空（libFuzzer
  `yaml_roundtrip`，`crash-2f6b1eff`，6 字节——已是最小：输入不再崩溃，`tmin` 无从缩小）。两个
  写手现在都以“没有内容行”为共同条件，形状一轮即达不动点，值也保住了。代价：首版对同一个值多扫了一遍，`serialize_block_scalars` 实测 +0.65%——在 runner 上是
  +2.11%，越过指令数门禁的 2% 容差，于是门禁在合并前拦下了这次改动。改为复用写手已经算出的
  首个内容行判据后降到 +0.15%，回到容差内。
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
- **Pydantic 集成** — `dump_pydantic()` 将 Pydantic 模型序列化为 YAML
  字符串（`model_dump(mode='json')` + `safe_dump`）；`parse_as()` 将
  YAML 字符串解析为 Pydantic 模型实例。两者均使用延迟导入，无硬性
  pydantic 依赖。(#61)

#### Internal

- **拆分 `py/mod.rs`** — 单体 1786 行模块拆分为
  `document.rs`（YamlDocument）、`yaml_instance.rs`（YAML 类）、
  `functions.rs`（模块级函数）、`stream_iterator.rs`、
  `walk_helpers.rs`。`mod.rs` 缩减至 128 行。(#61)
- **`needs_quotes()` 守卫 + `double_quoted_scalar()` 构造器** —
  `'true'` / `'42'` / `'null'` 等字符串现以双引号标量输出，避免 core schema
  重新解析时被误读（`pyobject_to_node` + `json_value_to_node`）。
- **CodSpeed 基准统一到 `codspeed-divan-compat`** — `exclude-allocations`
  去除分配器噪声；跨库基准合并到 `tests/test_benchmark_crosslib.py`，
  引入共享 `tests/data/yaml_samples.py` 夹具和流式覆盖。

### [v0.12.1] — 2026-08-06

#### Added

- **`set(create_missing=True)`** - 编辑路径上缺失的中间映射键会创建为嵌套映射
  （例如，对 `a: 1` 设置 `a.b.c` 会创建 `b` 和 `c`）；索引段缺失仍报错，
  路径上的标量中间层仍会引发异常。
- **`doc.walk()` / `doc.scalars()`** - Rust 后端的深度优先 AST 遍历，
  返回 `Node` 对象，避免逐节点 `to_dict()` 解析。
  `walk()` 返回所有节点；`scalars()` 仅返回标量/null 节点。
- **Rust 核心模块测试** - 39 个新测试，覆盖 `editing::navigate`
  （key_eq、navigate、navigate_mut、normalize_index、mapping_key_index）、
  `editing::region`（行辅助函数、node_is_flow、extend_delete_over_comments、
  nav_err）、`editing::dirty`（DirtyKind/DirtyUnit 构造函数）以及
  `editing::metadata`（with_metadata_from、needs_quoting）。
- **Python doc.walk() 边界测试** - 9 个新测试，覆盖空文档、空值、
  深度嵌套、流集合、混合类型。

#### Changed

- **Monorepo workspace** - 源码拆分为 `crates/pyrs-yaml-core/`
  （纯 Rust，无 PyO3）和 `crates/pyrs-yaml/`（PyO3 绑定）。根
  `Cargo.toml` 现在是 workspace。旧的 `src/` 目录和 `build.rs`
  已移除。
- **pyproject.toml** - 新增 `tool.maturin.manifest-path` 指向
  `crates/pyrs-yaml/Cargo.toml`。
- **解析热路径** - 单次注释/锚点提取、延迟重复键检测、`shift_insert`
  合并预处理，以及单文档解析跳过 `DocumentEnd` 深拷贝，大文档
  解析成本降低约 19%（CodSpeed: parse[large] +13.9%，parse[medium] +16.6%，
  roundtrip[large] +12.2%）。
- **`Arc<str>` 标量存储** - `CustomNode::Scalar` 和注释/事件
  文本通过 `Arc<str>` 共享分配；AST 节点缩减 8 字节，
  克隆变为引用计数递增而非深拷贝。

#### Fixed

- **`set(create_missing=True)` 嵌套链构建** - 创建的映射链
  不再将第一段重复为嵌套键层级。
- **`set(create_missing=True)` 资格检查** - 新创建的键现在
  可参与值写入（资格检查不再在合成对插入后运行）。
- **简单映射键前的独立注释** - 往返之前会丢弃附加到
  简单键节点的独立注释；现在保留（两个回归测试）。

### [0.11.7] - 2026-08-04

#### Changed

- **stub-build-check 替换为 release-guard** — 故意失败以复现
  v0.10.0 `--generate-stubs` 失败模式的总是失败的容器构建
  （`validate.yml`）被三个静态断言替换，当仓库正确时**通过**：
  `grep` 保护 `publish.yml` 不含 `--generate-stubs`，
  `git ls-files` 断言提交的 `.pyi` 已追踪，
  `test -f` 检查 `py.typed` 存在。任务现在在正确状态下给出绿色 CI，
  仅在回归时红色。

#### Added

- **Numpy free-threaded 跟踪** — ROADMAP.md 现在跟踪 `rust-numpy`
  free-threaded 支持状态（PyO3/rust-numpy#476），作为 Rust
  绑定成熟后在 cp314t wheel 上重新启用 ndarray 序列化的依赖。

### [0.11.6] - 2026-08-04

#### Changed

- **Free-threaded（cp314t）wheel 不再包含 numpy** — 使用
  `--no-default-features` 构建，rust-numpy 完全排除（更小的
  二进制，无运行时探测）。free-threaded 构建上对 `numpy.ndarray`
  调用 `safe_dump` 会引发 `YamlTypeError`；GIL 构建（Python 3.8-3.15）
  保留完整的 ndarray 序列化。

#### Added

- **Free-threaded CI 验证** — `test-freethreaded` 任务现在使用
  `--no-default-features` 构建和测试，与分发的 free-threaded
  wheel 配置匹配。
- **安装文档** — `docs/{en,zh,ja,ko}` 注明 free-threaded
  wheel 不含 numpy（cp314t 上不可用 ndarray 序列化）。

### [0.11.5] - 2026-08-04

#### Changed

- **解析器健壮性项目 3/4/5 通过 Phase 0 严格性审计关闭** —
  70 探针语料库（缩进、块映射键、流上下文）与 PyYAML 预言机对比，
  显示**无可修复的接受但无效案例**（64/70 匹配；6 处分歧是
  有意为之的 YAML 1.2 / yaml-test-suite 要求，PyYAML 是异常项，
  另有一个有意为之的重复键严格性）。合规率保持在
  **99.75%（405/406）**。完整说明见 `ROADMAP.md` §v0.11.5
  和 `tests/test_strictness_audit.py`。

#### Added

- `tests/test_strictness_audit.py` — 70 探针严格性回归语料库，
  固定当前拒绝/接受行为（两个方向），使未来解析器变更无法
  静默降低严格性或过度拒绝。

### [0.11.4] - 2026-08-04

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

### [0.11.3] - 2026-08-03

#### Added

- 流式写入：`YAML.dump_stream(file_obj, iterable)` /
  `YAML.dump_file(path, iterable)`，文档级恒定内存，自动 `---`
  分隔符，以及 `explicit_start`/`explicit_end` 标志
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

### [0.11.2] - 2026-08-03

#### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`：
  O(锚点数 + 块) 内存的惰性事件迭代器

#### Performance

- **解析不再计算拼接资格** — O(文档) 布局检查现在在首次编辑时通过
  `YamlDocument.splice_checked` 惰性运行，恢复 v0.11.0 回归：
  parse_comments -59%、parse_anchors -42%、parse/roundtrip/edit -10~35%
  全部回到 v0.10.0 水平
- **线性游标布局检查** — 取代基于预计算行偏移的逐节点二分查找
  （单调源码顺序遍历）

#### Changed

- `parse_with_options` 返回 `CustomNode`（原为 `(CustomNode, bool)`）；
  拼接资格现在内置于 `YamlDocument` 并按需计算

### [0.11.0] - 2026-08-02

#### Added

- **精准序列化** — 字节级源码范围追踪；基于段的拼接 — 编辑仅重新生成
  触碰区域，未触碰文本按字节复制
- proptest 保真度属性测试（新开发依赖）
- 10MB 编辑-刷新基准测试（divan）

#### Changed

- `flush_source` 现在使用分段拼接；回退到全量序列化：流风格区域、
  非默认布局文档、合并键、CRLF/BOM 文档，以及 materialize 之后
  （单次爆发模型）
- 拼接编辑保留 `---`/`...`/指令标记行为未触碰字节
  （全量序列化之前会丢弃它们 — 设计上的行为差异）

### [0.10.0] - 2026-08-01

#### Added

- **就地编辑** — 编辑已解析的文档而不丢失格式元数据：
    - 路径 API：`doc.set(path, value)`、`doc.insert(path, index, value)`、
      `doc.append(path, value)`、`doc.delete(path)`、`doc.rename(path, new_key)`，
      使用 JSONPath 风格路径（`$.a.b[0]`）；根节点语法糖
      `doc["key"] = value` 和 `del doc["key"]`
    - 节点 API：`doc.node()` / `doc.find(path)` 返回 `Node` 对象，
      支持 `set_value` / `append` / `insert` / `delete` / `rename`，
      以及树遍历（`parent`、`children`、`walk`、`filter`）
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

### [0.9.0] - 2026-08-01

#### Added

- **Python 3.13、3.14 和 3.15 支持** — PyO3 `abi3-py38` wheel 覆盖
  Python 3.8-3.15（GIL 构建）；`abi3t` + `abi3t-py315` 提供
  free-threaded 稳定 ABI
- **Free-threaded CPython（无 GIL）支持** — `#[pymodule(gil_used = false)]`
  声明模块对 free-threaded Python 线程安全；`Py_GIL_DISABLED` cfg
  标志门控 numpy（rust-numpy 尚不支持 free-threaded — 通过
  `--no-default-features` 为 free-threaded 构建禁用 numpy feature）
- **CI free-threaded 任务** — 新增 `test-freethreaded` 工作流任务，
  针对 Python 3.14t 验证编译和测试
- **`pyo3-build-config` 构建依赖** — 通过 `build.rs` 启用
  `#[cfg(Py_GIL_DISABLED)]`、`#[cfg(Py_3_15)]` 等编译器标志
- **`numpy` 改为可选** — 由 `numpy` feature 门控（默认启用）；
  在 `Py_GIL_DISABLED` 下自动排除
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`、
  `parse(..., allow_duplicate_keys=True)`、`parse_file`、`safe_load`、
  `safe_loads`、`parse_all_docs` 均接受该标志；重复映射键默认
  引发 `YamlDuplicateKeyError`，允许时采用"最后值生效"
- **`SerializeOptions` 扩展** — `doc.to_yaml_with_options()` 新增
  `width`（行包裹，0 = 关闭）、`indent_mapping`、`indent_sequence`、
  `indent_offset`，与现有的 `indent_size`/`explicit_start`/
  `explicit_end`/`sort_keys`/`max_depth` 并列
  （`src/py/mod.rs:432`）
- **标签处理器注册表** — `register_tag("!custom")` 装饰器和命令式形式
    - `clear_tag_handlers()`；携带已注册标签的标量节点通过处理器转换
  （`src/py/tag_registry.rs`）
- **标签处理器优先级链** — 同一标签的多个处理器按升序 `priority`
  执行；`YamlTagSkip` 让处理器传递给下一个，fallback 保留原值
- **Pydantic 集成** — `parse_as(Model, yaml, **yaml_kwargs)` 解析
  YAML 并针对 Pydantic v2 模型验证；缺少 pydantic 时引发
  `ImportError` 并附指导信息（`python/pyrs_yaml/pydantic.py`）
- **`.pyi` 类型存根** — 由 maturin 自动生成并提交，使
  `register_tag`、`parse_as`、`to_yaml_with_options` 和新异常
  对类型检查器可见

#### Changed

- CI Python 矩阵扩展：ubuntu、windows、macos 上的 3.8-3.14
- 稳定 ABI：`abi3-py39` → `abi3-py38`（更广的 Python 3.8+ 支持），
  新增 `abi3t` + `abi3t-py315`（free-threaded 稳定 ABI）
- `pyproject.toml` classifiers 更新 3.13、3.14、3.15 条目
- **CI 优化：消除冗余 Rust 编译** — 单个 `rust-lint` 任务运行
  `cargo clippy` + `cargo test` 一次；`build` 任务为每个 OS
  生成一个 abi3 wheel，测试任务安装而非运行 `maturin develop`，
  将 Rust 编译从 21 个矩阵任务中移除（减少约 86% 编译量）；
  所有任务添加 `Swatinem/rust-cache`
- **pydantic 测试依赖** — `pydantic>=2.10.6` 加入
  `[dependency-groups] test` 和 `.ci/requirements-test.txt`
  （通过 `uv sync` 在 ci.yml 中统一管理）

#### Fixed

- **Windows DLL 加载** — 移除 `src/py/tag_registry.rs` 中的
  `#[cfg(test)]` 块，该块在 Windows 上破坏了 `import pyrs_yaml`
  （`250b8d0`）
- **Python 3.8 兼容性** — `pydantic.py` 中添加
  `from __future__ import annotations`（`63d2495`）
- **CI pydantic 跳过** — 使用 `pytest.importorskip("pydantic")`
  使测试在未安装 pydantic 时通过（`7be011d`）
- **CI Windows glob 展开** — `pip install dist/*.whl` 使用
  `shell: bash`（PowerShell 不展开 `*`）（`2f7778d`）
- **非字符串标签处理器返回值现在引发 `YamlTagError`** — 返回
  非 `str` 值的处理器（之前被静默忽略，保留原标量）现在报错
  `Tag handler '!x' must return a string`（`src/py/mod.rs:resolve_tags`）
- **`to_yaml_with_options` 缩进连线** — `indent_mapping`/
  `indent_sequence`/`indent_offset` 现在被序列化器尊重
  （之前为死字段）；省略时分别默认 `indent_size`/0
  （`src/serializer.rs`）
- **`width` 不再对极小值死循环** — `width < 续行缩进` 时回退为
  直接输出未包裹的剩余内容而非无限循环（`src/serializer.rs:write_plain_scalar`）
- **`remove_tag(name)`** — 新增函数用于注销标签处理器；
  补充 `register_tag`/`clear_tag_handlers`（`src/py/tag_registry.rs`）
- **`duplicate-key` 错误国际化** — `YamlDuplicateKeyError` 消息
  现通过 `format_i18n_error` 在所有 4 个语言区域中传递
  （`src/i18n/locales/*.yml`）

### [0.8.0] - 2026-07-30

#### Added

- **`YAML()` 实例 API** — `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)`，
  可复用配置；`.parse()`、`.safe_load()`、`.safe_loads()`、
  `.parse_file()`、`.parse_all_docs()` 方法
- **Python `Node` API** — `Node` 类，具有 `find()`、`filter()`、
  `walk()`、`to_yaml()`、`parent`、`children`、`root_type`、
  `value`，用于 AST 导航；JSONPath 风格查询语言
  （`$.key.sub`、`$.arr[0]`、`$..deep`）
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

### [0.7.1] - 2026-07-30

#### Added

- **ryaml 基准对比** — `tests/test_benchmark.py` 现在与
  `ryaml`（Rust YAML 库） alongside PyYAML 和 ruamel.yaml 进行
  基准测试；`benchmark_compare.py` 重写为特性对比报告
  （`tests/test_benchmark.py:25-28`、`.github/workflows/ci.yml:219`）
- **CI 合规阈值提升** — YAML Test Suite 合规阈值从 70% 提升至
  75%（`test_compliance_report()`）；有效解析率阈值 95%
  （`tests/test_yaml_suite.py:251`）
- **CI 依赖整合** — 新增 `.ci/requirements-test.txt` 和
  `.ci/requirements-test-lite.txt`，用于发布工作流和本地开发
  的统一测试依赖管理
- **基准测试现代化** — 从 `pytest-benchmark` 迁移至
  `pytest-codspeed` 以实现更快的 C 扩展统计基准测试；
  所有 CI 任务现在使用 `-r .ci/requirements-test.txt`
- **Rust 基准测试迁移至 Divan** — 用 `codspeed-divan-compat`
  v5.0.1 替换 `codspeed-criterion-compat`；16 个基准测试从
  Criterion 组重写为 `#[divan::bench]` 属性
  （`Cargo.toml`、`benches/yaml_bench.rs`）

#### Changed

- CI 基准任务安装 `ryaml` 用于跨库对比
- `benchmark_compare.py` 现在委托计时至 `pytest-benchmark`，
  作为特性对比/报告工具

### [0.7.0] - 2026-07-29

#### Added

- **序列化器 `max_depth` 守卫** — `serialize_node_internal` 现在
  跟踪递归深度，超出限制时引发 `YamlMaxDepthError`（默认 1000），
  与解析器保护一致（`src/serializer.rs:135-145`）
- **序列化器热路径优化** — 5 项针对块风格序列化的优化，约
  4.9% 往返加速：
    - 内联 `write_anchor_tag` 和 `write_inline_comment` None 检查
      （消除约 99% 节点的函数调用）
    - `write_indent` 热/冷路径分离（缓存级别 ≤64 直接索引）
    - `write_plain_scalar` 短 ASCII 字母数字字符串快速路径
      （≤8 字符）
    - `write_scalar_for_key` Plain 标量直接分派（避免分派链）
- **pytest-benchmark 迁移** — Python 基准测试从原始
  `time.perf_counter()` 迁移至 `pytest-benchmark` 以获得统计严谨性、
  结构化 JSON 输出和 CI 集成（`tests/test_benchmark.py` + 更新的
  `tests/test_performance.py`）

#### Changed

- `pytest-benchmark` 替换 Python 基准测试中的原始 `timeit`
- CI 基准任务现在运行 `pytest --benchmark-json` 而非独立脚本

#### Removed

- `write_inline_comment` 方法 — 在所有调用点内联
- 序列化器的 `Comment` 导入 — 不再需要

### [0.6.0] - 2026-07-27

#### Added

- **异步序列化** — `safe_dumps_async`、`safe_dump_async`、
  `safe_loads_async`、`safe_load_async`，通过 `asyncio.run_in_executor`
  （`python/pyrs_yaml/async_dump.py`）
- **JSON Schema 验证** — `YamlValidateError` 异常 +
  `YamlDocument.validate(schema)` 方法（接受 `str` 或 `dict`）；
  委托至 Python `jsonschema` 模块
- **`YamlDocument.to_json()`** — 将文档序列化为 JSON 字符串
  （使用 Python `json.dumps`）
- **增量重新解析** — `YamlDocument` 现在存储源文本
  （`doc.source()`）；`doc.reparse(resolve_merges=True, schema="core")`
  就地重新解析
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

### [0.5.0] - 2026-07-27

#### Fixed

- **`Serializer::write_node`** — `block_mapping`/`block_sequence`
  中 `.unwrap()` on `values.iter().next().unwrap()` 替换为安全
  索引访问，消除边缘 AST 的潜在 panic
- **`YAML_SCHEMA` 常量** — 拼写错误 `yamorg2002` 修正为
  `yamlorg2002`（匹配 YAML 1.2 规范 URL）
- **开发文档** — `AGENTS.md` 更新，为 Python 命令添加强制
  `uv run` 前缀，Rust 命令直接使用 `cargo`

### [0.4.0] - 2026-07-27

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
- **YAML Test Suite 单个用例测试** — 八进制、十六进制、科学计数法、
  NaN、无穷大、合并键、显式/隐式键、布尔/空变体、块标量截断
  （`|-`）、流集合
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

### [0.3.0] - 2026-07-27

#### Added

- **NumPy ndarray 序列化** — `safe_dump()` / `safe_dumps()` /
  `from_dict()` / `dump_file()` 现在支持所有维度的
  `numpy.ndarray`（0-D 至 N-D）
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

### [0.1.0] - 2026-07-25

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
