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

- **Python 生成层如今画出它的叶子从前叫不出名字的家族** — `arbitrary_json` 原先把叶子限制在 `None`、
  `bool`、±10^12 以内的 `int`、有限 `float` 和 `str`，而清点它生成的叶子正好说明这排除了什么：25,000
  次抽样产出 270,602 片叶子，这五类各 58,064 / 53,396 / 53,251 / 52,978 / 52,913 片，此外一片别的类
  型都没有。于是数字协议、类型注册表、`!!binary` 与 `!!timestamp` 的写路从未被抽到，这一层的断言其实
  是“在 JSON 这个子集上不 panic”。`tests/strategies.py` 里的 `hard_leaf` 加上无界整数、非有限浮点、
  元组、`bytes`、`date`、`time`、`datetime`、`Decimal`，以及四种没有 YAML 拼法的类型；放宽后的生成器
  抽 20,000 次，229,798 片叶子里有 134,145 片落在这些家族上，每族约 10,000 片，另加 22,108 个越过
  i64 的整数与 10,470 个非有限浮点。不变量随定义域一起放宽：`test_dump_never_raises` 只接受文本或
  `pyrs_yaml.YamlTypeError`，换个名字的拒绝就是失败，把 handler 换成 `except ValueError` 就会在一个
  `set` 抽样上变红；而 `test_each_widened_family_is_text_or_the_typed_refusal` 逐行钉住十七个实测结
  果——越界整数读回 `str`，元组读回 `list`，`bytes` 读回 `bytes`，时间类型与 `Decimal` 读回自身，
  `set`、`frozenset`、`complex` 或未注册的类得到那个类型化拒绝——每个到达文本的行还断言第二次写出同一
  串字节。其中一条是新问题而不是重述：一个家族若要到第二次写出才稳定，那它就是会移动的往返，而确实有
  一个会移动。`safe_dump(-(2**64))` 写出裸数字，中枢把它读成字符串，再写时加了引号；同一个决定作用在
  `2**63` 上却复述自己的文本，一处引号守卫在正负两个符号上不一致。这一行按实测的第二次写出结果记进
  `NON_SETTLING`，而不是从表里删掉，并且作为未决缺陷挂着，不由本条关闭。最后一条守的是策略本身：
  `test_the_widened_leaves_reach_every_family_they_were_added_for` 用同一个生成器抽 400 次，凡是没到
  的家族就点名报错，把 `hard_leaf` 拿掉它就会列出全部十二个——于是这一层不可能悄悄重新变窄。引擎代码
  一行未动：这些行点名的是绑定层本来的行为，是量出来的，不是推出来的。精确相等的策略仍旧保持 JSON 形
  状，因为它们的断言是逐字节往返，而那只有那个子集才成立。这些行还需要一次由放宽本身暴露的隔离：把
  `tests/test_concurrency.py` 排在前面——该模块以 `clear_type_handlers()` 收尾——于是四个家族直接拒
  绝，`Decimal` 写下不带标签的 `1.5`、读回成 `float`；那个调用连同包在导入时装上的处理器一起清掉了，
  不只是它自己文档里写的 custom 那些。本模块现在带上 `tests/test_toml.py` 用的同一个
  `_register_builtins()` 守卫，任何顺序下这些行都给出同样的答案。其中两个家族——越过 i64 的整数与非有
  限浮点——在无界策略里本来就稀有，于是各自占一条分支，为的就是让可达性守卫不必赌运气：400 次抽样下，
  最初那一版在两条 CI 腿上以不同的名字变红，3.10 没抽到非有限浮点、3.8 没抽到越界整数，那是戴着测试
  名字的抛硬币。把分支装回去以后，同一个守卫连续十次都见到十二个家族；再把它们摘掉仍会红，点名的是非
  有限浮点。
- **TOML 的注释机制两侧都有价格了** — 此前每条 TOML 地板跑的文档里都没有 `#`，所以
  `push_note_text`、解析器的 `take_comment`、以及解析器随表一起交回的那份注释清单，从不带动任何已提
  交的数字。计指令数的场景集就此加入 `to_toml_notes` 与 `from_toml_notes`，数字来自强制镜像上的测量
  （`20261026.354.1`、`rustc 1.97.1`、重复三次）：写侧用的是 `to_jsonc_notes`
  与 `to_json5_notes` 正在写的同一份六注释文档，读侧用的是一份装着 `MEDIUM_TOML` 全部取值、另加七条
  注释的文档——于是读侧拿到的是一段实测对照，不是一个命名前提：对着 `from_toml_medium`，
  这七条注释把 harness 的 2,000 次迭代除掉后每条值 1,945 条指令。两条地板
  各自最大的一行正落在注释上：写侧是 `pyrs_toml::writer::push_note_text`，
  读侧是 `<pyrs_toml::parser::Parser>::take_comment`。度量时把两种形状留在价格之外，其中一
  种后来查明是没被读到的写侧槽位、而非 TOML 的限制：渲染成内联表的映射里第一个键上方的注释，靠表自己
  的领先注释清单到达 `crates/pyrs-toml/src/writer.rs`，而没有任何分支读它，于是注释被丢弃；下面的修
  复分条给这个槽位配了读者，夹具测试如今期望它那六条注释全部到位。另一种仍是边界：文档最后一个键之后
  的独占一行注释没有 AST 槽位——解析器把它作为那对返回值的第二个交回、写侧根本看不见——所以
  `crates/pyrs-yaml-core/tests/ir_fixtures.rs` 断言它不出现在写出的文本里，将来给它槽位会在那里读成
  一个决定，而不是一条自己动了的地板。定价过程中量到、如今写成断言的一点：rustc 会把 raw string 里的
  CRLF 归一化，所以没有夹具会编译出 CR 字节，夹具*文件*存成哪种换行约定进不了地板——这是一条辅助脚本
  把 `bench_inputs.rs` 从历次十个提交都存的 CRLF 改写成 LF、把 39 行改动放大成 919 行 diff 时学到
  的。一次派发就在 `accept_drift` 为空、无拒绝的情况下写下基线：接过来的五十三条地板有四十七条动了，
  向上最宽 +0.385 %（`from_json_long_strings`），向下越过 0.5 % 的只有
  `from_json5_medium` -1.132 %、`from_jsonc_medium` -1.130 % 与 `parse_inline_merge` -0.551 %，而门
  禁要到 2 % 之外才告警；这个 head 没有任何引擎路径改变，所以那些移动是 harness 自身代码尺寸带动的兄
  弟场景。
- **Ir 计测工具现在受 lint 约束，并有规则让它一直受约束** — `crates/*/benches/ir_gate.rs` 两个文件都
  声明 `required-features = ["ir-gate"]`，而 cargo 对未开启所需 feature 的 target 是静默丢弃、不是检
  查，所以仓库那条 `cargo clippy --all --all-targets -- -D warnings`——CI 用的就是这个范围，当初采用
  它正是为了让 `tests/` 与 `benches/` 无处漏过 lint——从未编译这两份指令地板赖以测量的文件。实测：朝
  两个 harness 逐个植入类型错误，宽范围两次都报告树干净，而
  `cargo clippy -p pyrs-yaml-core -p pyrs-yaml --bench ir_gate --features ir-gate` 以 101 退出，先后
  点名 `crates/pyrs-yaml-core/benches/ir_gate.rs:325` 和 `crates/pyrs-yaml/benches/ir_gate.rs:147`。
  `ir` 作业本来就*构建*过 harness，所以编译错误过不去—— 没人读的是 lint。如今这条命令是 `rust-lint`
  的一个步骤，同时是按这两个文件划定作用域的 `prek` 钩子，且两个 clippy 钩子都列进 hooks 作业的跳过
  清单，因为那个作业的工具链没装 `clippy` 组件；`tests/test_ci_ir_harness_lint.py` 从各 crate 的
  manifest 读出被 feature 门控的集合、从 `prek.toml` 读出 clippy 钩子，两份清单都不必第二次手写；四
  种突变——去掉步骤、去掉 feature、去掉 crate、跳过清单忘改——各自打红一条测试。`QUALITY_MATRIX.md` 记
  作"靠构建检查、而非靠 lint 作业"的那条限制就此关闭，同段的场景清单也补上 #673 给该 harness 加的两
  行 `safe_dump_*`。
- **Python 对象写路径现在有了指令地板** —— `safe_dump` 与 `from_dict` 服务
  `crates/pyrs-yaml/src/py/direct_dump.rs`，而没有任何已提交的数字到达过它：引擎 harness 的
  `serialize_*` 场景止步于节点模型，而这两条对象路线根本不构造节点模型，于是绑定层自己写器做的每一次
  分配、扫描与拼写决定都不带动任何地板。`crates/pyrs-yaml/benches/ir_gate.rs` 增加 `Channel::Dump`，
  含 `safe_dump_small` 与 `safe_dump_medium` —— 读的正是读通道已经在用的两份
  文档（`SMALL_YAML` 与 `MEDIUM_YAML`），两者指令数相差 8.32 倍；对象由 `to_python_*` 自己的函数体在
  计数环之外构造，并且在 `--setup-only` 里同样构造，所以加载器的价格留在 `to_python_*` 里、不进这两
  条地板。数字来自在本 head 上派发的刷新工作流，在强制镜像上生成（`20261026.354.1`、`rustc 1.97.1`、
  重复三次），并且既进平表也进该镜像的按键行，因为绑定 harness 的场景按测量它的镜像键化。这次派发没
  在 `accept_drift` 里点名任何场景却仍然写成了，所以没有地板越过 0.5% 线；平表里四个绑定读数确实动了
  （最多 1.68%，`load_toml_medium`），因为它们来自的那个文件是在 `20261023.334.3` 上生成的，而强制腿
  现在读自己那一行，那一行没被动过 —— `QUALITY_MATRIX.md` 里九处被引用的地板读数由脚本重新对齐到本文
  件，彼此差异最大的是 0.0003%。这条路线的第一次归因：`needs_quotes` 是 `safe_dump_medium` 里最大的
  非分配器项，占 5.45%，`DirectWriter::write_key` 占 3.70%，而前十二名里没有 `type_registry` 函数 ——
  这就是在一个什么都没注册的文档上，量出了 #672 只能断言的那条空处理器快路径。
- **对标库的版本上限改为实测，并由门禁保持实测** —— 测试依赖组里九个 `python_version < 'X.Y'`
  上限中有五个仍写着 `< '3.14'`，而 `pyjson5`、`numpy`、`ujson` 与 `python-rapidjson` 都已发布
  3.14 的 wheel，于是 ≥3.14 的测试腿装到的对拍参照更少：PR #665 的十二条绿腿里 3.13 跑 4,433 条
  断言，3.14 只剩 4,197，3.15 只剩 4,196；逐个把某个对标库设为不可导入可以定位这 236 条的归属：
  141 条属 `pyjson5`，31 条属 `orjson`，54 条属 `numpy`，其余在普通腿上是 0。`pyjson5` 抬到 `<
  '3.15'`，`numpy`/`ujson`/`python-rapidjson`/`yaml-rs` 抬到 `< '3.16'`，因为它们新覆盖的每一条
  腿（含两条 free-threaded 腿）都有 wheel；`orjson` 与 `rtoml` 仍留在 3.14 之下，因为 PEP 508 没
  有free-threading 标记，二者都不发布 3.14t 的 wheel，放入 3.14 就等于放入 3.14t，那条腿会装不上
  整个测试组。`scripts/counterparty_wheels.py` 用真实解释器按 OS 与 band 记录可得性，
  `scripts/check_counterparty_caps.py`（已挂成 hook）既拒绝把没有 wheel 的 band 放进上限，也拒绝
  把每条相关腿都有 wheel 的 band 排除在外。从此一个陈旧的上限是一次指名依赖包的红色运行，而不是
  一次相当于整个 JSON5 规范表规模的静默跳过。
- **共享 Python 对象写出后的形状现在是被校验的事实** —— 对
  `{"a": shared, "c": shared}`，两条写出路线都输出子树的两份副本，
  而不是锚点加别名；重新读取后值相等，但两个值并非同一对象。
  往返路线仍保留 `&x`/`*x`，所以这层丢失属于对象路线。
  `tests/test_safe_dump.py::TestSharedObjectShape` 把这两半，
  以及环形图得到的 `YamlMaxDepthError` 拒绝，一起钉住；
  PyYAML 与 ruamel 在同一张图上实测，两者都保留共享，
  所以这是记名的分歧而非静默差异，`docs/dev/boundaries.md`
  写下这一分歧以及为什么补它不免费。此处行为未变。
- **pytest 档位的宽度改为派生，不再靠手抄** — 三份贡献文档写着阻断型 Python 档位是「4 个 Python 版本
  × 3 个 OS」，`QUALITY_MATRIX.md` 写的是「3 OSes × 7 Python versions」；而声明的解释器地板有 8 条
  （3.8 到 3.15），且这个档位根本不是笛卡尔积：`scripts/ci_matrix_plan.py` 让每个解释器都在 ubuntu
  上跑，macOS 与 Windows 只跑首尾两个，所以把两个因数相乘会数出 24 条腿，而实际排定的只有 12 条。矩
  阵探针现在去问那个计划器——3 个 OS × 8 个解释器，共 12 格——并自己重写这句；模式同时接受改写后的拼法
  与被替换的旧拼法，因为只匹配旧散文的规则在自己生效一次之后，就会永远报告“找不到句子”。
  `tests/test_ci_matrix_plan.py` 把三份本地化页面对同一份计划作比对，并拒绝不再恰好带三个整数的条
  目：在这里，被改写的句子是一次判红，而不是一条没人查的宣称。定时档位有意构建完整网格，因此这组数字
  描述的是合并判定所要读取的 pull_request 计划。
- **格式化器要提升的那一行，现在在它能动手之前就被拒绝** — 在钉住的 0.2.78 上实测，`rumdl fmt` 修
  `MD018`（标题的 `#` 后头少了空格）的办法，是把任何行首的 `#` 串重读成标题：折行的段落和多行行内
  code span 一视同仁，前者被劈成"句子、标题、句子"，后者被劈成两个真标题。改写之后那行就是合法标题，
  任何事后翻语料的规则都分不出它和作者本意的标题——于是五条分类谓词被实测否证（144、107、100、135 处
  误报，还有一条读出 0 却是空判：它把 `#` 行当块边界，正好抹掉要找的构造）。改写之前的形状唯一可判：
  行首 1 到 6 个 `#`、其后紧跟非空格字符、且不在围栏之内。
  `scripts/check_doc_headings.py --predamage` 拒绝它，并且住在 `format` 优先级的钩子插槽里、声明在格
  式化器之前——放在 `checks` 档就只能给格式化器的输出做检查。177 个跟踪页读出 2 处，两处都是活的"待损
  伤"，0 误报。边界也说清：它挡住的是提交，不是改写——注入样本时 preflight 点名两行、整趟钩子集拒绝提
  交，而格式化器在同一趟里照样改了工作区。
- **账本自己的引用规则成了钩子** — `docs/dev/quality-ledger.md` 用 `### (xy)` 给小节编号，而
  `(details: quality-ledger (bf))` 这样的指针只有在字母递增且不重复时才指向一个小节。此前只有 Python
  层裁决这条规则：一次改动唯一的缺陷是把一节早放了一个字母，它通过了钩子集、文档门禁、`rust-lint` 与
  Ir 腿，随后把三种操作系统上的十二条 `test` 腿全部判红。规则现在归 `scripts/check_ledger_order.py`
  所有——退出码 1 指名排错的那一节，退出码 2 拒绝一份没有任何字母小节的文件，而标题形式被改名后正是这
  样换来永久通过的——`ledger-order` 钩子盯住账本本身。控制项在工作树里实测：把 `(gq)` 与 `(gr)` 对
  调，钩子以 `(gq) follows (gr)` 判红；还原后是 9 条通过。`tests/test_ledger_section_order.py` 改为
  看守这个检查器与它的 `files:` 模式，不再持有规则的第二份拷贝。
- **正文里抄写的 Ir 地板现在与它引用的文件对账** — 一次刷新会重写 `.ci/ir-baseline.json` 的全部四十
  七条读数，而不带日期的抄写会变成没人标记的历史：上一次刷新落地九分钟后，`QUALITY_MATRIX.md` 引用的
  十一条地板里有九条与文件不一致（差 15 到 31,977 条指令），而所有门禁都是绿的。该页现在声明哪些读数
  按“当前”理解——每条后面跟一个写明场景名的注释——`scripts/check_ir_floor_quotes.py` 把每条声明与基线
  比对，拒绝过期数字、不存在的场景、数字被删后剩下的孤立标记，以及什么都不声明的页面（退出码 2，绝不
  算通过）。用声明而不是解析，是因为同一页本来就要保留被推翻的测量、事件计数与夹具尺寸。
  `tests/test_ir_floor_quote_gate.py` 逐条逼出这些拒绝，并证明钩子的 `files:` 模式真的覆盖到这一页和
  基线。未标记的手抄仍然看不见：缺标记被读成“这是历史”，而这句话只有作者可能说错。
- **键控的 Ir 地板按测出它的镜像引用** — 七个场景按 runner 镜像各持一条地板，执法腿只与自家镜像的
  行比对，从不读平铺的 `scenarios` 列。可标记此前正是与那一列比对，于是讲 `20261023.334.3` 上那条
  腿的句子，引用 `20261026.354.1` 的数字也照样过关：提交的文件里 `load_toml_medium` 的两行是
  144,902,515 与 142,467,241，差 1.71 %，而平铺列装着哪一张镜像的数，取决于上一次刷新恰好落在哪张
  镜像上。`scripts/check_ir_floor_quotes.py` 现在要求键控标记指名自己引用的行，写成
  `load_toml_medium @ 20261023.334.3`，并拒绝四种形态：键控场景不指名镜像、在指名镜像下却抄平铺列
  的数、文件里没有对应行的镜像、以及给未键化的场景安上镜像。该页十三条读数里四条是键控的；其余九条
  引擎侧读数保留裸标记——对它们而言，平铺列正是每条腿比对的列。未重标的页面被这个检查器拒了四次；
  `uv run pytest tests/test_ir_floor_quote_gate.py` 读到 22 passed；四次突变各删掉规则的一个分支，
  每次都只让归属它的那条测试变红。
- **写出器接住读入器交回的那个 tuple** — `? [a, b]` 在 Python 侧投影成键 `("a", "b")`，而此前两条写
  出路线对任何位置的 tuple 都抛 `Unsupported Python type`，于是 `safe_dump(safe_load(text))` 在一份
  读入器已经接受的文档上直接报错，`from_dict` 与 `_set_path` 也一样。现在 tuple 在两条路线都按序列写
  出；被当作键的序列改用节点序列化器自己的显式键写法——直接写出器把这一对交出去，而不是再镜像一遍
  `? `，两条路线因此逐字节相同。只有一种形状仍然拒绝，理由在读入器那边：被投影的只是键的*最外层*序
  列，所以 `? [[1], [2]]` 抛出 Python 自己的 `unhashable type: 'list'`（对照 ruamel 实测），一个内部
  还套着序列的键没有任何能读回来的写法。`tests/test_route_parity.py` 添了九行，每行都同时对两条写出
  路线与不动点断言（182 passed），`tests/test_edit.py::test_set_tuple_value_raises` 改名为
  `test_set_tuple_value_writes_a_sequence`。
- **`load_json_lines` 按行读取 `.jsonl` 流的值** — 引擎自 #599（`pyrs_json::parse_documents`）就能
  描述行分隔 JSON，但 Python 层没有读取它的入口：调用方只能自己按行切分再逐行调用 `load_json`，在
  解析之外额外付出一份行列表和每条记录一棵树。`pyrs_yaml.load_json_lines(text)` 返回一个
  `JsonLines` 迭代器，它的 `__next__` 只解析一行：规范 JSON 走 `load_json` 使用的同一条免 AST 路径
  直接生成 Python 对象，其余交给 `pyrs_json::document_from_line`，所以同一时刻只有一份文档存活，
  提前停下的调用方读不到下一条记录。输入文本只在构造时拷贝一次——省下的是语法树，不是缓冲区。
  拒绝保持引擎的契约：空行、只有注释的行、需要下一行才能完成的值都会抛出错误，并指出它在整个输入
  中的行号；再次取值报告同一条记录，而不是跳过它。本机实测（release 构建，15 次取最好，10,000 条
  记录 / 576.5 KiB）：整条流 3.861 ms，首条记录 0.343 ms，逐行 `load_json` 8.440 ms。三条 Rust
  测试把值面钉在事件面上——同样的行、同样的树、同样的拒绝与措辞——另有 16 条 Python 测试约束这个
  表面；两个 CodSpeed 场景承载这组数字。
- **刷新作业现在也能回答"花在哪里"，而不只是"花多少"** — 自 #401 把它登记成测量工具以来，
  `scripts/ir_gate.py --profile` 就能按自身指令数给一个场景里的函数排名，但此前只有人手跑过它，
  而手跑的剖析配不上已提交的地板：本地构建与镜像构建是两份字节。`documents_short_strings`
  那个 -21.23% 的读数因此始终无人归因（`docs/dev/quality-ledger.md` (gk)）。"Ir baseline
  refresh" 现在接 `profile` 与 `profile_top` 两个输入，在强制这些数字的镜像上为每个点名的场景
  发一个 `--profile` 开关；这一步挂在 `if: inputs.profile != ''` 之后，所以谁也不点名时刷新仍旧
  只付一次测量。归因会带着符号重新构建，并在测量之前返回，因此它动不了任何地板。
  `tests/test_ir_baseline_workflow.py` 里有两条测试，分别钉住输入真的到达命令、以及空的默认值
  保住拒绝。已发布的行为没有移动，测量过的地板也没有。
- **aarch64 wheel 所走的标量扫描现在按整条路径来测，而不只测尾巴** — 每个 SSE2 字符串扫描
  都按 `target_arch` 分发，所以在 x86 CI 上标量循环只回答最后不足一个向量的那截；而 aarch64
  wheel 把它当作全部答案发布，此前没有任何测试问过它一个整长度的问题。现在有四条测试
  （JSON 读、JSON 写、TOML 读、TOML 写）要求标量循环与向量循环对同一个 80 字节正文给出相同
  下标：把 256 种字节值分别放到十三个偏移上，再从十二个起始位置读取，这些位置正好是一个向量
  的开始、结束与跨越；另外把一个三字节字符移到每个边界处。改坏任意一条标量循环（C0 上界、
  JSON5 的 `\r`、TOML 的多行分支、写器的 DEL 判定）都会让它自己的测试变红。这组比较按架构
  门控，不是自我对拍：非 x86 的腿上这条测试直接被编译掉，因为让标量路径只与自己一致什么
  都证明不了。
- **JSON 与 TOML 事件流在消费者停下的地方停下** — 此前 `parse_stream(..., on_event=...)` 对每种格式
  都先收齐全部事件再逐条回调，因此第一个事件就返回 `False` 仍要付出整次扫描的代价，文档后段的语法错
  误也仍会被读到并抛出。`pyrs_event::EventSink` 现在返回 `ControlFlow`，
  `pyrs_json::parse_stream_with_options_to` / `pyrs_toml::parse_stream_to` 在产出每个事件时即刻交给
  sink：回调在 `Python::attach` 下运行，生产者在 `py.detach` 下运行。`False` 能停下的是格式自身的事
  实——JSON/JSON5 在下一个事件的字节之前停止走查，停止点之后的错误永不被读到；TOML 从扫描已构建的中间
  表示停止发射，因此被拒绝的文档仍会在任何事件送达之前抛出；YAML 是 granit 之上的推送适配器（扫描无
  法中止），`False` 只停止送达，增量入口仍是 `load_stream`。收集式入口现在以 `Vec` sink 委托给 `_to`
  入口，迭代器与回调看到的流因此结构化地相同。回调抛出的异常优先于其后的解析错误，`False` 返回
  `None`。`tests/test_streaming.py` 的七条测试钉住停止前缀、各格式的拒绝事实与错误优先级。
- **每个流事件都指认它来自的源字节** — 事件此前只带行列号，希望从某个事件处续读的消费者只能沿着自己
  必须保留的文本往回数行。`StreamEvent` 现在为每种生产者的每个事件都带上 `offset` 与 `end`：`offset`
  是该事件所指的第一个源字节，`end` 是最后一个字节之后，两者都是落在源字节长度之内的字符边界。区间是
  词法记号而非子树——带引号的标量包含引号，flow 分隔符只占一个字节，块容器为空——在生产器只记录位置而
  无范围时 `end == offset`：TOML 的全部事件（其 IR 只存起点）、YAML 的流标记与块容器。文档标记在源码
  写下 `---` 或 `...` 时覆盖这三个字节，没有写时则为空。
  granit 交给 YAML 的是字符下标，文本路径经它本已计算的偏移换算，增量读取器则为每个非 ASCII 字符保留
  一条 `(chars, bytes)` 标记、ASCII 一条都不留，这正是 `load_stream` 的分块内存保持平坦的原因。
  `parse_stream` 与 `load_stream` 的事件字典多出这两个键。`tests/test_streaming.py` 的十七条测试钉住
  全部五种格式与 YAML 两条路径上的区间，其中包括按单字节喂入的读取器。
- **行分隔的 JSON：一行一文档，可选每分片一个 sink** — `pyrs_json::parse_documents*` 用与
  `parse_stream` 相同的一行走查描述 NDJSON/JSON Lines，每个事件与每个错误的位置都对着整个输入。
  空行、只有注释的行以及需要下一行才能完成的值会被拒绝而不是跳过，所选方言（JSONC 或 JSON5）应用到
  每一行，流的形状与 YAML 引擎一致：一个 `StreamStart`、每行一对 `DocumentStart…DocumentEnd`、
  一个 `StreamEnd`。可选的 `parallel` 特性把行切成连续分片，每片一个 sink、一个线程，生产者与 sink
  之间不缓冲任何东西；因为每个位置都是全局的，分片按 `offset` 归并。在 200,000 行（20.5 MiB、
  24 线程、交错取中位数）上实测：单 sink 112.3 毫秒，24 个分片 18.9 毫秒（5.96 倍）。单个共享 sink
  无法并行——142.1 毫秒，有界有序流水线 35.6 毫秒——因为一个 `StreamEvent` 是 136 字节，一行 107
  字节产生约 15 个事件，缓冲的代价约为输入的 19 倍。
- **可选 `mmap` 读路径** — 开启后 `parse_file` 直接从内存映射解析非空常规文件，省掉
  `read_to_string` 的整文件拷贝；默认关闭（`default = ["numpy"]` 不变）。空文件、目录、
  非常规路径与非 UTF-8 输入保持原有报错；详见 `docs/dev/boundaries.md`。

#### 修复

- **模糊测试的报表不再把发现丢掉** — `scripts/fuzz_rounds.sh` 原先用
  `grep -m1 -hoE "panicked at [^:]+"` 拼每条崩溃签名，而 `[^:]+` 停在第一个冒号，所以一条
  panic 到报表里只剩 `panicked at fuzz_targets/yaml_roundtrip.rs`：没有行号、没有列、没有消息。
  也没有别处带着那段文本，因为 `cargo fuzz run` 的输出写进一轮一份、从不打印的日志。后果是
  可数的：按计划跑的发现层连续五个周六都是 `completed/failure`（2026-10-03、-10、-17、-24、
  -31），而没人说得出它找到了什么。签名现在同时留下位置与其后第一行消息，这正是让归档
  artifact 可用的前提——对照今天这次跑，`yaml_roundtrip`、`toml_roundtrip` 与 `edit_yaml` 各报出
  三条崩溃输入，随后几分钟就依归档字节复现出两处写器契约违反。制表符被压平，因为
  `findings.tsv` 用它分隔两列，而 `.github/workflows/fuzz.yml` 以 `cut -f1 | sort -u` 数签名，
  消息里混进一个制表符就会少报作业打印的数。`tests/test_fuzz_rounds.py` 多出两条用例，把旧
  `grep` 放回去就让两条都变红（实测 `2 failed, 6 passed`）。该由谁来*消费*一次红的计划跑——
  开 issue、挡某条路径，还是只当报告——是另一个决定，此处不作。
- **章节头的两条注释栈不再是二选一** — 章节键与它领衔的映射各有自己的 leading 列表，而
  `crates/pyrs-toml/src/writer.rs` 的章节 pass 只填一个切片：先问章节自己的列表，空了才回退去读键
  的。于是两者都非空时就少写一栈——键上带 `# K`、`[a]` 上带 `# T` 的节点实测只输出
  `# T\n[a]\nx = 1\n`。被截短的文字再发射仍是它自己，所以固定点、可再解析、树相等全部通过，一份丢了
  行的文档照样绿。这一层里唯一把两个槽都记账的预言机是 `prop_toml_writer_carries_every_note`，它在
  `crates/` 与主干逐字节相同的 head 上以 20,000 例抽到了该形状——主干因此是间歇性红，而重跑变绿并不等
  于修好。现在两条列表按顺序拼接，键的行位于章节自身之上，正是本 crate 的读器交回双注释头栈的顺序，
  所以第一次重发射就收敛。`a_key_line_stack_beside_a_section_stack_loses_neither_note` 钉住字节与固
  定点；出货路由（YAML→中枢→TOML）上同一对形状由
  `a_header_note_stack_that_arrives_by_way_of_yaml_reaches_the_section` 钉住；绑定层的同名一条
  `test_a_section_key_note_beside_the_section_note_travels_the_route` 还把同一文本交给 `tomlkit` 与
  `tomllib` 读回同一个值。反证就是修复前那一行——
  两个列表只读一个，两处都变红，左侧分别是 `# T\n[a]\nx = 1\n` 与 `top = 1\n# T\n[sec]\nx = 1\n`。代
  价是那道守卫而不是注释本身：两个列表写成两个循环时 `to_toml_medium` 高出地板 +0.46 %（同样的顺序经
  `Iterator::chain` 要 +0.83 %，两者输出的字节完全相同），而真带注释的场景不动——`to_toml_notes`
  −0.03 %、`to_toml_long_strings` −0.02 %。
- **五个 TOML 注释槽位没人读，而看不出丢注释的那一层也补上了** — `to_toml` 在需要的行不存在时就把注
  释丢掉：自带注释的空表或空数组、注释自成一行的标量值，以及两处用 `or_else` 让一行只认一个槽位的情
  形——键的行内注释与它值的行内注释并排，以及与它 section 头的行内注释并排。原先没有任何性质问过这件
  事：第一次写就丢掉注释的文本，第二次同样丢，于是可重解析、不动点与树的相等三条都在一份已经少了东西
  的文档上通过。新加入的 `prop_toml_writer_carries_every_note` 要求树上每条注释都抵达写出的文本——先
  trim、压成一行，折行规则在检查器里重述而不从写器借用——20,000 例之内就把四种形状全部点名。它们改用
  键上方那一行，正是本 crate 的读者回报成员前置注释的位置，于是第一次重写就稳定，而值仍保持紧凑的
  `{}` / `[]` 写法。`crates/pyrs-toml/src/writer.rs` 里五条点名测试钉住这些形状，
  `tests/test_toml_writer_suite.py` 让三条值侧形状走通已发布的 YAML→中枢→TOML 路线并被 `tomlkit` 与
  `tomllib` 读回，而一次突变——把新的 helper 改成对任何节点都回 `&[]`——同时让这五条与那条性质变红。代
  价是那道守卫而不是注释本身：钉住的 `rustc 1.97.1` 上，`to_toml_medium` 相对地板 +3.29 %、
  `to_toml_long_strings` +3.43 %，而这两份文档根本不含 `#`（每份 harness 文档 462 条指令），
  `to_toml_notes` +1.24 %；`docs/dev/quality-ledger.md` `(hw)` 记下读数，三个名字进基线再生的
  `accept_drift`。留下的还是位置而不是存活问题：自身不带注释的 section 头，其键的行内注释仍挤在标题
  行上，并在第二次写时移到自己的行——实测如此、没有丢失，也不在不动点性质的射程内。
- **容器第一个成员上方的注释被丢弃，而非挪走** —— `to_toml` 把
  `nested:\n  # above the inner key\n  inner: 1` 写成 `nested = {inner = 1}`，把
  `arr:\n  # above the first item\n  - 1` 写成 `arr = [1, 2]`。中枢把结束于成员上方那一行的注释安在
  *容器*节点自己的领先注释清单上，而 `crates/pyrs-toml/src/writer.rs` 的两个紧凑分支只读成员携带的槽
  位 ——键的领先清单、键的行内注释、值的行内注释、序列项自己的注释——于是容器算作未装饰、留在同一行，
  注释无处可去。这是 PR #146 那个第五槽位问题高一层的样子。两个分支现在都经
  `container_leading_notes` 问容器自己的清单，而那正是 PR #114 为节标题读的那一对槽位，所以这个问题
  每个容器只花一次查找，不再是先立卫兵再查一次；同时把容器降级：注释取花括号或方括号内的独占一行，那
  正是本 crate 的读器报告它的位置，所以文本一跳就回到自身。每一半都写成断言
  ——`crates/pyrs-toml/src/writer.rs` 里表的领先清单、遗留的独占槽位与数组各自的精确字节加不动点，
  `crates/pyrs-yaml-core/tests/ir_fixtures.rs` 里夹具的全部六条注释，以及
  `tests/test_toml_writer_suite.py` 里出厂的 YAML→中枢→TOML 路线。多行内联表与数组是 TOML 1.1 语法，
  所以这笔代价只有注释能招来：把注释去掉，同一份文档仍写成 `nested = {inner = 1}` 与
  `arr = [1, 2]`。哪些读者会拒读带注释的那份，是关乎版本的事实，2026-10-29 实测：CPython 3.11 至
  3.14 的 `tomllib` 拒读；CPython 3.15.0 的 `tomllib` 与 `tomli` 2.5.0（3.11 以下各腿用的那份）都读
  得动。所以这道门按读者与版本键控期望，而不再假定，1.0 的裁判仍是本 crate 自己的 `from_toml_v1_0`。
  空容器曾是属性层在 20,000 例下找出的例外：自带注释的 `A = {}` 保住紧凑形却丢了注释，因为 `{ }` 里
  那行 `#` 落不到任何成员上，而本 crate 的读者在这种位置不交回任何注释。`docs/dev/quality-ledger.md`
  的 `(hu)` 把这个形状记成实测边界、`(hv)` 确认了它；本清单里排在它上面的那条把该形状移出边界，因为
  键上方那一行正是读者回报成员前置注释的位置。代价落在写端，并在执法镜像上量得：`to_toml_medium`
  +1.256%、`to_toml_long_strings` +2.081% 与 `to_toml_notes` +6.872%；两个 TOML 读地板只动了
  +0.0014% 与 -0.0007%，读路线从不查问的槽位没有让它多付什么。
- **注册的处理器接管它声称拥有的标量，也只接管标量** —— #671 留下的三种形状现在收成一条规则、
  一个答案，并且是在落下它的那棵树上量出来的。对一个只带 `__bool__` 的类（除处理器外没什么能替
  它说话），`safe_dump` 把 `{"m": Booly()}` 写成 `m:\n  !booly B\n`，而 AST 路线写成
  `m: !booly B\n`，因为直接写器是经由 `write_other` 才走到注册表的，而它自己输出缩进；同一个类
  放进序列项会让那一项丢掉破折号行，作为键时它抛 `YamlTypeError`，而 AST 路线写的是
  `!booly B: 1`。数组路线正卡在同一个排序的另一侧：为 `numpy.ndarray` 注册处理器之后，直接写器
  让缓冲路线先读了数组，所以 `safe_dump({"m": numpy.arange(4)})` 交出数组自身的
  `m:\n  - 0\n  - 1\n  - 2\n  - 3\n`，而 AST 路线交出 `!nd ND`。两条写器现在都在数组之前、在数值
  协议之前、在自己的拼写规则之前问处理器，位置覆盖值、键、序列项与紧凑项。有两个家族保留自己的
  拼写，而裁定的这一半是把 AST 路线挪回去：为 `str` 注册的处理器过去会在那条路线上重写文本
  （写出 `holder: !s <abc>`，而直接写器写 `holder: abc`），因为字符串本来就是 YAML 自己的标量，
  读者解析出的就是那段文本，所以 `python_type` 是读侧的事；而 `dict` 或 `list` 仍按调用方数据的
  形状写出——这一行在改动前后都通过，这正是它构成边界而不是修复的理由。新增
  `type_registry::claims`，因为一项能否保持紧凑是排版问题，而排版问题不许靠运行处理器来回答。
  六个测试函数加入拥有这些行的两个模块（五个在
  `tests/test_community_plugins.py::TestHandlerOwnsTheScalarNotTheString`，一个在
  `tests/test_numpy.py::TestNumpyBufferRoute`）；六条里有五条在本次落地的树上失败，而七个改动地点
  每个被单独改成惰性，并由变红的行点名——10、2、1、1、1、1 和 1——所以没有一处是装饰。有一行是这条
  裁定必然要动的：`!time` 同样是已注册的处理器，所以 `safe_dump({"v": datetime.time(9, 30)})` 从块状
  拼写 `v:\n  !time 09:30:00\n` 变成 `v: !time 09:30:00\n`；套件里只有这一行钉着旧形状，现在它钉住
  行内拼写，也钉住不许动的那一半——两条写路径都读回同一个时间。裁定没有决定的那件事是写明而非吞下：
  AST 路线仍会给处理器的文本加引号（`v: !time "09:30:00"`），而直接路线照 `to_yaml()` 写出的样子
  输出；`date` 之所以一致，只因它的文本里没有冒号。
  `docs/en/guides/plugin-development.md` 写下了这条规则。没有指令地板到达 `direct_dump`，因此每个
  标量多出的那一次类型比较不被门禁定价（`docs/dev/quality-ledger.md` (hr)）。
- **注册的 `CustomType` 会接管它声称拥有的值** —— 当 `python_type` 是带 `__index__` 的类时，
  `register_type("!money", ...)` 被两条 YAML 写路径绕过：`safe_dump({"m": Money(500)})` 输出
  `m: 500`，也就是 Python 数值协议交回的整数，处理器的文本只在 `f64` 提取恰好接住的那个类上出现。这
  就是 `Decimal` 一直被尊重、整数形态的值类型却从来没有的原因——两条写路径各自唯一的那次注册表查询是
  为浮点形态写的，与上面一行里数组路由需要修正的是同一个顺序问题。
  `crates/pyrs-yaml/src/py/direct_dump.rs` 与 `crates/pyrs-yaml/src/py/python_types.rs` 现在都在数值
  协议之前询问注册表，覆盖值、键与序列元素三个位置；唯一的例外在两侧写法一致：`int` 或 `int` 子类——
  以及属于它的 `bool`——保留读者会解析出的整数拼写。本机实测：三个位置在两条写路径上都输出
  `!money 500c`，且 `{Money(500): 1}` 能经 `from_yaml` 往返。直接写路径的 `sort_keys` 文本随之改动，
  因为序列化器的比较器读取键节点的标量值：`Money(5)`/`Money(50)` 在那里按 `5c` 与 `50c` 排序，若继续
  用协议的 `5` 与 `50`，同一条映射就会在一条写路径上排出相反的顺序。
  `tests/test_community_plugins.py` 新增八个测试函数（九个用例），四个反证控制——每个注册表调用点一个
  ——分别令 6、5、4、1 个用例变红；`direct_dump` 的值位置还承接了原有两个浮点行，这正是“新问句是被替
  换者的超集”的度量。`docs/en/guides/plugin-development.md` 写明了该规则。没有指令计数地板覆盖这条路
  径，所以每个标量多出的一次类型比较不受门禁定价（`docs/dev/quality-ledger.md` (hq)）。
- **3.15 上转储 NumPy 数组不再终结进程** —— 数组路由把对象交给 `rust-numpy` 强转后再读它的内存，而
  `cp315-abi3t` 轮子这样做并不安全。在已发布的 0.18.3 上实测：3.15 GIL 下
  `safe_dump(numpy.arange(8))` 以 `Fatal Python error: Segmentation fault` 死亡（退出码 139，Windows
  那条腿报 `Windows fatal exception: access violation`），同一份二进制在 3.15t 下回答
  `YamlTypeError`，而 3.12、3.14 与 3.14t 都转储正确。它是确定性的、单线程的，`numpy.zeros((0,))` 同
  样会崩，所以故障落在第一次读取数组内部数据之处，而不在任何竞态里。
  `crates/pyrs-yaml/src/py/ndarray.rs` 现在向数组本身索取 `dtype`、`shape` 与 `tobytes()`，再在 Rust
  里解码这块缓冲区：任何产物都不再触及 numpy 的 C API，`numpy` 这个 Cargo 特性门控的因此是路由而不是
  依赖。另有两处缺陷随之倒下。第一处在每个解释器上都看得见：旧路由读的是内存顺序、嵌套却按逻辑形状，
  于是 `safe_dump(numpy.arange(4).reshape(2, 2).T)` 写出 `[[0, 1], [2, 3]]`，而数组说的是
  `[[0, 2], [1, 3]]`——Fortran 序与跨步数组现在按逻辑值写出，跨步的也不再被拒绝。第二处由装着的 numpy
  版本决定，因为两条写器路由都在问数组路由之前先问了 Python 的数值协议，而这个协议会*替*一个单元素数
  组回答：`float(numpy.array([1e30], dtype="float32"))` 是拓宽成 double 后的
  `1000000015047466200000000000000.0`，维度随之丢掉，而 numpy 直到 2.3 才把这个行为改成报错。于是一
  台机器写 `1.5`，另一台写 `- 1.5`；`docs/en/guides/numpy.md` 为显式布尔往返推荐的那个写法
  `numpy.array([True])` 回答的是 `1`。现在带维度的数组被拒绝进入该协议，0-D 数组保留同一页所声明的标
  量拼写，接受的 dtype 与每个标量的拼写都不变。`tests/test_numpy.py::TestNumpyBufferRoute` 钉住十一
  条，其中三条在装有 numpy 2.2.6 的已发布轮子上判红、在这个工作区上通过；随后同一只重建的轮子承载整
  个 54 条模块通过——3.8 配 numpy 1.24.4、3.10 配 2.2.6、3.12 与 3.15 配 2.5.4，3.15t 上是 53 条通过
  加一条按设计跳过的 GIL 并发——那正是此前根本没法运行这个模块的两个解释器。
- **`load_stream_file` 的内存上界现在真的被度量，而不是永远跳过** — 守着这条承诺的测试用
  `pytest.importorskip` 索要 `psutil`，可没有任何依赖清单声明这个包：`pyproject.toml` 与
  `.ci/requirements-test.txt` 都没有。于是在每一条腿的每个解释器上它只报一个 skip——看着像覆盖，实际
  一点没有。现改成基于标准库 `tracemalloc` 的标度不变量：同一路线读完 2,000 行与 200,000 行文档，留
  下的被追踪内存必须相同，今天的读数是两种尺寸都留 3,168 字节（3.14t 是 3,200）。边界写清而不默认
  ——`tracemalloc` 只数解释器自己的域，所以引擎侧读缓冲（固定 `DEFAULT_CHUNK_SIZE`，此处从不累积）落
  在度量之外，这也是旧的常驻内存数字不再复原的原因；"某条路线读了文件多少"仍由旁边的 `read_calls` 守
  卫守着。界值是同一次运行里累积式增长量的百分之一，两半都能在同一文件里被证伪：流式读数始终差 0 字
  节，而累积式读法在 3.12 上增长 199,854,440 字节、在 3.14t 上增长 242,617,688 字节，所以累积型读法
  会让守卫判红。最初写的是固定字节容差，`test-freethreaded (3.14t)` 那条腿把它否证了：同一份小文档在
  那台 runner 上留 292,544 字节，而本机只有 3,168——差在构建怎么数内存，不在这条路线读了多少。
- **`to_toml` 与 JSON 写器现在拒绝一对只有 YAML 分得开的键** — tag 属于键同一性的一部分，所以读方把
  `c: 0` 与 `!A c: "!"` 当成两个键收下，而 TOML 与 JSON 只按文本作键。投影过去会发出 `c = 0` 加
  `c = "!"`——本项目自己的 TOML 读方会判成重复键——以及 `{"c": 0, "c": "!"}`，`json.loads` 把它归约成
  `{'c': '!'}`，一条记录无声消失。两条路线现在都抛 `YamlSerializeError`，reason key 分别是
  `toml-keys-collide-without-metadata` 与 `json-object-keys-collide-without-metadata`。检查只对着"键
  上带 tag、锚或注释"的映射跑，所以没有装饰的文档每个键只多一个布尔判断。发现方式是属性层的
  `prop_toml_writer_is_fixed_point` 随机抽到了这个形状。
- **`to_dict` 与 `safe_load` 拒绝只有 YAML 能区分的键对** —— 写侧刚拒绝的那对键还有第三条路线，也是
  最安静的一条：Python 的 dict 只按文本作键，所以 `c: 0` 与 `!A c: "!"` 得出 `{'c': '!'}`，第一项消
  失且毫无提示。现在两条路线都点出撞车的文本并抛 `YamlTypeError`，形状与 `to_toml()`、`to_json*()`
  的拒绝一致；AST 仍保留两个键，`to_yaml()` 不变，不撞车的装饰照常投影。与对手库对照：ruamel 两项都
  留，PyYAML 在同一地点报错。
- **三条编辑地板不再定价一次出货调用方不跑的扫描** — 三个 `edit_*` 场景把 `line_offsets` 交成
  `None`，而编辑层对 `None` 的答复是重建文档的行表，那一张表是 `scan_yaml(text).line_offsets`：一整
  趟全文扫描，顺带决定 `has_hash`、`has_amp` 与 `is_ascii`。在基线自带的 `rustc 1.97.1` 上实测，这趟
  扫描在三个循环里各花 10,104,000 条计数指令，三者逐字节相同——因为它不可能依赖那次编辑。绑定层把这张
  表按文档 memoize（`Splice::line_offsets`），所以量具量的是"只有树、没有源文本"的那条路，而它的夹具
  是先解析再编辑。现在量具在解析旁边把表建好、把 `Some(&offsets)` 交给三个动词，`--setup-only` 像减
  掉解析那样把它减掉。执法镜像重采（run 81）三条读数 −24.45%（`edit_set_nested` 48,288,771 →
  36,483,718）、−27.48%（`edit_insert_sequence` 42,462,454 → 30,793,506）、−27.86%
  （`edit_delete_key` 42,392,088 → 30,583,030），比扫描自己 20.9% 的份额多出的那部分是这张 `Vec` 的
  分配、追加与释放。地板就是该次运行的读数，不是本地改出来的线；这三条现在断言的是"一份已解析文档的
  首次编辑，加上每轮必需的树克隆"——克隆留着，因为同一棵树编辑两次不是同一次编辑。
- **复合键内部的独立注释不能再被提到键外面** — `hoist_marker_note` 会把属于 `? … :` 块内某个节点的独
  立注释写到最外层 `?` 之上，因为读器重读时就在那一行报告它。于是只差这样一条注释的两个键会发成两
  对，而读器把两个键看成同一个并拒掉第二对
  ——`DuplicateKey("Sequence { items: [Scalar { value: \"null\", …")`，由属性层抽到（seed
  `de78e195…`；注释文本本身不含折行，所以这个形状早于 #645 放宽字母表就可能被抽到）。
  `keys_collide_in_text` 现在在一份丢掉全部独立槽的副本上回答，因而把发不出的那对键挡在生成的树之
  外；行内注释仍然区分两个键，两个方向都写成了名字——收缩后的反例留在
  `proptest-regressions/pbt.txt`，边界留在 `the_key_rule_agrees_with_the_writer_and_the_reader`。
- **注释不能再折断它落上的那一行** — `arb_comment` 从 `[ -~]+` 里抽注释文本，所以属性测试层没有任何
  一条能走到单行注释载体欠下的那次压平。把它放宽到也能抽 `\n` 与 `\r`，一次让四条属性变红：YAML 与
  TOML 写器把折行之后的文本写进了文档，于是 TOML 拒绝自己的输出
  （`bare carriage return (must be part of a CRLF pair)`），数组一项的注释 `"<\n&"` 回来时只剩一个裸
  `&` 值。两个写器现在都在单个 `push_note_text` 里把折行写成空格并直接写进文档：那个形状起初返回的
  `Cow` 每条注释都不分配，却仍让 `serialize_note_flow` 在强制镜像上贵了 +8.80%。落地这一形状的价格，
  由本次改动移动的地板说出：62,066,033 → 64,411,980（+3.78%），逐条注释问一次有没有折行的价钱。删掉
  任一写器的那条分支，它自己那两条属性重新变红；`proptest-regressions/fmt_pbt.txt` 留下已收缩的反
  例。
- **破折号和冒号为一个已经决定改走块式的子节点开了行** — `write_mapping_node` 与
  `write_sequence_node` 会在成员带的注释多到大括号装不下时，把标为 flow 的容器降级成块式；但七处判定
  只信 `flow_style` 旗标：`is_compact_item`、序列元素的起始行、映射值的起始行、`?` 标记的提取分支、
  `hoist_marker_note` 的两个分支，还有它的只读孪生 `spine_has_notes`。它们各自先写指示符，再让降级的
  子节点在父节点的列上摊开块体，产出 `- # first` / `# second` / `"": v`；被降级的值*序列* 更糟，任何
  读取端都不允许 `-` 出现在 `key:` 行上。属性层在 20,000 例里撞上元素这一支
  （`prop_style_setting_roundtrip`），而且只经由一个 `<<` 对：把两条注释叠到同一个键上，正是 merge
  解析做的事。七处现在都问降级自己那个问题；`spine_has_notes` 也补上了孪生早有的序列分支 —— 缺了它，
  骑在复杂*序列*键上的注释会留在容器自己的 `&anchor` 标记行之下。被降级的复杂键随后保住大括号：把溢
  出的注释提到标记之上，就抽掉了降级的理由，源文本要的 flow 拼法因此活下来。两张表测试在被上报的种子
  之外钉住十九种形状：每条都要能读回、注释与标量文本不丢、拼法与实际落定的形态一致 —— 降掉的容器不再
  带括号，装得下的仍带 —— 并且首轮产出即已收敛。逐个回退这七道守卫都会让某一行有名有目地变红，这说明
  七处都在承重。表不是唯一的防线：`arb_node_meta` 每个节点至多带一条注释，所以 `arb_custom_node` 的
  任何抽样都造不出触发降级的注释栈 —— 报出这一类的属性层其实再也报不出它第二次。新的生成策略按构造把
  溢出摆进八种角色（序列元素、映射值、复杂键、`&anchor` 头行下的复杂键；mapping 体或 sequence 体；带
  锚或带标签），256 例就能在七道回退里的六道上变红，剩下的 `is_compact_item` 由表守住。执法镜像给这
  道额外的问句开了价：写侧一族整体动了 +0.18% 到 +0.72%（`serialize_small` 最贵），读侧没有一条超过
  0.19%，所以这两条地板在那台镜像上重采，而不是放宽容差。这种形状"读取端就接受"的四种写法也进了
  `fuzz/seeds/yaml_roundtrip/`，于是每次 `cargo nextest` 都会由 `note_survival.rs` 和两读取端对拍门
  一起考问它们 —— 那道门还先得学会渲染复杂键，因为 `json.dumps` 拒绝元组当键。20,000 例复跑 809/809
  全绿；没有新增场景、桩或公开 API。
- **刷新作业不再丢掉第一个之后的场景名** — `profile` 与 `accept_drift` 都用 `read -ra <<< "$VAR"` 切
  分输入，而 `read` 只吃一行：刷新 run 40221999337 收到把两个名字写在两行的 `profile` 输入 —— 正是该
  输入说明鼓励的写法 —— 却只记录 "attributing 1 scenario(s)"，于是产出的 artifact 只带了一半的逐函数
  归因。文本断言看不见这件事，因为两种写法里那个旗标都在脚本中。现在两步都先把分隔符压平再切分，
  `tests/test_ir_baseline_workflow.py` 会把 `python3` 换成 echo 后真的执行每一步的命令体，要求打印出
  的命令带上两个名字、日志里的计数也对得上；对照组把单行 `read` 放回去，必须重现丢名字的那种行为。
- **合并裁定可能全绿而钩子集是红的** — `Test matrix (all legs)` 由 `ci.yml` 的作业算出，而
  `toJSON(needs)` 带不动另一个 workflow 里的作业，所以住在 `hygiene.yml` 的
  `prek hook set on the full tree` 从来不在裁定读取的名字里。PR #632 首个 head 上的实测：那条
  check-run 报了 `failure` — `check_changelog_coupling.py` 拒绝了一个挪动产品却没带发布说明的
  changeset — 而同一 sha 上的必需名报的是 `success`。这一步现在就在 `ci.yml` 里跑，只有一份且无
  条件，`hygiene.yml` 已删除，`quality_matrix.py` 也从裁定等待的那个作业读跳过清单。两个方向的
  守卫：扇入的精确集合测试会在作业进了文件却没进 `needs` 时变红，
  `tests/test_matrix_verdict_gate.py::test_the_hook_set_is_a_leg_and_exists_once` 则会在作业离开
  `ci.yml`、或第二份 `prek run --all-files` 在别处重现时变红 — 重复正是这个洞悄悄回潮的方式。
  引擎代码与已发布的行为都没有移动。
- **JSON5 的键位报错说出了方言允许的写法** — 键位置的消息此前写作
  `expected a quoted object key`，可 JSON5 的键位置也接受裸标识符，于是这条报错要求了一个语法
  从未要求的写法。措辞现在由方言自己的旗标决定：允许无引号键时是
  `expected an object key (a quoted string or a bare identifier)`，不允许时仍是原来那句。一个
  `Parser` 服务三种方言，两条走法读同一组旗标，所以这层共享的措辞同时覆盖建树路径与事件流路径，
  严格 JSON 与 JSONC 的原文一字未动。
  `the_key_position_error_names_what_the_dialect_accepts` 把方言与走法的五种组合都问一遍，在这层
  helper 出现之前它是红的；`test_json5_key_refusal_names_the_bare_identifier` 再透过已发布的
  Python 模块问同一个问题。
- **单行文档的事件流重回线性时间** — `LineTable::line_col` 过去为每个事件都从行首数一遍字符，
  源里没有换行时生产者每报一个事件就付出 O(n)，整条流退化为二次方：一个 1.5 MB 的压缩 JSON
  数组报告其 224,006 个事件要花 4.9 秒，输入每翻一倍耗时变为四倍。现在表在上次应答的位置保留
  游标，只走过相邻两个事件之间的字节；落在游标之后的查询——生产者补报此前暂存的注释时会出现——
  仍从所在行首重新计数。同一文档现在 19 毫秒完成，逐记录换行的孪生版本为 18 毫秒（开发机）。
  `visited()` 报告游标走过的字符数，因此
  `a_forward_sweep_advances_the_cursor_once_per_character` 不依赖计时就能断言 200 KB 单行的
  扫描只走一遍，另一条测试把前向、回退与码点中间偏移的答案与重扫结果逐一比对。JSON 与 TOML
  共用这张表；YAML 自行计算位置，从未有过这项开销。
- **部署站点的目录曾把每个小节排在版本标题同级** — 四个本地化 changelog 页面把 `[Unreleased]`、
  `[v0.18.3]` 与 `[v0.18.1]` 的小节放在与版本标题相同的层级，于是文档站把这些小节列在版本旁边而非其
  下；根文件一直正确，缺陷因此只出现在本地化页面。`scripts/check_changelog_mirrors.py` 现在检查这一
  层级关系（`level_errors`），`tests/test_changelog_placement_gate.py` 同时钉住干净的代码树与被注入
  的错误形状。
- **公开基准表现在覆盖四种格式，不再只有 YAML** — 测量面早就是五格式
  （`tests/test_benchmark_crosslib.py` 已与 `json`、`orjson`、`tomllib`、`tomlkit`、`pyjson5` 对拍，
  divan 侧有十二条非 YAML 基准），缺的是发布面：`.ci/perf-snapshot.json` 只记 PyYAML 与
  ruamel.yaml，每张表的小节标题都写着 YAML 文档。`scripts/bench_snapshot.py` 现在按格式轴测量——YAML
  测解析/序列化/往返，JSON、TOML、JSON5 测解析—— 并且每张生成的表由 `<!-- bench: ... -->` 标记绑定到
  它的测量（写明格式、操作、夹具与加速基线），不再靠数列数来认领。`tests/test_perf_docs.py` 两头都
  问：测了却没发布的用例要红，标记指向一个没测过的用例也要红。新增行里有两处是我们更慢，也照原样登出
  ——1.9 KB 文档上 `orjson` 快 1.5 倍，JSON5 配置上 `pyjson5` 快 2.4 倍——而 TOML 比 `tomllib` 快 5.5
  到 5.9 倍。JSON/TOML/JSON5 的写侧仍不发布，快照里记着原因：它们从引擎自己的 AST 出发而非 Python 对
  象，`json.dumps` 与 `tomli_w.dumps` 不是同一件事，那一半由指令地板定价。
- **Rust 后端的读取器进了公开表，其中两个赢了我们** — 此前的 YAML 表只与 PyYAML、ruamel.yaml 这两个
  纯 Python 库对拍，TOML 表只与 `tomllib` 对拍，于是真正拿来衡量 Rust 引擎的对手在发布记录里无处可
  见。`scripts/bench_snapshot.py` 现在在 YAML 轴上带上 `yaml-rs` 与 `ryaml`，在 TOML 轴上带上
  `rtoml`；重测的快照（2026-10-25）显示 `yaml-rs` 解析快 2.4–2.6 倍、往返快 1.6–1.9 倍，`rtoml` 比
  `load_toml` 快 1.9 倍，而 pyrs-yaml 序列化比 `yaml-rs` 快 2.2–4.1 倍、比 PyYAML 快 198–339 倍。输
  的行照输登出，并把理由写在旁边：那两个读取器进出都是普通 Python 对象——它们的往返是值的往返，不是文
  档的往返——而 pyrs-yaml 那一行构建的是后续编辑赖以保格式的树，`load_toml` 返回 dict 之前也要走这棵
  树。门禁随表一起改动，而不是继续描述旧表：`tests/test_perf_docs.py` 的库清单取自快照而非写死的五列
  形状，把表中 `faster`/`slower` 用词与它旁边的比值对账，要求每行的尺寸标签认领它数字来自的那个夹
  具、比值列说出它除的是哪个库，并让没有 `bench:` 标记的对比表直接变红。为此实跑了六个反证——过期的单
  元格、颠倒的方向词、删掉的行、移除的标记、调换的标签、不再标明基线的表头——每一条都按页面与行名失
  败。
- **已经闭合的缺口被写成还开着，如今是一次红灯** — `scripts/check_hole_claims.py` 原先只读
  `AGENTS.md` 与 `CLAUDE.md`，所以一条关于本树已不再复现的测量的过期断言得以在八处存活：五份变更日志
  镜像、`docs/dev/quality-ledger.md`、一条测试为自己存在的理由，以及早已闭合该缺口的 `ir-gate` 代码
  上的 Rust 文档注释。它还跟着两个 PR（#613、#614）一路全绿。现在的范围按现在时划定：两份指令文件，
  加上每份镜像的 `[Unreleased]` 区块，切分交给 `scripts/check_changelog_mirrors.py` 新增的
  `unreleased_lines`，于是「什么是 `[Unreleased]`」只有一套解析器。已发布章节与
  `docs/dev/quality-ledger.md` 仍是带日期的记录，那里可以点名一个此后已经闭合的洞。某份镜像切不出任
  何 `[Unreleased]` 行时以 2 退出而非通过：漏掉方括号的话会让这道门静默失效。
  `tests/test_hole_claims_gate.py` 里有四条反证：`[Unreleased]` 内的洞 id 会失败并报出路径与行号，同
  一个 id 放在已发布标题下通过，矩阵仍在测量它时同一个 id 也通过，默认范围丢掉任一镜像即为红。谓词也
  在真实树上跑过：把该 id 插到 `CHANGELOG.md:9` 返回 1，还原后返回 0。
- **键行上的注释不再在通往 TOML 的路上消失** — `to_toml("sec: # note\n  x: 1\n")` 原先返回
  `[sec]\nx = 1\n`：那条注释既没留在表头也没挪到别处，是被丢掉了。TOML 写出器的段落通道只读映射自己
  的行内槽，而 hub 把 YAML 文档键行的注释交给的是**键节点**——正是 #146 在下一层（内联成员）教写出器
  去读的那个槽。这次测量同时撤掉了本仓库用来解释另一处缺口的两个说法：`pyrs_toml` 自己的扫描器确实捕
  获了 `[sec] # note` 的行内注释，granit 根本不在该路径上，而 YAML 来源的 `sec: # note` 确实带着位置
  穿过 hub。仍未关闭的是 `api:set-comment-container-owner` 残差记录的那条摆放规则——TOML 来源的表头注
  释依旧回到该表最后一个成员行——该行为未变且仍被钉住。由
  `writer::tests::a_note_on_the_key_line_of_a_section_header_is_written_on_the_header` 与
  `TestSectionHeaderCommentBoundary::test_a_note_on_a_yaml_key_line_reaches_the_toml_header` 钉住；
  只读映射槽的反证会逐字重现丢失：`"[sec]\nx = 1\n"` 对上 `"[sec] # note\nx = 1\n"`。
- **段内条目键行上的注释不再在通往 TOML 的路上消失** — 上一条目为表头行补上的那个槽位，写
  `key = value` 这一行仍然没读：`to_toml("a:\n  b: # note\n    x: 1\n")` 交出 `[a]\nb = {x = 1}`，
  数组形状交出 `b = [{x = 1}]`，注释两处都不见；而 YAML 路线会重新写出它，TOML 出身的
  `b = {x = 1} # note` 两轮不变。`emit_pair` 现在退而读键节点的行内槽。表头是少数，每一对键值都要
  付钱。在执行地板的那台镜像上，用基线钉住的编译器、三轮取最大，TOML 写出侧的两块地板一起动：
  `to_toml_medium` 从 26,604,776 到 27,024,692（+1.58 %），`to_toml_long_strings` 从 28,255,071 到
  28,607,004（+1.25 %）；37 个场景里其余没有一个移动超过 0.32 %，所以这次改动自带的刷新在
  `accept_drift` 里两个都点名。内联表写法一轮就定；数组会展开并把注释带到最后一个元素上，在那儿定
  下来——是挪位置，不是丢内容。
- **六道门禁曾在"什么都没量到"时答"通过"**——`(fz)` 撞见过一次的那一类，如今一次扫清。
  `check_changelog_coupling.py` 在路径表为空时打印 `OK: … listed no files` 并以 0 退出；
  `check_doc_headings.py` 数的是交给它的路径，而不是真正打开的文件，于是一个不存在的路径
  读起来就像整棵树是干净的；`check_line_endings.py` 从不检查 `git ls-files` 的退出码；
  `check_doc_signatures.py` 与 `check_i18n.py` 在空的通配结果上照样打印 OK；
  `ir_gate.py --only a,b` 里的 `--only` 是追加，所以那是一个谁也匹配不上的名字，于是它一个
  场景都没比，却报告地板守住了。六处现在一律以 2 退出——就是落地断言门与桩类型门早已写明的
  那个结论——每处都由测试逼住。纯文档的改动集、只含不可比示例的一页仍然算通过，这条边界也被
  钉住：拒绝的第一稿把签名门自己的套件改红了。`check_i18n.py` 原本连一条自测都没有，
  `tests/test_i18n_gate.py` 是新加的。记为 `(gc)`。
- **十条断言可以在一条都没执行的情况下报告通过** —— `(gc)` 收的是检查脚本那一侧，同一个形状也躺在测
  试套件里。只有写在 `for` 体内的 `assert` 的测试，在空的可迭代对象上照样通过，而
  `all(x for x in [])` 就是 `True`。按 AST 扫 `tests/` 得到 41 个候选；其中十处的可迭代对象来自测试
  之外，现在各自带上一条写明实测数字的地板。`tests/test_dependency_merge_gate.py` 读
  `.github/dependabot.yml`，少于 3 条就拒绝（实测 4 条），一次锁住它的三条逐项循环，外加
  `test_every_entry_targets_main` —— 那是一条扫描谓词看不见的 `all(...)`。mkdocstrings 的 `paths`
  （实测 `['python']`）拒绝空表：被吩咐什么都不去读的处理器，照样能过那条检查目录存在的循环。API 页
  通配拒绝两种空形状 —— 一页都没有，以及某个页名只出现在一种语言里，此时页面与自己比较永远一致（实测
  28 页 = 7 个页名 × 4 种语言）。`load_stream` 与 `parse_stream` 的对拍现在写明两侧各 13 条事件、且
  长度相等，因为 `zip()` 会截到较短的那一侧；逐事件的键检查点名一对键值发出的 8 条事件。`a: 1` 的
  `walk()` 少于实测的 2 个节点就拒绝。七次突变 —— 清空来源、截断生产者 —— 各自把对应的地板逼红，每个
  文件还原后都按摘要核对一致；也正是这次运行暴露出 `YamlStream` 没有 `__len__`，所以长度相等这条不变
  量在把流物化之前根本无法表达。其余 32 个候选逐个看过后保留：写在函数体内、不改动同一函数就不可能变
  空的字面量，已由别的测试钉住的常量，具体兄弟用例已经定住形状的 `@given` 循环，以及空登记表本身就是
  要断言的那两条。还没有任何东西拦住下一条空循环测试：这次扫描是一次性脚本，不是注册检查，做成规则会
  连字面量与 `@given` 两种情形一起误拒。记为 `(ge)`。
- **跨库表里有一行点了竞品却从未计时** — `tests/test_benchmark_crosslib.py` 把读不进 fixture 的
  竞品退成 skip，而 `yaml_edit` 那条分支的注释把拒绝说成较新 yaml_edit 版本的行为。对着锁住的
  0.1.3 实测，与新旧无关：三个 fixture 全部抛 `Document::from_str: Input contains stream-level
  comments`，所以那一行是三个 skip 摆在已安装的包旁边，而只数通过条数的套件看不见一条从未运行的
  基准。0.2.0 是用有范围的 `uv lock --upgrade-package` 取的，不是整项目重解析带出的漂移：它解析
  `small` 与 `medium`，仍然拒绝 `large`，于是这一行现在计时两个 fixture。它的 `YamlError` 继承
  `ValueError`，既有那条 `except` 覆盖两个版本，变的只是走哪条分支。耐久的一半是下限
  `tests/test_leaderboard.py::test_yaml_edit_row_measures_something`：已安装必须意味着至少读进一个
  fixture。它住在排行榜文件里，因为基准文件在常规运行中被 deselect，而 `--codspeed` 又 deselect
  所有没有 `benchmark` 标记的用例（实测 `50 deselected`）——放在基准文件里，这条下限两条腿都不会
  跑。对照：venv 退回 0.1.3 它就变红，消息里带着版本号；在 0.2.0 上是绿的。记为 `(gg)`。

#### 变更

- **数进解释器的 Ir 地板改为按 runner 镜像键控** — `.ci/ir-baseline.json` 新增 `image_floors`
  表，`scripts/ir_gate.py` 让执法的那一趟跟自己所处镜像的那一行比，而不是拿一个数去问所有主机。
  哪些场景进表由 harness 映射派生，所以只有把解释器也数进去的那几条才有键：同一棵树、两个 harness
  摘要相同的前提下，`load_toml_medium` 在两个镜像上读出 142,467,241 与 144,904,012，这是一个地板
  装不下的 1.71 %。表里没有的镜像被拒绝（exit 2）而不是判过或判红；没有这张表的基线照旧判定。
- **`no_std` 的主机测试现在也在 Windows 上跑** — 那个 job 钉死在一台 runner 上，于是"`#![no_std]`
  crate 去掉 std 后逻辑仍成立"的证据只来自一个操作系统，而 wheel 是给三个系统发的。裸机构建与宿主无
  关，仍留在一台（`if: matrix.os == 'ubuntu-latest'`），这样矩阵不会为一次不可能出现差异的检查再付一
  遍依赖构建；会出差异的单元测试则铺开。这条腿是先实测再加的：在 Windows 主机上对六个基建 crate 跑
  `cargo test --locked --no-default-features`，724 条测试跑在 23 个 target 上、0 失败；而它之所以成
  立，正是因为同一批改动止住了三条清单边经 feature 统一把 `std` 重新打开。
  `tests/test_no_std_gate.py` 会拒绝退回单主机的 job，并写明它守护的是哪种不对称。
- **名为 no_std 单元测试的那条 CI 腿，现在真的跑在它名字所说的配置上** — `no-std-check` 把六个基建
  crate 交给一次 `cargo test --locked --no-default-features` 调用，它的注释记下当年加宽列表"值 112
  条从没在 std 之外跑过的主机测试"。用 `cargo test -v --no-run` 把每条 rustc 命令行里的
  `--cfg feature="…"` 读出来，那句话并不成立：单独 `-p pyrs-toml` 时它的测试单元一个 feature cfg 都
  不带，而同样命令一次点六个 crate 时，`pyrs_toml`、`pyrs_json`、`pyrs_ast` 都带着
  `--cfg feature="default"` 与 `--cfg feature="std"`。cargo 会在一次调用里把所选包的 feature 统一，
  而 `pyrs-yaml-core` 指向这三条兄弟的 dev-dependency 边没写 `default-features = false`。出货的东西
  都没受影响——dev 依赖不在任何下游图里，裸机构建从头就是以 `features=[]` 编译每个 lib 单元——但"在
  no_std 下测过行为"这句话，靠的是一条其实在测 std 构建的步骤。三条清单边把它关掉，门禁也跟着改成读
  清单而不是信 `-p` 列表：`tests/test_no_std_gate.py` 现在拒绝任何通往"声明了 `default = ["std"]` 的
  兄弟"却不带 `default-features = false` 的基建 path 边；反证控制用改动前的清单（它正好报出这三条
  边），另一条控制钉住读取器按 section 逐个走。修完后 CI 那一形状在 std 之外的单元里跑 23 个
  target、724 条测试全绿；默认形状仍是 `cargo nextest run --all` 的 815 条，
  `cargo clippy --all --all-targets` 在 `-D warnings` 下干净。
- **Ir 门禁为两条 YAML 读路径新增按注释定价的地板，此前只有写路径有** — `serialize_note_flow`
  给写器必须发出的那条注释定价，可没有任何地板给读器解析一条注释定价：五条 `parse_*` 场景读
  `SMALL_YAML`、`MEDIUM_YAML`、`BLOCK_STYLE_YAML`、`BLOCK_SCALAR_YAML`、`ANCHOR_YAML` 与
  `MERGE_INLINE_YAML`，两条读文档的 `stream_*` 行读 `MEDIUM_YAML` 与生成的长记录文档，这些文本
  里一个 `#` 也没有，于是 `parser/yaml/comment.rs` 的字节扫描、接收机对“这条注释归哪个节点”的
  判定和待归位槽的携带，每条地板都在最便宜的答案上跑满 2,000 次迭代。`parse_notes` 701,017,318 与
  `stream_notes` 579,067,612 读的就是写器那一行早已在写的同一个夹具，于是这两个新数字之间的差别
  只有路线，没有别的；`crates/pyrs-yaml-core/tests/ir_fixtures.rs` 钉住两个读器各见十一条注释，
  并钉住旧行所用的每个夹具一条也见不到。把同样 434 字节里的每一段注释换成等长的空格——两个锚点、
  两个标签、每一对花括号都不动——十一条注释在建树路线上贵出 +16.30 %，在事件路线上贵出 +19.37 %，
  其中 71 % 与 74 % 落在被锁定的解析器里（行级归因见 `docs/dev/perf.md`）。写出这一对地板的刷新
  起初自己拒绝了一次：`pyrs-json` 一行未改、绑定二进制的摘要也没变，`from_json5_medium` 却
  +0.55 %——动它的是场景集从四十九涨到五十一，于是这个名字被写进 `accept_drift`，
  `tolerance_hint` 保持 0.5 %。
- **多文档流交到单文档路由会被拒绝，而不是被择一返回** — `parse`、`parse_file`、`safe_load`、
  `to_toml` 以及 `YAML()` 实例的对应方法都只读一个文档。此前把带着两个文档的输入交给它们，返回的是事
  件遍历最后落脚的那一个：`a: 1\n---\nb: 2\n` 给出 `b: 2`，`a: 1\n---\n` 给出 `null`，而且没有任何信
  号说明有文档被丢弃。现在它们抛出以 `multi-document-input` 起始的 `YamlParseError`，报出第二个文档
  的行列，并指向 `parse_all_docs`、`safe_loads` 与 `parse_stream` —— 这三条仍然接受分隔符的路由，每
  一条都在同样的十种输入上实测过。PyYAML 与 ruamel.yaml 对这十种全部报错，却仍然接受开头的 `---`、结
  尾的 `...`、单独的 `---` 与空输入，这四种便被钉住作为守卫不得误伤的控制样本；CLI 的默认路由给出同
  样的拒绝，指向 `-A/--all-docs`，而 `-A` 自己的行为没有改变。没有声称的一点：位置与参考库并不一致，
  它们标记第二个文档的指令行，这里标记它的 `---`。此事连同它暴露出的合规性测试脚手架的路由错配，记入
  `docs/dev/quality-ledger.md` 的 `(hg)`。
- **Ir 门禁新增两条为编辑携带元数据定价的地板** — 每次替换都要问 `editing::metadata::merged_meta` 该
  带谁的注释、哪个锚点、哪个标签，`delete_path` 还要问 `extend_delete_over_comments` 把范围放宽到
  哪；可这些分支没有一条地板量到过：三个 `edit_*` 场景跑在 `MEDIUM_YAML` 上，那里零个 `#`、零个
  `&`、零个 `!`，所以三条分支在 2,000 轮里每次都落空。`METADATA_EDIT_YAML` 是同一个形状，只是
  `server.port` 带锚点、标签和注释，另一处的别名让这个锚点必须留着，被删的键上方还有一条独立注释；
  `edit_set_metadata` 与 `edit_delete_notes` 用现有两个动词跑它，于是把携带做贵就会动一个数。
  `crates/pyrs-yaml-core/tests/ir_fixtures.rs` 钉住每个答案，并在共享夹具被添上元数据时判红—— 那会让
  这一对失去对照。执法镜像写下 `edit_set_metadata` 42,838,349，与 `edit_set_nested` 36,483,730 并
  列，而带元数据的那份文档还小 26 个字符：把两个数拉开的是携带本身，不是更大那棵树的克隆。同一次刷新
  还动了它根本没进入的两条地板——`documents_short_strings` −10.46%、`documents_long_strings` −7.00%——
  门禁自带的剖析把这整个移动归到 glibc 的拷贝行与其 malloc 簿记，而两台机器上每一个引擎行的指令数逐
  条相同。地板数的是这次构建做了多少，不只是引擎做了多少。
- **两条 YAML 条目之间的空行现在是登记过的边界，并有测试钉住它** — 用已发布的扩展实测八个位置
  （根条目之间、嵌套键之间、序列项之间、独立注释之前、连续空行、块标量之后、文档首、文档尾）
  以及编辑路线：空行一处都不保留，而值、键序、锚点和注释都保留，返回的文本已是不动点。
  `tests/test_roundtrip.py` 现在按今天的行为断言，所以补上这个缺口会让断言变红，并在同一改动里
  逼出登记项与边界说明。之所以保持敞开：恢复排版要求序列化路径携带每容器状态，而这正是指令地板
  在每条 `serialize_*` 场景上计价的开销。已发布的行为没有任何改变。
- **两个锁文件都升到各自声明区间允许的最新版本** — Rust 侧只剩 `similar` 3.2.0 → 3.2.1 这一步，
  因为八个 crate 声明的 MSRV 1.96 地板挡住了其余更新；Python 侧共 9 个包：`zensical`
  0.0.71 → 0.0.72、`mkdocstrings` 1.0.7 → 1.0.8、`mkdocstrings-python` 2.0.9 → 2.0.10、
  `python-dateutil` 2.9.0.post0 → 3.0.0、`deepmerge` 3.0.1 → 4.0.0，另有 griffelib、markdown
  和两个补丁版本。刻意钉住的版本没有动：`maturin` 停在 1.15.0（它本身就是最新），`pyo3` 留在
  0.29 系列；最新的 `pytest`、`hypothesis`、`pydantic`、`numpy` 要求的解释器高于本版本仍支持的
  3.8 地板。本机实测：806 条 Rust 测试、4,306 条 Python 测试、
  `cargo clippy --all --all-targets -- -D warnings`、整套钩子、四 locale 严格构建全部通过。
- **注释在桥的两端各自有了地板** — 为了说明 PR #641 写侧地板为何移动，去数了基准夹具里到底有什么，结
  果把当时写在旁边的解释本身推翻了：那时 serialize 场景跑的每一份 YAML 输入 `#` 计数都是 0，五条写侧
  夹具的 `{`/`[`/`]`/`}` 计数也都是 0——于是 `notes_have_flow_spelling`、流式转块级的降级、
  `hoist_marker_note`、`spine_has_notes` 和两个 `lift_first_*_notes` 不回答任何被测量的问题，把它们
  改贵也不会让任何数字动一下。同一个计数也照出了出向的 JSON 半边：`to_jsonc_medium`、
  `to_json5_medium` 与它们的长值孪生场景拿到的是一份 `#` 计数为 0 的文档的 AST，所以一个把交到的注释
  全丢掉的方言写器照样会全绿。现在三份夹具各自带着自己那一半必须回答的形状。`serialize_note_flow`
  （执法镜像上 62,066,048 条计数指令）让一份文档走完那次遍历能给出的五个答案：保留花括号、因某个成员
  带两条注释而放弃花括号、`?` 标记把注释留在标记行上、标记自己没有注释而注释挂在脊上第三层、锚点头行
  的首项带注释；镜像自己的归因说这次遍历占该场景的 12.61%（同一谓词的两份实例合计 7,824,000）。
  `to_jsonc_notes`（17,002,806）与 `to_json5_notes`（17,120,806）把一份带六条注释的文档写过两个方言
  写器，run 75 的归因也点了名钱花在哪（那还在 PR #643 把块注释那半收成一次遍历之前）：
  `<str>::replace`——每条注释写成注释之前都要过的压平，而且只有注释文本能走到它——在两个场景里都是最大
  的自身项，16.08% 与 16.01%，其后是 `push_block_note` 和 `emit_value_block_notes`。它们的读取孪生
  `from_jsonc_notes`（50,684,157）与 `from_json5_notes`（50,864,157）之所以存在，是因为防线矩阵把
  `to_<格式>_<形状>` 与它的 `from_<格式>_<形状>` 配成一对，并且对提交进去的 harness 断言这一对；这两
  条读入一份七条注释的文档 ——五条行式、一条块式、一条在文档之前——最大项在两边都是
  `<pyrs_json::parser::Parser>::ws_extended`（19.86% 与 19.79%），而让注释找到所属节点的那一步
  `attach_pending` 是 1,484,000。`crates/pyrs-yaml-core/tests/ir_fixtures.rs` 按引擎真正发出的文本逐
  个钉住这五个答案、六条写出的注释与七条读回的注释，从三份夹具里任一份删掉一条注释都按名报红，所以地
  板不会悄悄变成守住一份没有注释的文档。扩大场景集把 15 条、接着 13 条、再接着 18 条地板推过
  0.005%，而越过 0.50% 线的名字三次并不相同：`from_json_medium` 78,671,707 → 79,283,878 → 79,904,977
  前两次每级 +0.78%，第三次它反向掉了 0.41%，改由四条本次根本不进入的读取场景越线
  （`from_json5_medium` +1.05%、`from_jsonc_medium` +0.72%、`parse_inline_merge` +0.67%、
  `from_json5_long_strings` +0.50%）。名字要从派发自己的拒绝里拿、而不是靠预测，这就是理由：run 74
  带着按前两次越线预先填上的 `from_json_medium` 仍拒绝写入并点名那四条，run 75 点名这四条才写进去。
  `tolerance_hint` 仍是 0.005。全部四十七条地板都来自那几次运行与 run 79 的重测，`QUALITY_MATRIX.md`
  §2 把数法写在明处。
- **评估文档的盲区计数改由列表算出，不再靠人记** — `QUALITY_MATRIX.md` 里那句
  `Seven blind spots were found by measuring` 写下时，它下面的列表其实已经有八条，而没有任何门禁能
  看出这个差别：派生出的层级行只数测试与场景，从不数段落。现在有一条测试会从那句话引出的列表里重新
  算出这个数，而且两种破坏方式都验证过 — 把数字换成 `Ten` 会让它变红，多塞一条也会让它变红 — 于是
  记下洞却忘了改计数，从此是一次红运行，而不是一句曾经为真的话。计数现在是九，因为钩子集那一腿进了
  这张表。测量过的地板与已发布的行为都没有移动。
- **指令门禁的散文改为只写实测过的东西** — `docs/dev/perf.md` 里有三句把「绑定通道很抖」记在
  `AGENTS.md` 名下，而那个文件并没有这句话（`grep -c noisy AGENTS.md` 是 0），它们转述的归因也错了：
  那个宽的读数是 CPython 每进程自取的哈希种子，如今已钉住。指针改为点名这一项，
  `QUALITY_MATRIX.md` 第 2 节里跨主机的上限被标明是它测量当时的那份读数——在钉住种子的树上重测，
  绑定通道相对镜像地板最差 +0.15%，而三个扫描族场景最多差到 -21.23%；这些都连同试过并被反证的
  两个解释一起记进 `docs/dev/quality-ledger.md` (gk)。补一条守卫测试，指向并不存在的声明就会报红。
  已测地板与出厂行为都没动。
- **行分隔路由有了属于自己的地板，而长值在这里比在事件路径上贵得多** — 记下事件流长值地板的那条
  记录，也留下了它没有闭口的一句：NDJSON 路径还没有属于自己的数。`Work::Documents` 现在用
  `pyrs_json::parse_documents` 把八条单行记录读成事件，这一对只在值的宽度上不同——9 个字符对
  256 个。在执行这套数字的镜像上，那是 95,815,186 对 328,951,144 条计数指令，**+243.3%**；
  同一个只改值长度的对在事件路径上是 +8.16%。三十三块地板都出自同一次运行，它只需要点一个
  `accept_drift` 名字：把 harness 变长让 `from_json5_long_strings` 从 100,340,744 走到
  101,160,437（+0.82%），而本改动根本不进入那条场景；`from_json5_medium`（+0.479%）与
  `from_json_medium`（-0.905%）靠自己留在了线内。重测也移动过一个已公布的比值：上一条记录写下
  8.33% 的事件对，在这次运行里读作 8.16%，`QUALITY_MATRIX.md` §2 现在带的是新的那个数——由
  `tests/test_ir_baseline_workflow.py` 对着文件核，而不是对着记忆核。
- **每条 Rust 基准现在各自一个进程跑** — `codspeed-divan-compat` 从 5.0 升到 6.0。上游的 6.0.0 给
  每条 Divan 基准另起进程，于是一条基准留下的分配器状态不再污染后面的基准，官方口径是抖动超过
  1% 的基准少约 70%；`CODSPEED_PROCESS_ISOLATION=false` 可退回旧行为。决定合并的东西都没动：
  判定仍是 `Test matrix (all legs)`，指令地板是提交在树里的 `.ci/ir-baseline.json`，文档里的
  墙钟数字仍来自 `.ci/perf-snapshot.json`。会动的是 CodSpeed 看板——6.0 自己声明升级会让部分
  数值漂移，所以跨过这次 bump 读出的趋势线不是同一次度量；run-to-run 的偏差正在同一 head 上
  实测，拿到数之前不声称结论。`cargo codspeed build --workspace --exclude pyrs-yaml` 用 5.0.3
  的 CLI 对着 6.0 runtime 建出了两套 suite；CI 装的 CLI 不钉版本，而 6.0.0 与 runtime 同日发布。
- **事件流现在有了属于自己的长值地板** — 记下四条长值读路径地板的那条记录，也写下了它留下的
  缺口：“`stream_medium` 与 NDJSON 路径仍然只带短值，而事件路径没有属于自己的长字符串场景”。
  一条场景补上事件这一半：`stream_long_strings` 用同一个 `Work::Stream` 跑 `LONG_YAML`，也就是
  四条写侧地板用的那同八条 256 字符记录，于是这些数字只在字符串的形状上不同。在执法这些数字的
  镜像上，长值形状花 276,633,813 条计数指令，`stream_medium` 是 255,359,778，即 **+8.33%**：
  这是本门禁第一次能给出的“事件路径上的长正文”读数。刷新只需点一个 `accept_drift` 名字：把场景
  数从 30 加到 31 让 `parse_inline_merge` 从 355,404,524 走到 359,069,238（+1.03%），而本改动
  并不进入那条场景；`stream_medium` 自己涨 +0.485%，还在线内，所以其余一个都不放行。如今三十一条
  地板全部出自那次运行，`QUALITY_MATRIX.md` §2 把它们念成文字，句子与文件不一致时
  `tests/test_ir_baseline_workflow.py` 就会失败。
- **长字符串度量面现在覆盖全部五种格式** — 向量化的读路径此前只为严格 JSON 与 TOML 记了长值
  地板，因为门禁把一座桥的读侧与写侧配对，而两条方言桥两侧皆无。现在四条场景把这份形状在读与写
  两端走通：`from_jsonc_long_strings` 141,062,599 条计数指令、`from_json5_long_strings`
  99,998,599、`to_jsonc_long_strings` 60,056,647、`to_json5_long_strings` 60,480,707——每条都是
  三次采样中的最大值、离散度 0.000%，30 个数字全部由刷新派发写在执法镜像上。那次派发不需要
  任何 `accept_drift` 名字：场景集从 26 增到 30，没有把任何一条推过 0.50% 线，最大的上行是
  `from_toml_medium` 的 +0.363%；同一次运行也把上一条改动记下的四条地板重测了一遍——
  `from_json_long_strings` 112,664,928 → 112,987,111、`from_toml_long_strings` 109,254,466 →
  109,535,451、`to_json_long_strings` 58,550,997 → 58,550,735、`to_toml_long_strings`
  72,438,982 → 72,438,675。夹具是 `bench_inputs::long_string_jsonc`（2,557 字节：同样八条记录，
  成员之间插一行注释与一段块注释）与 `long_string_json5`（2,550 字节：值用单引号——向量里由单
  引号那个实例扫描其字符串正文的方言——键是裸的，且每个成员都带结尾逗号）。
  `crates/pyrs-yaml-core/tests/ir_fixtures.rs` 把两者钉到严格 JSON 夹具渲染的那份记录列表、最宽
  游程正好 `LONG_VALUE`，并且两种注释形式都必须在，所以悄悄丢掉注释的夹具无法继续蒙住场景；它
  确实当场逮到了本作者写出的一个双逗号夹具。两条写侧地板同时记下出口扫描今天的代价，免得有人提议
  向量化它时没有基准可比。
- **长字符串形状第一次拥有常驻度量，两条通道都覆盖读写两端** — 本仓库每个基准读的都是短值
  （`MEDIUM_JSON` 最长的值只有 21 字符，两个排行榜文档构造的是 `item_N` 名字），所以本发行版
  向量化的批量扫描此前只能靠一次性探针来量。`bench_inputs::long_string_json` /
  `long_string_toml` / `long_string_yaml` 生成同一份记录列表、值长 256 字符——只有字母，
  没有任何让扫描停下的字节，由 `crates/pyrs-yaml-core/tests/ir_fixtures.rs` 钉住——两个 harness
  都在这两座桥的两端读它。排名通道拿到四条基准（读 2.599 µs 与 3.499 µs、写 950 ns 与
  1.174 µs，本机同一次运行）；执法通道拿到四条地板——读 112,664,928 与 109,254,466，写
  58,550,997 与 72,438,982。按字节算，执法镜像的那一次运行里，长文档约为 `from_json_medium`
  的五分之一、`from_toml_medium` 的八分之一，写侧约为各自 medium 孪生的五分之二——扫描而非节点
  才是这里的成本中心。写的一半不是装饰：`scripts/quality_matrix.py` 会把只给一侧编号的桥报成
  `ir-bridge-unidirectional`，只写读侧的第一版正是这样被抓住的。给 harness 加场景还会推动它从不
  介入的那条场景：上一条头在执法镜像上把 `stream_medium` 读成 +0.01%，这一条读成 +0.63%，变量
  只多这一个，那条 leg 因此变红。把 26 个数字重新记到那张镜像上是刷新作业的活，派发时点名
  `accept_drift stream_medium`，其余任何上移照旧自我拒绝。
- **定义被测地板的编译器不再出现在依赖机器人的报价里** — 每周的 `github-actions` 分组开了 PR #620 去
  升两个 action，顺手在 `ci.yml` 里把 `dtolnay/rust-toolchain@1.96` 改写成 `@1.120`；对 `msrv-check`
  这道工序而言，那个 ref 不是版本号，而就是要装的那个编译器。防线抓住了它 — 那条 head 上
  `tests/test_msrv_gate.py` 与 `tests/test_ci_rust_leg_exclusion.py` 里五条测试转红 — 但每周一都红的
  PR 只会被急急忙忙关掉再重发，而配置本身就保证重发：`actions` 分组带着 `patterns: ["*"]`，没有
  `ignore`。现在这个名字在 `.github/dependabot.yml` 里被排除，理由旁边写着测量，排除被改掉时
  `test_the_bump_bot_is_told_never_to_offer_the_floor_selector` 会转红；抬地板仍然是一件人做的事。
- **TOML crate 的自述写着不构建中间树，而它构建了一棵** —
  `crates/pyrs-toml/src/lib.rs` 开头那句 "no intermediate value tree"，是 `(fs)`、`(ft)`、`(fv)`
  三条记录各自记在"移除树的那次改动"账上的欠项。`(fw)` 把它量了出来：两张表插入行占
  `load_toml_medium` 的 8.10 %、占 `from_toml_medium` 的 9.68 %，而该剖析里更大的行是无树解析器
  同样要做的扫描工作。于是重写并未排期，这句话改为描述真实的构建。纯文档改动 — 行为、输出与
  计数都不动。
- **依赖机器人终于能读懂自己的配置** — `.github/dependabot.yml` 里三个 `groups` 条目写的是
  `version-update:semver-patch` / `-minor` / `-major`，那是 `ignore` 与 `allow` 的词汇表：
  `groups.*.update-types` 只接受 `major` / `minor` / `patch`。于是 Dependabot 拒收这份文件——在它落地
  九天之后，以一条没有任何合并门禁会读的 check-run 的形式，而所有 workflow 依旧全绿。这五个值现在改
  用短形式，唯一声明了类型的那条 `ignore` 规则继续在它该在的地方用前缀形式；
  `tests/test_dependabot_config.py` 分开钉住两套词汇表，并且任一侧被管的条目消失时直接判红，所以再有
  人把两边"统一"，会先撞红一条测试，而不是安静地停止升级。

#### 性能

- **块注释一次写完** — `to_jsonc_notes` 与 `to_json5_notes` 给"注释写成评论前那次压平"第一次定了价，
  而块注释那一半要分配两次：`push_block_note` 做的是
  `text.replace(['\\n', '\\r'], " ").trim() .replace("*/", "* /")`，于是一条块注释要先花两个中间
  `String`，再写进本来就是 `String` 的缓冲区。现在一趟写完：先裁边、把内部换行映射成空格、只在 `*/`
  可能提前闭合评论时才拆开。先裁边是等价的而非猜测——`\\n` 与 `\\r` 是空白，替换它们的空格也是空白，
  两种顺序掉掉的头尾一致。强制镜像读到 `to_jsonc_notes` 为 17,002,806、`to_json5_notes` 为
  17,120,806，比 PR #642 落下的地板分别降 38.27% 与 38.11%，其余 45 条都在 ±0.16% 内
  （`to_json_medium` −0.0004%）——这次刷新没点名任何漂移就写出了全部四十七条。第一版把六处行式站点也
  走同一趟，随后又撤回：它们在两条注释地板上再多换 2.5 个百分点，却让六条碰不到注释的写侧越过
  `ir_gate.py` 强制的那条线（`to_jsonc_medium` +1.63%、`to_json5_medium` +1.59%、
  `to_jsonc_long_strings` +1.44%、`to_json5_long_strings` +1.42%、`to_json_medium` +1.02%、
  `to_json_long_strings` +0.96%），同一棵树再跑一次逐条复现，而给新辅助函数加 `#[inline(never)]` 对
  这六条一个都没改变——所以代价就在那次编辑本身，不是这趟被内联；2.5 个点不值得在永远到不了注释的路径
  上付 1%。落地的部分仍由那两条地板与
  `a_note_is_flattened_and_a_block_carrier_splits_its_own_terminator` 守住：它钉住行式在注释文本为
  `"  first\\r\\nsecond  "` 时写作 `// first  second`、块式写作 `/* ends * / here */`，而移除 `*/`
  拆分就会让第二条断言变红。
- **一行 JSON 现在一次按八个字节来索引** — `pyrs_event::LineTable` 此前每步只数一个 UTF-8 字符，
  既用来找行首，也用来把游标带到下一个事件；而在强制指令地板的那台镜像上，这段走查占
  `documents_long_strings` 的 51.79%、`documents_short_strings` 的 21.09%
  （`docs/dev/quality-ledger.md` (gl)）。现在一个 `u64` 一次答八个字节：`zero_lanes` 找断点，每个
  候选 lane 再回查它声称的那个字节 —— 零 lane 的借位会把它后面的 `0x01` 也照成断点，这条是新加的
  property 测试在上线前抓到的。只要一个 word 里没有 0x80 及以上的字节，游标的列就按字节数前进；
  有的话交回逐字符走查。同一台机器、地板点名的那个编译器，前后各测两遍：`documents_long_strings`
  从 308,642,512 条计数指令降到 126,840,512（**-58.84%**），`line_col` 降 82.6%、
  `LineTable::new` 降 66.0%；`documents_short_strings` 从 75,506,559 到 64,952,559（**-13.98%**）。
  位置一个没变：property 会在任意文档的每个字节上把 `starts` 与 `line_col` 对一次重扫，另有一张表
  覆盖断点落在一个 word 八个位置里的每一种、以及一个 word 里八个断点。只用 core —— 无依赖、无
  unsafe，`no_std` 集合不变。地板随代码一起下调，读数来自强制地板的那台镜像：
  `documents_long_strings` 从 328,990,934 到 147,188,912（**-55.26%**），`documents_short_strings`
  从 95,854,968 到 85,300,932（**-11.01%**）；另有两场本地剖析从未点名的场景也一起降了 ——
  `stream_json_medium` **-14.90%**、`stream_json_stop` **-27.17%** —— 因为同一张索引服务于整个
  JSON 流路径。其余 38 场最多只动 0.0051%。
- **门禁读到的最宽离散来自解释器，如今已经钉住** - 三次基线刷新作业把绑定 harness 记在同一个
  二进制摘要（`7adbeac719edacd6`）之下，九个采样把 `load_json_long_strings` 铺开 1.121%，而
  同样字节上的引擎场景在 0.00001% 以内一致。链接 CPython 的只有一侧，那一项就是解释器每进程
  自取的哈希种子：在 `PYTHONHASHSEED=0` 下该场景三次都读到 95,136,157，七条绑定场景全部收敛
  （0.2811%、0.2550%、0.1249%、0.0787% 降到 0.0000-0.0004%）。`scripts/ir_gate.py` 把这颗种子
  钉在 glibc 上限旁边——当初消掉 1.44% 跨主机项的就是同一手法——本改动 head 上的刷新作业报告
  42 个场景彼此不超过 0.0010%。地板只在种子原先做工的地方移动：`load_json_long_strings`
  -0.526%，另外五条加载器 -0.166% 至 -0.055%，`to_python_medium` +0.051%，35 条引擎场景不超过
  0.001%，向上无一越线。线仍是 0.5%，因为仪器不再需要这点余量；键一旦丢失
  `tests/test_ir_gate_provenance.py` 会点名，已发布代码没有改动。
- **TOML 加载器走的那条路现在有指令地板了** - `(fs)` 把 `load_toml` 移到了
  `pyrs_toml::read_toml` 上，而两条 Ir harness 都不进这条路：引擎通道的 `from_toml_*` 停在中间树，
  绑定通道的 `to_python_*` 量的是 YAML 的 AST→Python 遍历。绑定通道新增 `load_toml_medium` 与
  `load_toml_long_strings` 两个场景，经一道特性门控的缝直接驱动 `load_toml` 本身；harness 在计数
  循环之前先问这条加载路自己的路线守卫，而不是顶着读取器的名字给节点模型定价。在执行地板的那台
  镜像上量、取三次里的最大值：144,803,683 对 `from_toml_medium` 的 120,986,802，140,182,211 对
  `from_toml_long_strings` 的 109,012,890 - 加载器是建树器的 1.20 倍与 1.29 倍，那段差额正是此前
  没有任何计数描述过的 Python 一半。地板从 33 个场景增到 35 个，增多这一步让同通道的三条
  `to_python_*` 分别动了 +0.212%、-0.357% 与 -0.111% - 都在 0.5% 线内，所以这次刷新没点名任何
  场景。JSON 加载器的快速路径仍只由墙钟把关，同形状的地板是后续的事。
- **`load_json5` 走上两个同胞早已启用的免 AST 路径** — 本家族的快速路径
  （`json_fast::try_load`，一趟直接构建 `dict` / `list` / 标量）只接到了 `load_json` 与
  `load_jsonc`，于是自称同一家族的这个加载器对每份文档都要付完整的 `CustomNode` 往返：同样的
  123 B 字节上 `load_json` 是 0.473 µs，它是 2.063 µs。JSON5 是严格 JSON 的超集，所以快速路径
  接受的文档本身就是合法的 JSON5 文档、解析出的值也相同；方言额外加的一切——单引号、无引号键、
  `.5`、`0x…`、`+7`、`Infinity`、注释、尾逗号——都会让整份文档退出快速路径，落到拥有这些语义的
  AST 路径。值没有变，这一点的裁决者是 `test_json.py`、`test_json5_spec_table.py`、
  `test_json_crosslib.py`、`test_key_resolution_parity.py`、`test_property_dialects.py` 里
  712 条对照 `json.loads` 与 `pyjson5` 的测试。在同一进程里跟同一加载器自己的方言路径相比——
  这是 Python 能同时看见两条路径的唯一办法——快速路径在 300 项文档上快 6.25 倍、在 1,200 项上快
  9.34 倍，两者都是 5/5 配对；这个比值现在由 `tests/test_json_leaderboard.py` 把关，因为这个
  加载器的两条路径都没有指令地板给它定价——CodSpeed 用 `--exclude pyrs-yaml` 构建 workspace，
  而 Ir 通道的绑定 harness 只给一条路留了地板，即 `bench_to_python`，YAML 的 AST→Python 遍历。
  手工关掉探测后两侧都停在 187.8 µs，门禁逐对报出
  并变红。已公布的 JSON5 行用的是方言形状文档，因此它仍走 AST 路径、仍慢于 `pyjson5`；那道缺口
  需要一个懂 JSON5 的直接构造器，不是这次改动能填的。
- **出口转义扫描改为一次 16 字节** — 读路径先向量化，因为它的份额更大；写路径要等到长值形状
  有了常驻地板（#600）才轮到，而"这个值需要转义吗？"这一问测得占 `to_json_long_strings` 的
  75.12% 与 `to_toml_long_strings` 的 69.01%。`write_json_string` 与 `write_single_quoted` 现在从
  一趟 SSE2 上取第一个需要决定的字节——停止集合是 {定界符、`\`、`b < 0x20`}，用 const 泛型让
  严格 JSON、JSONC 与 JSON5 各自单态化自己的定界符——然后在字符循环原样接手之前，用一次
  `push_str` 复制干净前缀。`write_basic_body` 原先问 `any(forbidden_single_line)`，答"是"之后把
  整个值重走一遍；现在它从 {`"`、`\`、C0、DEL} 取位置，其中 DEL 必须有自己的一致性掩码，因为
  抓住 C0 区间的饱和减法看不见它。在执法镜像上、由本改动携带的刷新派发包测得（三条采样取最大）：
  58,550,735 → 22,038,711（−62.36%）、72,438,675 → 28,254,711（−60.99%）、60,056,647 →
  23,544,666（−60.80%）、60,480,707 → 23,968,721（−60.37%）。同一会话里只换分发开关的墙钟配对
  是 TOML 1.199 µs → 399.9 ns、JSONC 943.5 ns → 431.1 ns、JSON5 987.2 ns → 474.9 ns；`to_json`
  的配对在计时器 100 ns 精度下分不开，所以它的凭据是地板而不是时钟。四条新测试对跨越向量
  边界的语料在每个偏移上询问每个字节值，并在长度 0、1、15、16、17、31、33、256 上比对输出：
  把 TOML 的 DEL 掩码从 `0x7f` 改成 `0x7e` 会让两条一起变红；把向量编译掉，187 条单元测试在
  aarch64 wheel 所走的标量路径上照样全绿。短值是另一种形状，那里不宣称收益——同一个扫描在
  那类场景只占 25.95-26.61%——而镜像上越过 0.5% 的只有一条：`to_json_medium` 升到
  21,492,584，+0.52%，也就是对一个 21 字节的值问一个 16 字节的问题的代价；点名它写进那次刷新，
  其余任何上移照旧自我拒绝。`to_jsonc_medium`（+0.48%）与 `to_json5_medium`（+0.47%）留在线内，
  `to_toml_medium` 降了 0.84%，而在开发主机上读到 +0.55% 的 YAML 场景 `parse_inline_merge` 在
  镜像上是 +0.00%——这正是移动要由那张镜像裁决的理由。YAML 写入器未动，它的取答已经是一趟字节
  判定；那里只剩 `serialize_block_scalars` 落在 `std` pattern 机制里的换行搜索。
- **字符串正文与 TOML 行改为一次扫 16 字节** — JSON 与 TOML 的读路径此前逐字节询问"是收尾引号、
  转义，还是本方言拒绝的字节？"，而在字符串很长的文档上，这一询问占到整体解析的四分之一到
  五分之二。TOML 还会在每次调用时完整走一遍源码以拒绝裸回车，任何超过一兆的输入都要为它付出约
  五分之一的时间，且与字符串长短无关。两处搜索现在跑在 SSE2 上——它是 x86_64 的基线指令集，因此
  没有新增依赖、没有运行时探测、也不依赖 `std`——逐字节循环保留为向量的尾段，也是其他架构上的
  完整路径。开发机实测（release 构建，11 次交错取中位）：2 MB、每串 5000 字节的 JSON 由
  2.47 → 1.97 ms，同样记录改成 6 MB 中日韩文本为 6.17 → 5.27，JSON5 单引号 1.53 → 1.11，
  2 MB 的 TOML 基础字符串 2.36 → 0.99，字面量字符串 2.41 → 0.96，多行字符串 3.37 → 1.01，
  0.45 MB、每值 100 字节的配置 2.98 → 2.54。每个停止集都由差分测试钉住：对一个含全部字节取值的
  语料，逐一遍历每个起始偏移，并与就地重述的方言规则比对；把向量路径整个编译掉之后，同样的 200
  条测试仍然通过——那正是 aarch64 wheel 走的路径。指令数门禁在已提交的夹具上给同一改动定价，
  结果是下降而非勉强持平，而且量在执法它的那张镜像上：`from_toml_medium` −4.02%
  （125,580,997 → 120,534,055）、`from_jsonc_medium` −3.69%、`from_json_medium` −3.55%、
  `from_json5_medium` −1.09%。已经没有地板高过它现在描述的那棵树——刷新派发把 26 个数字都记到了
  那里，那是它的职责，不该用本地 `--update` 抹平。YAML 没有属于我们
  的搜索可向量化：词法器归 granit，我们这一侧在长标量上是标量拷贝（占份额的三分之二，下限就是
  整份源码拷一次），在短标量上是逐节点构造（50,000 个节点时 39 ms 里有 24 ms）。
- **`load_toml` 不再构建它留不住的树** — 这个加载器对外宣称走 JSON 同胞那条免 AST 路线，此前
  并没有走：TOML 文本依次经过扫描、中间表树、`CustomNode`、Python，于是一个普通 dict 替一份只为
  保格式编辑而存在的节点模型付了钱。`pyrs_toml::read_toml` 现在把 IR 读给调用方的 `TomlSink`，
  绑定层的 sink 经 AST 与事件路线早已共用的同一个 `scalar_projection` 构建 `dict` / `list` /
  标量——说明 `0b1101`、多行字符串与日期各意味着什么的函数只有一个，第三个读者无从改写成别的
  答案。闸门是标签注册表：已注册的处理器改写带标签标量所在节点上的文本，而只有节点模型才有
  节点，所以只要装着任何处理器，文档就走 AST 路线。以同一轮里的 `rtoml` 为参照度量——两次快照
  之间主机漂移约 20%，绝对读数因此不算证据——已发布的那一行从落后 1.92 与 1.91 倍变成落后 1.47
  与 1.46 倍，两个夹具上都约快 1.31 倍；与闸门所选路线在同进程里配对比较，读作 1.15-1.34 与
  1.31-1.35 倍。被度量并否决的替代方案是事件流：它同样到达那个 dict，也花同样的钱（文档带注释时
  更慢，去掉位置与注释后打平），所以这个读者不携带注释、偏移与锚点。
  `crates/pyrs-toml/tests/value_reader.rs` 把树与新读者都压成事件对比已在使用的 token 列表，要求
  两者在种子语料与两种方言上相等，并写明 16 份文档的下限，还要求被拒绝的文档到达不了任何值；
  `tests/test_toml.py` 的 `TestValueReader` 把八份文档与中枢自身的导出再导入相比，用一个只有 AST
  路线能改动的值证明那道闸门。这一行剩下的部分是中间树，它仍在。
- **JSON 加载器的快路径如今有了自己的指令地板** — `(ft)` 给 `load_toml` 的路线钉了地板，也写下它没关
  的那一环：`load_json`、`load_jsonc` 与 `load_json5` 都先问 `json_fast::try_load`，而两个 harness
  都没进过这条路线。引擎的 `from_json_*` 停在快读者跳过的节点模型，绑定通道只测 `bench_to_python`
  与 `bench_load_toml`，所以在 JSON 快路径里每键多分配一次不会动任何已提交的数字。现在
  `bench_load_json` 端到端驱动 `load_json`，`load_json_medium` 与 `load_json_long_strings` 用引擎
  读的同一批字节（`MEDIUM_JSON`、`long_string_json()`）钉下地板：强制镜像测得 57,286,388 与
  95,502,189 条计数指令，是 `from_json_medium` 与 `from_json_long_strings` 的 0.74 倍与 0.86 倍，
  方向与 TOML 的 1.20 倍相反，因为这条路线是替掉那棵树与 YAML 文本，而不是在其上叠加。`load_jsonc`
  与 `load_json5` 有意共用这一行：快读者接受的字节上三者走完全相同的代码，差别只在引擎已经定价的
  回退路径。前提是逐文档而非逐进程的——`try_load` 遇到任何非规范写法都会整篇放弃——所以 harness 在
  计数循环之前用样例自己的字节问一次 `json_route_is_fast`，宁可 exit 4 也不以这个名次去量回退路
  径；把该 guard 强制为 `false` 后 `load_json_medium` 返回 4，而 `load_toml_medium` 仍然通过。场
  景集合变大也会牵动兄弟项，于是刷新就在本 head 上跑：`to_python_anchors` 读到 +0.1343%，
  `load_toml_long_strings` 读到 -0.2175%，都在 0.5% 线内，因此这次刷新没有点名任何场景。
- **编辑层现在有指令地板了** —— `AGENTS.md` 把 parse、edit、AST serialize 列为三类昂贵的纯 Rust 计
  算，而门禁此前只给其中两类定价：没有任何场景进入 `set_path`、`insert_path` 或 `delete_path`，于是
  一次编辑多做的工作不会挪动任何提交的数字。三条场景加入，每个动词一条，跑在读路径已在用的
  `MEDIUM_YAML` 上；同一棵树改两次不是同一次编辑，所以克隆放在计量区间之内，`--setup-only` 把那次克
  隆连同解析一起减掉，剩下的就是动词本身；`move_path` 不必单列，它的两半已被定价。命名走 gate-only，
  没有借用 divan 的 `edit_set_medium`：它替换的是整个 `database` 小节，与这里定价的嵌套键不是同一次
  编辑，同名会让两份不同工作的报告读成同一个度量。前置守卫有两道，且都观察到会咬：harness 在
  `missing-path` 上以 3 退出，测试期的钉住让同一场景以同一理由失败。扩场景集的代价由刷新量出而非预测
  ——diff 里没有一行引擎代码，六条 JSON 读路径却在执法镜像上移动 +0.73%…+1.91%；把它们写进
  `accept_drift` 点名，`tolerance_hint` 保持 0.5%。记为 `(gf)`。
- **事件流的取消半边有了数，是整趟 walk 的四分之一** — `parse_stream(..., on_event=...)` 能在文档
  中途停下生产者，但"停"这件事从没被定过价：JSON 生产者任何形状都没有地板（`stream_medium` 与
  `stream_long_strings` 走的是 YAML 经 granit 那条），所以改动结束遍历的那次判断、退出时丢弃的状态、
  或为一个已经拒绝的消费者造出的事件，都能在门禁全绿下落地。在 `MEDIUM_JSON` 上成对加两条
  `stream_json_medium` 与 `stream_json_stop`；写入文件那次运行读到 50,237,295 对 12,603,257 ——
  停只花整趟的 **25.09 %**，这就是 `Break` 结束的是扫描、不只是投递的证据。该文档发射 49 个事件，
  所以停在第十二个；上限在 setup 里由流长导出，不写死；两条前提守卫都对着变异看到咬下：要整条流
  就 exit 3（"so a quarter asks for 49"），让 sink 晚一个事件才 `Break` 也 exit 3（"the sink saw 13
  events where the stop asks for 12 of 49"）。扩充场景集的代价仍由刷新量定而非预测：第一次派发因一条
  向上的移动拒写 —— `load_json_long_strings` +0.76 %，而那次跑的绑定层二进制摘要正是已提交文件记录的
  同一个，所以动的是 runner 不是代码 —— 把它写进 `accept_drift`，`tolerance_hint` 保持 0.5 %。TOML
  有意不给停止场景：它的 `Break` 结束的是发射不是扫描，那个数只会把中间表示计价在一个承诺取消的名字
  下面。记为 `(gi)`，连同 `(gh)`：harness 曾把一个地板抄进注释（`parse_medium` 写成 "321,836,091"，
  提交值是 321,580,801），而刷新只重写 JSON、从不碰注释，所以这些引用现在只说关系，数字留给
  `.ci/ir-baseline.json`。

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
