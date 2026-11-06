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

- **编辑那几页点名了三种根本不会发生的拒绝** — 穿过别名的一次写入曾被写成会抛 `cannot-edit-alias`，
  而 `set()` 曾被写成会回 `anchor-is-referenced`；在发布的构建上实测，`set`、`delete` 与 `rename` 都
  会改写 `use: *x`，而没有任何一条 `set` 路由能到达 `cannot-edit-alias` — 那个令牌住在 delete、
  rename、move 与元数据这几条路径上。别名的*父节点*仍被拒绝，别名节点上的那十二个元数据动词也一样。
  `api/exceptions.md` 现在列出十五个可达的原因令牌与应答每个令牌的那次调用，而 `validate()` 的两项依
  赖写在使用者会碰到它们的位置：本项目发布的任何东西都不安装 `jsonschema`，而作为文本传入的 schema
  先由 `json.loads` 读取，因此一个 YAML 拼法的 schema 抛 `json.decoder.JSONDecodeError`。
- **文档开头那行标签头部之下的注释，在第一轮输出就定下来** — 搭不上自己那一行的注释要
  写在读者所报告的那个头部之上；可找头部的走查问的是头部行*之前*的换行，所以头部正好是
  文档第一行时根本不抬升，注释就留在缝隙里。在那个位置重读会把它归给根节点，下一轮输出
  便把它挪到头部之上。`!base:` 领一个不缩进的序列时，实测第一轮是
  `!base: ` / `# fuzz 47` / `- etss: &a0 !s: ~  # fuzz 47`，第二轮把注释行放到最前。
  crash-92f3fc65 与 crash-e392c402（81 与 82 字节）以及 29 字节的最小复现，现在第一轮
  输出就是不动点；`fuzz/seeds/edit_yaml` 47 条全部干净回放，`cargo nextest run --all`
  895 条通过。同批第三条是另一种落点——行尾注释被读者归到上一层节点——仍留在 issue #732。
- **从嵌套 `?` 标记抬上来的注释，不再落进头部的缝隙** — `hoist_marker_note` 把搭在内层
  标记上的注释提到那个标记自己的层级，读者报告的就是那个位置；可抬上来的那一行随后是
  接在写器末尾写下的。当那个标记正处在一对 `&a !tag:` 领头的值里，末尾就是头部行下面
  那道缝隙；granit 把一个节点的范围一直算到结束那一行的换行，于是重读把注释归给上一层，第二
  轮输出又把它挪到头部之上。如今这次写也用 `note_line_above` 已经在用的那条缝隙规则，
  并且同样先闭合挂起的纯标签行。逐一回放 issue #732 与 #733 点名的每个输入：17 条
  `yaml_roundtrip` 输入从 `serialization not idempotent` 变成不动点（最小的一条 44 字节），
  受审的那一批从 36 条失败降到 19 条；`yaml_roundtrip` 128 条种子全部干净回放，
  `edit_yaml` 47 条，`cargo nextest run --all` 896 条通过。其余 19 条留在那些 issue 上。
- **块标量的 chomping 指示符现在能扛住它上面的注释行** — 正文为空的块标量，是被拿它前面那一行
  来读的：搜索标头时从正文回退到最近一条非空行，结果撞上注释、没找到标头，于是答成 Clip 并丢掉了
  作者写的 `+`。这份文档在第一轮就不是固定点 — `# Irrr` 配上 `>+4` 先发射 `>+`、再发射 `>` — 指示符
  从引擎写出的文本里消失了。标头扫描现在改为回落到 sigil 自己那一行，而且只在上面的回退找不到标头时
  才用，所以只是以 `>` 开头的正文行仍走原来的路径。实测：引出该问题的两条 fuzz 输入
  （`crash-aa8a0447`、`crash-95b66309`）已离开失败集合，`yaml_roundtrip` 从 9 降到 7，而 `edit_yaml`
  仍为 14；两条都已归档为种子 — 语料 284 到 286，`seed_reach` 依旧在那四个往返目标的 242 个种子上
  报出 11 条不可达（这道门现在走查六个目标，报出 259 条里的 12 条）。

#### 变更

- **文档工具链升到其范围允许的最新 `zensical`** — `zensical` 0.0.72 → 0.0.74，随之带来 `zensical-ui`
  0.0.41，`zensical>=0.0.69` 约束未变。绿色的 `Docs` 运行不构成关于某个页面的证据：被锁的 0.0.56 会
  发布 front matter 不是映射的页面，而 0.0.69 直接让构建失败，所以这次升级由
  `scripts/check_doc_metadata.py`（165 个页面的元数据站点生成器读得懂）与四 locale 的 strict 构建判
  定。

#### 内部

- **`edit_yaml` 数一个名字由几对回答，而不是问它死没死** — `rename` 和 `move` 动的都是查找在身份那一
  趟解析出的那一对；文档把同一个键文本拼两遍时，兄弟那一对经查找的拼写回落趟继续回答这个名字，所以
  “旧名字已经没了”并不是这份输出作出的断言。这就是 2026-11-02 那批 `edit_yaml` 剩下三条输入所在的同
  一类（issue #732），`!base k: 1` 挨着 `k: 2` 用 19 字节就能复现。这一层现在向
  `duplicate_fold_count` —— `on_duplicate` 方针本来就在用的兄弟计数器 —— 要求每个动词留下的名字恰好
  少一对，而 `move` 自己陈述这条断言，不再沿用 `delete` 的 `all` 保证。改问“一对也没动”会让 19 字节
  和 26 字节两条输入一起变红；真正变红的是语料里 45 个 `edit_yaml` 种子中的 15 个，所以这个计数扛住
  的范围远宽于为它写下的两条最小输入。引擎的答案由
  `rename_moves_one_pair_of_a_name_the_document_spells_twice` 与旁边那条 `move` 钉住。三条输入与两条
  最小输入都进了那个语料，它在 `-runs=0` 下干净回放，矩阵读到 907 条 Rust `#[test]`、242 个种子。

- **被目标拒绝的种子现在被计数，不再被算作保护** — 六个 fuzz 目标都以“类型化拒绝即通过”开场
  （`yaml_roundtrip` 与 `edit_yaml` 看 `parse(Schema::Core)`，`json_roundtrip` 与 `parse_json` 看三
  种 JSON 方言，`toml_roundtrip` 与 `parse_toml` 看 TOML 的两道门），所以一份读者拒绝的归档输入在
  `-runs=0` 下干净回放，而它当初被归档时所要防的那条写侧承诺根本没被问过——语料总数却依旧把这个文件算
  在内。`crates/pyrs-yaml-core/tests/seed_reach.rs` 逐目标询问它自己的引擎调用，只要
  `.ci/seed-reach.txt` 与实测在任一方向不一致就变红；在这六个目录的 259 个种子上它量出 12 条不可达，
  而 `scripts/quality_matrix.py` 为清单点名的每个目标导出一个 `seed-unreachable:<target>` id——现在是
  五条，五条都已登记并写明各自移除它的条件。`parse_json` 与 `parse_toml` 是在本条写下之后才加入这个
  总体的：`parse_json.rs:36` 与 `parse_toml.rs:57` 遇到拒绝就 `continue`，全部断言都在它们身后，这正
  是四个往返目标的形状，而不是那句排除理由所说的“它们断言的就是拒绝本身”——同一句话还把出路说错了：把
  一份被拒绝的种子挪进 `fuzz/seeds/parse_toml/` 只是把盲区搬走，并没有关掉它。这些输入留在语料里：12
  条中有 5 条答的是 `max depth exceeded (max=1000)`，那正是 PR #694 自己的后置条件，所以它们防的是那
  道拒绝，其余一概不算。登记表里已经有一条被重新挖出去了：`shape-literal-and-folded` 当初写的是
  `l = |`，两道门都判它非法；改写成多行字面量字符串与基本字符串并排之后，它被读侧接受，在 `-runs=0`
  下干净回放，并抵达写侧——所以 `seed-unreachable:toml_roundtrip` 现在三条里只点名两条（账本 (jo)）。
  另有两条也按同样步骤出去了。`cr-only-break-note-ownership` 被拒的原因是 `- :` 空键，不是 CR 换行；
  `special-keys` 被拒的原因是裸 `@literal`。各自重挖——前者改成跨 lone CR 让 note 落在两个条目之间的
  序列，后者改成同样四种标量拼法、把保留字用引号包住——两条现在都抵达断言，并在 `-runs=0` 下干净回
  放。裸 `@` 开头的平文拼法从合法 YAML 里根本读不出来，那一半仍由直接构造节点的 Rust 测试守住。今日
  读数是 259 条里的 12 条（账本 (jp)）。`edit_yaml` 这一条是有意比它自己的目标少问一道前置条件：该目
  标还要一个 NUL 分隔符、至少一条完整的两字节记录、非空的路径清单，以及一个答 ok 的动词。这道缺口是
  量出来的，不是想当然：把目标插桩成每到前置条件就 panic，再让每条种子在自己的进程里回放，它的 62 条
  种子全部抵达了某条断言，所以今天这个弱一点的谓词和真谓词说的是同一件事
  （`docs/dev/quality-ledger.md` (jm)）。

- **fuzz 找到、而什么都没留下的 23 条输入现已进入语料** — 今天回放干净的 2026-11-02 与
  2026-11-03 那批发现（15 条 `edit_yaml`、8 条 `yaml_roundtrip`，45 到 234 字节）从未被归档，
  而一份不存在的种子什么都保护不了：矩阵按文件计数，所以那道缺口是看不见，而不是没被测出。每条
  都按原字节登记到 `fuzz/seeds/<target>/former-crash-<short>.seed`，再向它自己目标的引擎重新发问
  ——`seed_reach` 报出 259 条走查里 12 条不可达，而这 23 条都不在其中，所以它们都抵达了归档它的那
  条断言，而不是
  靠一次拒绝通过；回放层跑满 198 条中的 198 条，产物为零。仍在失败的 23 条继续挂在各自 issue 上，
  其中包括需要注释归属裁定的 `edit_yaml` 容器 note 形态。

- **导出路径里数字原文拼写的界限现在有了具名** — 抵达 `to_json5` 或 `to_toml` 的树在第一轮保住
  原文、第二轮却规范化：`k: 0Xeeeeeeeeeeeeeeeeeeeeeeee4e6`（26 位十六进制，core schema 把它判为
  *字符串*）被导出成裸的 `{"k":0Xeeeeeeeeeeeeeeeeeeeeeeee4e6}`，于是导出改变了它的类型；而
  `k: +83e6` 先写 `k = +83e6`、再写 `k = 83e6`。`crates/pyrs-yaml-core/tests/exports_settle.rs`
  把两个形状钉成当前状态，因为任一侧写器都无法自己回答这问题：实测在同一棵树上，`from_json5` 与
  `parse` 对一个 JSON5 数字和一个只是长得像数字的 YAML 字符串，交回的都是同一个
  `Scalar { style: Plain, tag: None, fidelity: None }`。把原文放进节点的 `fidelity` 槽、总是规范化，
  还是做成按次参数，是尚未拍板的决定，连同实测记在 `docs/dev/quality-ledger.md` (je)。

- **合并链的注释槽冲突已由具名测试钉住** — 合并源里只含另一个 `<<` 的 `<<`，读法取决于外层注释是否贴
  着那一行：`<<: #a` 配 `  <<: ":!"` 得出一层，`# a` 摆在 `<<:` 上方得出两层，而这是同一棵树。两种写
  法都一轮沉降，所以沉降预言机与属性层都看不见它。
  `test_note_slot_currently_changes_merge_consumption` 记下当前的两个答案，并写明规则选定后该收敛哪
  一条断言；这里没有改变任何行为。两条既有规则在此相互冲突 ——`collect_inline_pairs` 会把源里的合并键
  一并交出，而 v0.16.0 的"无贡献 ⇒ 普通键"规则把它们保留，`is_merge_key` 又称注释只是元数据、不是身
  份——所以哪种读法正确仍是待决问题，实测记在 `docs/dev/quality-ledger.md` (jg)。

- **写入端的保留首字符规则现在被端到端地问，而不再只问判定** —
  `serializer.rs::leading_indicator` 回答一段文本能否起头成 plain token，而
  `needs_double_quoted_agrees_with_the_scan_for_every_code_point` 已拿一份对这 17 个字符的独立重述
  逐码位钉住了该判定。没人问过它的后果：以一个这类字符开头的标量，写出去之后要在键侧和值侧都按原文
  读回来，并且一轮就收敛——那正是 `fuzz/seeds/yaml_roundtrip/special-keys.seed` 过去替我们守着的
  主张；由于其中裸写的 `@literal` 让整份文档成了语法拒绝，那条种子已不再抵达断言
  （`.ci/seed-reach.txt` 点了它的名）。先实测再落笔：17 个字符 × 两侧 = 34 次往返，全部保住原文。
  `a_plain_scalar_leading_with_a_reserved_indicator_survives_the_round_trip` 把它落成了测试：它在
  内存里构造节点，而不是解析带引号的源文，因为 `DoubleQuoted` 标量只能证明写器守得住别人递给它的
  引号。突变定价：从规则中逐个删去 `@`、`*`、`|`、`>` 与反引号，这条测试都变红并报出发射的文本；
  删去 `?` 不变红，因为 `?x` 对我们自己的读者就是合法的 plain 标量——那一成员是为与其他解析器互通
  而存在，并非我们的往返契约，这就是这条防线的边界。实测：`--test settles_in_one_round` 29 passed
  （此前 28），突变脚本末行读作 `restored -> GREEN`，且 `byte-identical after restore: True`。

- **只含注释的 JSON 文档现在由三种方言各自具名拒绝** — 钉住这条的那份语料输入
  `fuzz/seeds/json_roundtrip/shape-comment-only-document.seed`，既被它自己的目标跳过（没有方言接受时
  `json_roundtrip` 会 `continue`），也被逐条重问种子的 JSON 语料门跳过，于是规则无人断言，语料总数却
  依旧把这个文件算在内。把当时登记为不可达的 15 条逐个问“那一类由谁防守”，只剩这一条是裸的：5 条深度
  门输入由 PR #694 落下的带类型 `MaxDepthExceeded` 拒绝断言，CR 换行与保留首字符两类由
  `settles_in_one_round.rs` 里具名的测试断言，TOML 多行字符串由读写两侧四条测试断言，BOM 边界则两个
  方向都有断言。`comment_only_documents_are_refused_by_every_dialect` 把主张配成对：
  `// nothing here`、`// c`、`/* x */` 以及 `# hash` 再接 `// c` 必须被 JSON、JSONC、JSON5 三者拒
  绝，而同样注释放在值之前必须在两种带注释的方言里读出、在严格 JSON 里仍被拒绝（实测：12 次拒绝、6
  次接受）。其余 12 条保留登记：它们记的是语料信用的丢失，不是防守的丢失。

- **文档换行器的中日韩安全性现在用出厂页面来量，而不只用注入样本** — `--fix` 把正文重排到 100
  显示列，保护排版的规则此前只在 `tests/test_doc_wrapping_gate.py` 里由人工注入触发，而真正出厂的
  一万一千行没人同样问过。前几次会话里流传的那句"`--fix` 会把 Hangul 粘在一起、会把片假名拆开，
  所以本地化页面只能手工折行"，追到底是一份临时脚本里的注释，而不是工具本身：`join_lines` 对每个
  边界问"这里空一格算不算残渣"，只在不算的地方留空格；`_is_hangul` 的存在就是为了让韩语只在词间
  断开。`test_reflowing_the_shipped_localized_pages_changes_only_line_breaks` 把三份本地化变更日志
  的真实段落接成超长行，交给 `fix()` 实际写盘所用的函数，然后守住五条不变量：去空白后正文不变、
  反引号数量不变、不切开韩语词、一轮之后按检查器自己累加的计数残渣为零、再来一轮什么都不动。
  三页全过；把 `strip_residue` 变成空操作会让其中两页变红，放开 Hangul 断点会让第三页变红，脚本
  结束后按字节还原。测量本身踩掉两个坑：检查器按**行**报一条并在内里带空格数，所以比 `len()` 量的
  是行数而非排版。

- **散文宽度门现在看得见它过去豁免的那些行** — 旧的豁免问的是"有没有某次合法换行能让*第一段*落进
  100 个显示列"，所以一个以超宽代码段或链接开头的行只能答否，后面跟着多少正文都一样；而
  `_wrap_line` 会把整段余文塞进同一行，然后当作已经处理完。在这棵树上实测：十一个受管页面里有七页
  共 18 条超限行从未被报告，最宽的是账本里一段 1,410 列、有 169 个合法换行点的段落，`--fix` 原样
  退回。现在折行器先吐出那个不可拆的词元，再折它后面的内容；检查器只豁免*整行就是一个原子词元*的
  情形，所以两半依旧相互一致——被报告的行，必然是 `--fix` 能缩短的行。重排把账本从 13,024 行变为
  13,083 行、变更日志动了一行，并以"去空白后正文一致、反引号数/标题数/表格行数一致"证明只动了断行；
  仍以豁免留下的有 19 行，宽 101 到 153 列，每行都是一个词元。

### [v0.19.0] — 2026-11-04

<details markdown="1">
<summary>在调用点命名的策略 · 重复键 · 注释落点 · 按执法镜像键控的地板</summary>

测量历史 — 哪一次运行拒绝了什么、哪一处归因后来被撤回 — 住在
[`docs/dev/quality-ledger.md`](https://github.com/759401524/pyrs-yaml/blob/main/docs/dev/quality-ledger.md)。
这些说明只写调用者这边的变化。

#### 新增

- **`duplicate_keys=` 在调用点命名读者的重复键策略** — 一个开关，今天按身份给出四种答案：`a: 1` 挨着
  `a: 2` 抛 `YamlDuplicateKeyError`，`: 1` 挨着 `: 2` 折成后一对，`0set: 1` 挨着 `!base 0set: 2` 两
  对都保住、只有 `to_dict()` 拒绝，`'a': 1` 挨着 `a: 2` 抛错。`as_is`（默认）*就是*这些答案；`error`
  连空键与合并键一并拒绝，`last_wins` 正是 `allow_duplicate_keys=True` 一直以来的含义，而
  `first_wins` 保留先出现的那一对，并把被折叠那对的注释与锚点搬回幸存者身上。同时传入两种写法会被拒
  绝，一个无法识别的取值会说出它想指的四个值，每个读取入口点与十二个 CLI 命令都收
  `--duplicate-keys`。fuzz 目标按默认值解析，所以那三个具名的替代值还没进那一层。
- **`note_placement=` 与 `on_duplicate=` 把剩下的歧义交到调用点** — 在一对 `&a` / `!tag` 标头之下，
  写器曾把一个键对的行尾注释停在标头的缝隙里，而读者把它归档在上方；`note_first`（默认）按读取给出的
  位置写，所以第一次发射就是不动点，`header_first` 保留旧落点并注明它不安定。两个键也能拼出同一个名
  字，只靠一个标记或锚点区分，于是删掉其中一个后，重读时另一个仍应答那个名字；
  `delete(path, on_duplicate=...)` 答 `one`（默认）、`all` 或 `error`（`duplicate-key-target`，树不
  动），而 `delete_path_with_policy` 重跑锚点守卫，所以一个别名还在指着的定义是被拒绝，而不是悬空发
  出。
- **`load_json_lines(text)` 一次一行地读 NDJSON** — 一个每次 `__next__` 解出一个文档的迭代器，所以
  10,000 条记录的流整条读作 3.861 ms、读到首条记录 0.343 ms，对照逐行 `load_json` 的 8.440 ms；空
  行、只有注释的行与续行都以点名该行的方式抛出。可选的 `parallel` feature 给每个分片自己的汇与线程：
  200,000 行走一个汇是 112.3 ms、走 24 个是 18.9 ms（5.96x）。`parse_file` 增加一条可选的 `mmap` 路
  径，省掉一次整文件分配，默认关闭。
- **流在消费者说停时停下，而每个事件点名它的字节** — `parse_stream(..., on_event=...)` 原先把一切都
  收集完才第一次回调，所以回 `False` 仍要付整次扫描的代价；`EventSink` 现在返回 `ControlFlow`。
  `False` 停住的是一个格式事实：JSON/JSON5 在下一个事件的字节之前停住，TOML 停止发出（于是一个被拒的
  文档仍先抛），YAML 的适配器在不可中断的解析之后停止交付，于是 `load_stream` 仍是 YAML 的增量表面。
  每个 `StreamEvent` 都带 `offset`/`end`，事件字典也得到这两个键。dumper 接受加载器交回的那个元组
  （`safe_dump(safe_load("? [a, b]\n"))` 曾在被接受的输入上抛错）；一个嵌套的键序列仍抛
  `unhashable type: 'list'`。
- **测试层得到原先是暗的那几副面孔** — 跨格式的注释桥（`json -> jsonc` 与另外四对单向格式对从未被走
  过）、一层针对 JSON Schema 表面在 YAML 类型键上的属性测试、跨三种方言的 JSON 树等值，还会点名旧生
  成器到不了的那些家族的 Python 生成叶子（`bytes`、日期、`Decimal`、无界整数），以及一条拒绝任何没有
  调用点的公开可调用对象的 API 调用表面门禁 — 它发现 `get_plugin` 与 `YamlDocument.flush_source` 从
  未被调用。fuzz 语料也做了内务：未带种子的 artifact 归档为 `former-crash-*.seed`，每个目标带着它的
  发现开一张 issue，而每目标的种子计数被发布并被门禁管住。

#### 修复

- **一个别名唯一的定义被一次折叠丢掉时，现在在引用够得着的地方补一个** — 重复键折叠丢掉了定义 `x` 的
  那一对，而活下来的 `&x` 骑在一个落在别名之后的键上，于是 `*x` 出现在所有 `&x` 之上，而读者以自己的
  发射回一个 `found unknown anchor` 拒绝它。修复现在问的是这个名字是否在这次引用*之前*被定义，合并那
  一遍内联它删掉的幸存者，而 `delete_path` 也读键节点上的锚点。一个从别名改写而来的键在它落地的地方
  折叠，而以引号结尾的锚点名保住那个名字，不再在重读时打开一个被引节点。
- **五种往返形状在第一次发射就到达不动点** — 第一个条目在 `?` 标记体内的注释不再爬到标记之上，一个块
  序列的第一项保住它自己横杠之下的前置栈，一次标记提升不再跨过一层它不该跨的标头，一个注释不再于一个
  只带标记的闭合把它下方的行拉长时把一个值劈成两行，而删掉一个容器的最后一对也不再把自己的注释拖到键
  值对行上。每一个都曾是一条 `yaml_roundtrip` 或 `edit_yaml` 的 crash 输入；这些输入已归档为种子。
- **只有节点元数据能区分的键被拒绝** — 读者把带标记的键当作整节点比较，而写器正是拿样式与锚点做选
  择，于是它自己的发射回来是 `duplicate key`。键现在按标记与值跟踪，也就是 YAML 的标量等值，一次合并
  问同一个问题；两个被写器拼成一样的容器键会问写器一个容器键拼成什么，两个读回来相等就拒绝那份源。投
  影按键的文本取键，所以 `c: 0` 挨着 `!A c: "!"` 发出的条目被 TOML 读者拒绝、并被 `json.loads` 折成
  一项：两个写器现在都抛 `YamlSerializeError`，而 `to_dict`/`safe_load` 抛点名那段文本的
  `YamlTypeError`。`to_yaml()` 不变。
- **重复的 JSON 键把注释留在它替换掉的那个值内部** — 重复键折叠保住第一个位置与最后一个值，却只带着
  被丢掉的键与值节点上的注释，于是一个被丢容器*内部*的注释随之消失，而事件流仍在报告它。
  `subtree_notes` 现在按源次序收集一个节点及其之下的一切，而一个带注释的键夹在几个重复之间时留下最后
  一个值、不是倒数第二个。一个元素的两组注释落进同一个槽位而不是互相覆盖，而根的行尾注释由每一个入口
  点写出：`{"a": 1} // after` 不再往返成 `{"a": 1}`。
- **JSON5 引起一个只是看着像数字的字符串，而它的拒绝点名它自己的语法** — 裸令牌测试在自己的数字检查
  之前剥掉一个前导正负号，于是 `+-33337333333373333333733333` 被当成裸令牌发出，而这台引擎的读者拒绝
  它；只有落在一个无符号主体之上的前导 `+` 才是 JSON5 拼法，于是 `+7` 与 `+1.5` 保住它们的裸令牌，而
  这样的字符串被引起来。在键位置上，消息要求一个被引的键，而 JSON5 也接受一个裸标识符；措辞现在来自
  方言自己的那个标志，而严格 JSON 与 JSONC 保住它们自己的。
- **TOML 里那些没有槽位可住的注释现在被读了** — `to_toml("sec: # note\n  x: 1\n")` 返回
  `[sec]\nx = 1\n`：小节那一遍只读映射自己的行内槽位，而中枢把键行注释交在键节点上。同样没人读的还有
  五个成员注释槽位，而一个元素注释被写了两次。每一对都要付：执法镜像上 `to_toml_medium` +1.58 %、
  `to_toml_long_strings` +1.25 %。
- **一条点号路径像其它下降一样被限界** — `parse_inline_table` 把成员键路径铺在 `parse_value` 之外，
  而那里正是 `DEFAULT_MAX_DEPTH` 起作用的地方，于是 1,059 段被 `from_toml` 接受、被 1.1 的读者拒绝，
  一条长路径还能溢出原生栈（一个 2 MiB 线程上的 `0xC00000FD`）。两条下降现在共用一份预算：1,000 段可
  往返并定居，1,001 段是一个带类型的 `max depth exceeded`。
- **一个注册的处理器拥有它声称的那个标量** — 两个写器都先向数组路由与数字协议要答案、再问注册表，所
  以带 `numpy.ndarray` 的处理器输了，而一个只带 `__bool__` 的类写在 AST 路由发出的那一行之外；那项检
  查当初只为浮点形状写，于是一个带 `__index__` 的 `python_type` 被绕过，`Money(500)` dump 成一个裸整
  数。处理器查找现在在每一个位置都排在最前；一个 `int` 或 `int` 的子类保住整数拼法，而一个 `str` 处
  理器仍然不动文本。
- **转储一个 NumPy 数组不再终结进程（Python 3.15）** — 数组路由经 `rust-numpy` 强转那个对象并读它的
  内存，而 `cp315-abi3t` wheel 无法可靠地做这件事：`safe_dump(numpy.arange(8))` 以 segmentation
  fault（退出 139）死掉。这条路由现在向数组自己要它的 `dtype`、`shape` 与 `tobytes()`，并在 Rust 里
  解码那个缓冲区，于是没有任何制品触及 numpy 的 C API。两个缺陷随之倒下：一个转置视图写出未转置的网
  格，而数字协议对一个单元素数组丢掉维度答出来，于是一台安装写 `1.5`、另一台写 `- 1.5`。
- **一个注释不再折断它自己的行，而一个块标量能拼出它携带的那个断行** — 生成器只从可打印 ASCII 里抽注
  释文本，所以没有任何属性测试能到达一个单行注释载体欠下的那个展平；连断行一起抽就让四条属性判红，因
  为两个写器会把断行之后的文本放进文档。两个写器现在都把断行展平成一格空格，而一个携带断行的块标量把
  它引起来，不再丢成 `~`。
- **一行文档的流式重新回到线性时间** — `LineTable::line_col` 为每一个事件从行首重数一遍字符，所以一
  份没有换行的源为每个事件付 O(n)：一个 1.5 MB 压缩后的 JSON 数组为了报出它 224,006 个事件花掉 4.9
  s。它现在用 19 ms 流完。JSON 与 TOML 共用这张表；YAML 自己算位置，从来没有这个代价。
- **那些能在什么都没测的情况下报成功的门禁与守卫** — 六个 checker 脚本在一份空通配、一个读不了的路径
  清单，或一个匹配不到任何东西的 `--only` 名字上打印 OK，这六个现在都退出 2；十条原本在空可迭代对象
  上就通过的套件断言各得到一条写明实测数量的地板。一条经 `importorskip` 要 `psutil` 的内存上限，而没
  有任何需求文件写过它 — 每条腿都报一个看着像覆盖的跳过 — 现在是一个 stdlib `tracemalloc` 的标度不变
  量（2,000 行与 200,000 行都追踪到 3,168 字节，对照一条缓冲式行走的 199,854,440）。`yaml_edit` 排行
  榜那一行怪罪「较新的发布」，而钉住的构建在三个夹具上全都抛：装的是 0.2.0，而那一行由它量到的东西把
  关。
- **被报告过的 fuzz 发现不再消失** — 下载原先把归档的两个根目录直接解进 crash 目录，于是造出两个名叫
  `artifacts` 与 `collected` 的幽灵目标，一个归档的三份输入被开成两张 issue；计数用的是
  `-eo pipefail` 下的 `grep -c .`，于是一个空归档在该步结束、开单从不发生；而 panic 签名停在第一个冒
  号，这就是连续五次定时运行回来全红却没有一样可分诊的原因。目标现在是任一根之后的那一段，零是一个可
  报告的计数，而签名保住位置与消息的第一行。
- **已发布文档的更正** — `Node.is_valid()` 曾被说道会拒绝一个被改过的文档，可它只测文档引用；同一个
  `repr` 把一个过期节点、一条缺失路径与被字典投影拒绝的路径混为一谈，这些都读得穿 `to_dict()`。
  `rumdl fmt` 对 MD038 的一次「修复」删掉了代码跨度*内部*的空格，在钩子打印 `Passed` 的同时悄悄改写
  了五个镜像与台账，`MD038` 现已关闭。本地化的 changelog 页面把自己的小节放在版本标题自身的层级上，
  于是站点把每一个都渲成它所在版本的兄弟，而镜像检查现在也读这一点。

#### 变更

- **向只读单文档的路由要一个流现在被拒绝** — `parse`、`parse_file`、`safe_load`、`to_toml` 与
  `YAML()` 的等价物读一个文档，而两份文档的输入回的是行走最后停在哪一个：`a: 1\n---\nb: 2\n` 回来是
  `b: 2`。它们现在抛一个以 `multi-document-input` 开头、带着第二个文档位置的 `YamlParseError`，并指
  向 `parse_all_docs`、`safe_loads` 与 `parse_stream`。PyYAML 与 ruamel.yaml 在同样十种形状上抛，却
  仍接受一个前导 `---`、一个尾随 `...`、一个裸 `---` 或空输入，而守卫不可在这些上过度触发。CLI 里
  `-A/--all-docs` 说的是同一件事。
- **指令计数门禁现在在每个计数区触及 CPython 的运行镜像上各带一条地板** — 同一棵树、harness 摘要相
  同，`load_toml_medium` 读到 142,467,241 与 144,904,012，一个 1.71 % 的差是一条地板兜不住的，所以
  `.ci/ir-baseline.json` 得到 `image_floors`，而 `scripts/ir_gate.py` 按运行所在镜像的那一行判定这次
  运行；没有对应行的镜像以退出 2 被拒绝，而不是被猜。harness 变大时会移动这次变更从未进入的地板，所
  以越过容差的名字由一次刷新运行的拒绝点名而不是预测，而 `tolerance_hint` 留在 0.5 %。
- **每个 Rust 基准都在自己的进程里跑，而两份 lockfile 都移到它们的范围允许的最新版本** —
  `codspeed-divan-compat` 5.0 → 6.0 为每个 Divan 基准 fork，于是一个基准留下的分配器状态不再染下一
  个。没有任何决定合并的东西被移动，但跨过这次 bump 的 CodSpeed 趋势不是一次测量。`similar` 3.2.0 →
  3.2.1 是八个 crate 的 MSRV 1.96 地板之下唯一剩下的 Rust 步骤，九个 Python 包动了；`maturin` 留在
  1.15.0、`pyo3` 留在 0.29 线，因为最新的 `pytest`、`hypothesis`、`pydantic` 与 `numpy` 要一个 3.8
  地板之上的解释器。
- **`no_std` 的宿主测试在 Windows 与 Ubuntu 上都跑，而单元测试那条腿跑它名字里写着的那个配置** —
  wheel 发向三个宿主，而那个作业钉在一个上。那次六 crate 的调用把它够到的包里的 `std` 统一了，所以关
  于 112 条测试在无 std 下跑过的断言其实是在 std 构建上量的；三处清单边补上，而
  `tests/test_no_std_gate.py` 现在读清单、而不是信一份 `-p` 名单。无 std 的命令在 23 个目标上跑 724
  条测试、Windows 上 0 失败。
- **bump 机器人既不递出地板的那台编译器，也不拼错自己的配置** — 每周的 `github-actions` 组把
  `dtolnay/rust-toolchain@1.96` 改写成 `@1.120`，而那个 ref 正是 `msrv-check` 安装的编译器；这个名字
  现在被排除，而三个把 `ignore`/`allow` 的词表用在 `update-types`（它只收短形式）上的 `groups` 条目
  已改正并由一条测试钉住。

#### 性能

- **写器的输出不再为中间分配付费** — 每个写器都从 `String::new()` 起，而那七次涨到 512 字节的重新分
  配就是一次配置大小写入的大半。在门禁钉住的 `rustc 1.97.1` 上：`to_json_medium` 22,884,635 →
  17,718,056（−22.6 %）、`to_toml_notes` 16,468,905 → 12,071,715（−26.7 %）、`serialize_block`
  14,518,712 → 10,899,301（−24.9 %）、`serialize_note_flow` 64,411,669 → 59,227,943（−8.1 %）— 十六
  个写器场景全降，最小的是 `to_toml_long_strings` −5.2 % — 而未变的树把 `to_json_medium` 的地板读到
  +0.002 %。一趟注释展平让 `to_jsonc_notes` 与 `to_json5_notes` 比它们那个 PR 提交的地板低 38.27 %
  与 38.11 %。没有一个输出字节改变。
- **字符串正文、TOML 的那一行与转义扫描在 SSE2 上一次十六字节** — SSE2 在 x86_64 的基线里：没有新依
  赖，没有 `std`。在宿主上，2 MB 带 5000 字节字符串的 JSON 2.47 → 1.97 ms，2 MB TOML 基本串 2.36 →
  0.99 ms、字面串 2.41 → 0.96、多行 3.37 → 1.01；执法镜像上 `from_toml_medium` −4.02 %、
  `from_jsonc_medium` −3.69 %、`from_json_medium` −3.55 %。那个转义问题量得 `to_json_long_strings`
  的 75.12 %，而四条长值写器的地板读作 −62.36 % 到 −60.37 %；短值是另一种形状（大约 26 %），有一个付
  了钱：`to_json_medium` +0.52 %。
- **一行 JSON 一次八个字节地建索引** — `pyrs_event::LineTable` 一步一步各数一个 UTF-8 字符，而在执法
  镜像上那趟行走读掉 `documents_long_strings` 的 51.79 %。一个 `u64` 一次答八个字节，而每一条候选道
  都要与它声称的那个字节核对。`documents_long_strings` 328,990,934 → 147,188,912（−55.26 %）与
  `documents_short_strings` 95,854,968 → 85,300,932（−11.01 %），外加 `stream_json_medium` −14.90 %
  与 `stream_json_stop` −27.17 %，因为同一个索引服务整条 JSON 流路由。位置不变。
- **`load_toml` 不再建它留不住的那棵树，而 `load_json5` 走了它的两个兄弟早就有的免 AST 路由** — TOML
  文本走扫描、表树、`CustomNode`、Python，于是一个普通 dict 为一棵保格式节点的模型付费；`read_toml`
  现在把 IR 走进调用方的 `TomlSink`，而任何处理器装着时的一次读走 AST 路由。对 `rtoml`，已发布那一行
  从落后 1.92x 与 1.91x 变成落后 1.47x 与 1.46x。`load_json5` 原先付的是整趟 `CustomNode` 往返（同一
  条 123 B 上 2.063 µs，对照 `load_json` 的 0.473 µs）；对照它自己的方言路由，它在 300 项上赢
  6.25x、在 1,200 项上 9.34x，值不变。
- **加载器路由、三个编辑动词与流停止都有了指令地板** — 以前没有 harness 进入它们：`load_toml` 读作
  `from_toml_medium` 与 `from_toml_long_strings` 的 1.20x 与 1.29x — 那个 Python 一半，此前没有任何
  计数数字描述过它；`load_json_*` 把 JSON 快路由钉在它们的节点模型孪生的 0.74x 与 0.86x 上，而
  `stream_json_medium` 对照 `stream_json_stop` 把停止放在整趟行走的 25.09 %。注释路由与元数据搬运也
  有了地板：`edit_set_metadata` 的 42,838,349 对照一份小 26 个字符的文档上 `edit_set_nested` 的
  36,483,730，那是搬运，而不是一棵更大的树。
- **门禁最宽的那个读数是解释器，而它现在被钉住了** — 三次刷新运行在同一个二进制摘要下记录绑定
  harness，并把 `load_json_long_strings` 在九次抽样之间散布 1.121 %，而同样字节上的引擎场景一致到
  0.00001 % 以内。那一项是 CPython 每进程的哈希种子：在 `PYTHONHASHSEED=0` 下七条绑定场景全部收敛到
  0.0000-0.0004 %，而在那一个 head 上跑的这一次刷新把全部 42 个场景都报在它们自身 0.0010 % 之内。
- **一个空的块体写出它声明的那个 chomping** — 映射等值读 `chomping`，所以读者把
  `? hF&a?k !tag o: |+1` 与 `? hF&a?k !tag o: |` 各留一份；写器对每一个空体强制 `Clip`、两者都拼成
  `|`，而它自己的读者把这次发射拒绝成 `DuplicateKey`。`effective_chomping` 现在对一个空值给出声明的
  chomping，于是只差在那里的两个容器键仍是两个键。

</details>

### [v0.18.3] — 2026-10-22

<details markdown="1">
<summary>1.1 的数字家族 · `%YAML` 协商 · 时间戳标签 · 不带 std 的引擎 · 写入侧的外来读取器 ·
`pyq` 应答 jq 与 yq</summary>

#### 新增

- **`pyrs-yaml-core` 无需标准库也能构建** — 解析器、往返序列化器、保留注释的编辑层和 splice
  只依赖 `core` + `alloc`，其下的 `granit-parser` 本来就是不带 std 构建的。`std` 默认仍然开启，
  因此绑定、CLI 与各基准测试保持原有的 API，而 `ci.yml` 为该 crate 构建 `thumbv7em-none-eabi`
  目标、同时在宿主上运行它的单元测试；`std` 加上的是三面需要操作系统的表面：翻译过的错误消息
  目录、运行时 schema 注册表与基于 regex 的规则语言，而一个 `no_std` 调用方改为直接构造
  `pyrs_schema::types::Schema` 变体来选择 schema。引擎把已经见过的标量键记在它自己的有序节点
  映射里，因为 `alloc` 不带哈希器，这在 `parse_medium` 上代价是 74 次分配、而 std 构建是 67 次，
  每个映射多一次，而 `tests/alloc_gate.rs` 按 feature 集分别钉住一个上限。
- **`pyq` 应答 `jq` 与 `yq` 的命令行** — 凡一个 argv 不是 pyq 自己的子命令，现在就按 jq 1.8 或
  mikefarah yq v4 解读（由被调用的名字决定，也可以用 `--as jq|yq` 强制指定），再翻译成等价的
  子命令。被接受的写法在真实工具位于 `PATH` 上时逐字节拿去与它们判定 — 共 24 条调用，含 `.a.b`、
  `.list[] | select(.x > 1)`、`del(.a.n)`、`.a.n = 9`、`-o json`、`-r`、`-N`，还有一个拒绝把本
  工作区内任何文件当参考的探针。凡壳无法用同一份文档答出的，一律点名拒绝并以退出码 2 结束：jq 的
  函数（`keys`、`length`、`map`、`to_entries`）、jq 的编辑表达式、`--arg`/`-s`/`-c`/`-S`/
  `--stream`，以及 yq 的
  `-n`/`-e`/`-P`/`--split-exp`/`--front-matter`/`--xml-*`/`--lua-*`。`pyq get --no-doc` 是探针
  找出的唯一一处行为差异，因为 yq v4 打印多项流时不带 `---` 分隔符，而那现在就是方言的默认。
- **每种格式的写入侧都由一个不归本仓库所有的读取器判定** —
  `tests/test_json_writer_suite.py` 把 1,500 棵生成的树经 `to_json`、`to_jsonc` 与 `to_json5`
  写出，并要求 Python 的 `json` 与 `pyjson5` 交还写进去的那棵树；`tests/test_yaml_writer_suite.py`
  对 `safe_dump`、`from_json` 与编辑层做同样的事，裁判是 PyYAML 与 ruamel.yaml（每条路径 400 棵
  树）；`tests/test_toml_writer_suite.py` 把 `to_toml` 的输出交给 `tomlkit` 与 `tomllib` —
  1,200 棵树里有 1,074 棵以 0 次拒绝、0 次值变化写出，而从未到达陌生读取器的那 126 棵只对应一种
  带类型的拒绝，`toml-cannot-represent-null`。只被我们自己的读取器满足的不动点并不是不动点，而
  它前面那三个修复中有两个就属于这一类。
- **`Document.validate()` 以官方语料测量** —
  `tests/test_document_validate_suite.py` 回放提交 `0b4a7fa` 处的 JSON-Schema-Test-Suite（46 个
  文件、1,302 个用例），并在 `jsonschema` 4.26.0 下到达其中 1,261 个用例预期的结论。
  `refRemote.json` 整个文件按契约边界排除（接口不暴露解析器），另有十个 `dynamicRef`/
  `vocabulary`/`\p{...}` 用例被点名为被委托引擎的边界；同一语料直接经 `jsonschema.validate()`
  回放，在全部 1,302 个用例上结论一致，因此没有任何排除可归因于转换层。排除集按已安装的版本键控，
  且两侧都有断言 — 一个被点名的用例开始达成一致会让这次运行变红，一个未被点名的用例停止一致同样
  如此。
- **编辑层有了自己的 fuzz 目标** — `fuzz_targets/edit_yaml.rs` 是工作区的第九个目标，也是第一个
  驱动一次*修改*而非一份文档的目标：它读取 `document NUL script`，选取读取器真正找到的路径，并
  要求每一次被接受的编辑都能重新解析、把自己写入的值留在它声称写入的位置。它第一次种子回放就到达了
  上面那个锚点缺陷。
- **第二份登记表收纳矩阵无法测量的残留** — `.ci/known-residuals.json` 为每一处刻意保持开放的
  行为记下复现用的节点 id、归属层与一个可检查的退出条件；复现带着 `@pytest.mark.known_residual`，
  而 `tests/test_known_residuals.py` 双向失败，所以修好任何一条都会让它的特征化测试变红，并迫使
  同一次改动把这条登记项删出去。

#### 变更

- **声明过的 `%YAML` 版本按需生效** — 向 `parse`、`parse_file`、`parse_all_docs`、`safe_load`、
  `safe_loads` 或 `YAML(...)` 传入 `honor_version_directive=True`，就能让一个 `%YAML 1.1` 声明
  选定 1.1 档位（`flag: yes` 读作 `True`、`012` 读作 `10`），而 `1.2`、`1.3` 与一份未声明的文档
  仍按 Core 解析，一个显式的 `schema` 优先于该声明。标志开启时，被采信的声明会在它自己的 `---`
  之前重新发射为 `%YAML <version>` — 编辑之后与 `reparse()` 之后同样如此 — 所以一个 Core 读取器
  不会悄然重判这段文本的类型。默认值没有改动，而 `schema` 现在的默认是 `None`，好让「未给出」与
  「显式 Core」可以区分。
- **`schema="1.1"` 档位解析 1.1 所定义的各个数字家族，而写入器给它定型的文本加上引号** —
  `1_000` 是整数 1000、`0b101` 是 5、`+0x1f` 是 31、`+.inf` 是无穷而 `017` 是 15，每一个都是
  PyYAML 的加载器对同一文本给出的回答。加引号的守卫此前只问过本引擎自己的 Core 读取器，于是
  `safe_dump({"v": "1_000"})` 写出 `v: 1_000`，而 PyYAML 与 ruamel 都交回整数 1000；`8_0`、
  `0b101`、`+0x1f`、遗留布尔词 `yes`/`on`/`off`/`y`、只有日期的时间戳 `1990-06-23`，以及
  `0x_1f` 这类下划线紧跟进制前缀的写法，都是同一个缺陷在每条路径上的 26 种拼法。只有那些拼法的
  输出会变：`1_000` 现在写作 `"1_000"`。时间戳与六十进制仍是文本，而 `0o17` 在这里仍被判型、
  PyYAML 却把它读作文本，因为该档位是 Core *plus*。
- **TOML 整数保留它被写下时的拼法** — 解析器在记录之前清洗了 token，于是
  `to_toml(from_toml("a = 1_000"))` 回来成了 `a = 1000`，尽管下划线与显式正号都是合法的 TOML。
  token 文本现在整体保留，而投影只在一个拼法被 YAML Core 解析出不同取值时给它加标签：`0x1f` 与
  `+5` 保持无标签，`1_000`、`0b1101_0110` 与 `-1_000` 携带 `!!int`。取值不变并与 `tomllib`
  一致；一个无标签的带引号 `"42"` 仍是字符串。
- **事件流里的注释会说明它属于哪个节点** — `StreamEventType::Comment` 原本只带一个同时回答两个
  问题的标志位：这条注释是否独占一行（源码的事实），以及它被归档在哪个节点上（树的事实）。
  `owns_line` 保留前一个答案，而新增的 `NoteOwner` 承载后一个（`FollowingLeading`、
  `FollowingInline`、`PrecedingLeading`、`PrecedingInline`）。`parse_stream` 的 Python 调用方看到
  旧的 `style` 键保持不变，另外多出一个 `owner` 键。两处可见的细节来自生产者们从自己归档该注释的
  那个节点出发回答这个问题：写在逗号之后的 TOML 注释现在报告 `style: "inline"`，而此前报告
  `standalone`；而 TOML 表的注释现在排在它的键事件之后，而不是之前。
- **时间值以所有读取器都认识的标签离开** — `safe_dump()` 此前写出 `!date` 与 `!timestamp`，那是
  本库之外没有构造函数的本地标签：在主干上实测，`yaml.safe_load` 与 `ruamel.yaml` 对这样的文档都
  回答 `ConstructorError`，所以值不是被改了类型，而是根本读不回来。`date` 与 `datetime` 现在以
  `!!timestamp` 离开，那两个读取器会按本读取器所做的同一条分叉把它解析成 `date` 或 `datetime`，
  而 TOML 的投影也跟着它们一起移动（在 `parse_stream` 里可见为 `tag: "!!timestamp"`）。旧的标签
  保持注册并继续解析，所以更早的版本写出的文档仍然读得回来，而带着其中一个标签读入的文档也原样再
  写出。`!time` 仍是一个本地扩展：YAML 没有为一个裸的时刻定义标签。
- **标量路径上的 `sort_keys` 是有文档说明的空操作，而不是静默的成功** — `sort_keys("$.a")` 回答
  `None` 且不改动任何东西，而 `sort_keys("$.missing")` 抛出 `YamlEditError`，包所交付的任何东西
  都没有说过这件事。桩文件现在写明了它，而该残留从 `.ci/known-residuals.json` 移出，两侧都有
  断言。
- **站点上的每一项性能数字都来自一次已提交的实测** — 四种语言的页面上的墙钟表此前靠手工誊抄，而
  同一批调用现在的测量比它们背后的 2026-08-17 数字快 22–108×。
  `scripts/bench_snapshot.py --update` 对九个每批至少 0.1 秒的批次计时，并把读数连同它的解释器、
  库版本与夹具大小提交到 `.ci/perf-snapshot.json`；`--docs` 用它重写全部十二张表，而
  `tests/test_perf_docs.py` 把每一格对照回来。散文此前是单独漂移的，所以承载加速区间的 24 个
  页面上、每一段以散文形式引用的速度区间都由同一个门禁推导。页面现在写作：对 PyYAML 解析
  29–34×、序列化 223–368×，对 ruamel.yaml 68–81× 与 541–876×，往返指南为 45–59× / 102–125×。
- **指令数门禁成为一次合并判定，并且读两侧** — `Instruction-count baseline` 作业原本住在
  `codspeed.yml`，而 `Test matrix (all legs)` 只汇总 `ci.yml` 的作业，于是一次实测出的回退只是
  一份报告而非一次拒绝；该作业已经迁走，而结论现在等待它。门禁原本只读上侧，所以每次引擎变快都会
  让被容许的余量变宽 — 2026-10-14 那次刷新发现六个写手地板比它们自己的树低 22–46 %。一个场景低于
  它的地板超过 2 % 现在会警告 `FLOOR_STALE`，超过 5 % 则失败，并点名那次刷新派发作业而不是
  `--update`，后者会把地板搬到本地主机上。
- **防线矩阵改为派生，而它的外部权威列测量的是一个调用点** — `scripts/quality_matrix.py` 从声明
  防线的文件算出 `QUALITY_MATRIX.md` 的格式表，并新增一个 `external authority` 列，而
  `--write-rows` 现在由那次同时检查层级计数的测量把层级计数写下来（那处缺口让 PR #549 赔上了
  全部 14 条测试腿）。与推导对账之后，`schema validation` 是被低报的，而属性层是 27 个而不是 25
  个。这一列不再为一次 import 记账：一个 `test_*_suite.py` 只有真的伸进那个读取器才挣得读取器的
  记账，一个语料套件只有点出自己的语料才算，而一个 `partial` 的对拍格现在是一个已登记的洞、不再
  只是脚注，因为「JSON 家族有跨库层」这句话当时可以被引用到一条模块什么都不导入、只是跳过、却
  报告绿的腿上。
- **跨库参考读取器现在覆盖本包承诺的每一条腿** — 3.8 地板与 3.14+ 上，JSON5 与 JSONC 曾拿什么都
  不比较而那条腿报告绿，而 TOML 的三方一致在 3.8 到 3.10 上跳过，因为 `tomllib` 随 3.11 才到。
  纯 Python 的 `json5`（`>= 0.17.3`、`py3-none-any`）与 `tomli>=2.0` 回移包都声明在 test 组里，
  于是全部 24 条 TOML 比较和该方言的规范表在每个地方都运行；标准库 `json` 在装不上读取器的那些腿
  上充当一个 RFC 8259 的神谕（实测：29 条比较运行，而此前运行 0 条）。两个 JSON5 参考实现意见不
  一致的地方，本引擎的解读是被钉住而不是被藏起来：代理对在这里合成一个字符，而落单的代理在这里被
  拒绝。
- **MSRV 地板与 wheel 矩阵现在说出它们实际交付的东西** — `msrv-check` 安装的是 `@stable` 并把
  那称作最低版本，所以一个需要 1.97 的依赖会通过那个作业、却在用户的安装上失败；它现在钉住八个
  crate 所声明的 1.96，而在改钉之前，地板本身是在 `cargo +1.96.0 check --workspace --locked` 与
  `... test --workspace --locked` 上实测的。aarch64 的 glibc 与 musllinux wheel 腿在原生 arm64
  runner 上构建，s390x 成为唯一仍在模拟环境下运行套件的 wheel 目标。`.ci/requirements-test.txt` —
  Publish 唯一安装的需求文件 — 既没点名 `tomlkit` 也没点名 `json5`，而本次发布的第一次 Publish
  运行在上传任何一个 wheel 之前，就在收集阶段弄红了五条 wheel 腿。

#### 移除

- **自研的规则校验退役；`Document.validate()` 成为唯一的 schema 表面** —
  `validate_against_schema()` 与 `validate_against_registered_schema()` 被移除，schema 定义里的
  一个 `validate:` 小节也不再被读取。两者求值的是本仓库自创的一种规则语言，而
  `Document.validate()` 早已把求值交给 Python 的 `jsonschema`，所以两个校验器意味着两份错误契约、
  以及对「这份文档有效吗」的两个回答。调用方改走 `Document.validate(schema)`，它的
  `YamlValidateError` 携带参考引擎的那条消息，或者直接走 `jsonschema.validate()`。标量类型的解析
  没有被动过：`register_schema()`、`load_schema()`、`list_schemas()`、`YAML(schema=...)` 与
  `rules:` 列表仍然生效。CLI 的 `validate` 把 `--schema`/`--schema-file` 绑定到同一个 JSON Schema
  参数，并且在两者都没给出时只检查语法。`TypeSpec`、`ValidateKind`、`ValidateRule` 与
  `SchemaValidationError` 被删除，矩阵的 `schema validation` 行随之离开，而 fuzz 目标改名为
  `parse_schema`，因为它 fuzz 的是存活下来的那份语法。

#### 修复

- **八处注释落点现在在第一次发射就固定下来** — 一个块容器无法拼在自己 `&a` / `!tag` 行上方的注释
  每往返一轮就向上爬一个头部；写在头部与其正文之间的容器注释会移到那个头部之上；紧凑条目的注释落在
  它所属序列的头部之上；映射注释在嵌套序列上方时落到了某个条目的破折号上；块序列条目上的注释把整个
  映射推到各自独占的行上；一行只有标签（`- !fuzz2`、`{a: !t }`）时注释没有正文可待；而序列自身的
  注释移进了它的条目。八个里有七个曾是 `edit_yaml` 的崩溃输入，并被存成种子，好让这次修复继承那份
  输入；映射注释那一个由它的测试名钉住。任何一轮都没有丢失东西，这正是别的一个层级都看不见它们的
  原因。
- **一个空值按它所解析成的那个 null 拼写** — `k:` 与 `-` 会重读成一个拼法本身就是 `~` 的 null，
  于是第一次发射要到第二轮才固定。写手现在凡是正文会是空的地方都写出 null 的文本（嵌套值、根值、
  序列条目、flow 配对、flow 成员）。带标签的空值保持原样，因为标签压制了解析，而它的正文保留让它
  固定的 `''` 拼法；锚点不是同一个情形，所以 `- &a3` 写成 `- &a3 ~`。
- **让锚点失去引用者的编辑会被拒绝，而不是写下去** — 重命名一个锚点、在不带走其元数据的情况下替换
  它的节点、把一个定义移走，或者对一个成员带着 `&x` 的容器做 `set`，全都回答成功并发出本库自己的
  读取器所拒绝的文本（`found unknown anchor`），而每一次拒绝都回答 `anchor-is-referenced`。守卫
  本来就在引擎里；这四扇门绕过了它，所以现在这个问题在丢弃发生的地方被问出来。它故意比真正会悬空的
  别名集合更宽：一个别名绑定的是它这个名字*最近的前置*锚点，所以一次后来的 `&x` 本来能保住其引用的
  重命名也会被拒绝。多拒绝只花掉再一次编辑；少拒绝则写下一个谁都读不了的文件。
- **重复键折叠移走的锚点会被内联，而不是被抛弃** — 折叠丢掉的是*值子树*定义了 `&x` 的那个条目，
  修复遍拿到的只是被丢掉的那个条目自身的锚点，于是发出的文本点出了一个没有任何东西定义过的名字。
  折叠现在登记每一个离开这棵树的定义，所以存活下来的用法会和它的锚点一起被内联，输出也能在一轮之内
  读回来。
- **merge 不能再给同一个映射第二个 null 键** — `~`、`null`、空键与一个裸 `Null` 节点都解析为
  null，而读取器是有意折叠两个 null 键的；merge 的过滤器问的是另一个问题（无标签标量比文本，其余
  比整节点相等），于是一个 `<<` 源在映射本来就有的那个键旁边又贡献了一个 null 键，发出的文本写了
  两次 `~:`。由属性层在 20,000 例时发现。
- **JSON5 与 JSONC 接受自己语法所允许的东西** — `\xHH` 转义此前哪里都解析不了，而随包的桩文件却说
  `load_json5` "Accepts the full JSON5 grammar"（那两个数字是一次累积一个字节的，因为
  `from_str_radix` 会接受 `\x+1`；严格 JSON 与 JSONC 继续拒绝这种写法）；不带引号的键里的
  反斜杠-u 转义，以及字符串内 CRLF、LS 或 PS 之前的反斜杠，现在都能解析了；一个双引号 JSON5
  字符串携带除 CR 与 LF 之外的每个 C0 字节，而 RFC 8259 的那条规则此前被用到这个方言身上、而不是
  用在严格 JSON 身上；而一个除了 Unicode 空白（`U+2028`）什么都没有的键不再被丢成 `null`，因为
  JSON 解析器用 Unicode 规则裁剪纯标量，而加引号的规则只认识 ASCII 空格与 NBSP。全部 25 个 Unicode
  `White_Space` 码点都会在三种方言的键位置和值位置上扫一遍，而严格的加载器也被同样这些字节钉在 RFC
  上。
- **拼成长整数的 JSON 字符串在过中枢时保住自己的引号** — 对一长串放不下 `i64` 的数字，Core 回答
  `Str`，而 JSON 不给整数文本定边界，于是 `"9999999999999999999"` 被投影成一个纯标量，而
  `to_json` 把它写回成一个数字。投影现在在把一个字符串裸写出去之前，先问过 JSON 自己的数字语法。
- **目标格式拼不出来的值会被拒绝或重新拼写，绝不会被凭空发明** — 一个 TOML 没有语法可拼的 YAML
  浮点写法（`083e6` — Core 的指数形式允许前导零，TOML 的 `float-int-part` 不允许）是按取值渲染
  出来的，而不是原样透传；带符号的零写成 `0`，因为 TOML 没有负零，而 `ist = -0` 会重读成 `0`；
  一个 TOML 无法排在其配对之后的根表保住自己的位置、改为内联，而不是重排这个映射；而被清空的块
  序列保住它的破折号行。
- **省略 `extends` 的 schema 按它的文档所说的方式解析** — 默认值是 `core`，但解析器只在这个键存在
  且取值为空时才走到 Core，于是省略了它的 schema 会把每个标量都判成自己的文本，而结论的两个方向都
  在无声中翻转：`type: int` 拒绝 `port: 80`，而 `type: str` 接受它。一个本引擎找不到的基底名称
  仍然没有留下任何后备，那是关于公开错误表面的另一个决定。
- **逐字写出的 `!<tag:yaml.org,2002:X>` 也能命中以 `!!X` 注册的插件** — YAML 1.2 §6.8 说这两种
  拼法是同一个标签，而加载器对它手工点名的那五个标签本来已经两种都认；对其它任何名字，登记表是按
  渲染出的文本键控的，于是 `!<tag:yaml.org,2002:widget> 7` 会忽略一个 `!!widget` 插件、把整数
  `7` 加载进来。文档自己用的那种拼法先被试一次，所以只有未命中时才付出第二次查找，而 YAML 命名
  空间之外的 URI 保持它现有的解读。
- **`pyq` 把一个区间答成区间，并把 `--json` 两次都答成同一种样子** — `[0:2]` 掉到了键分支，于是
  `.list[0:2]` 去查一个名为 `0:2` 的键并回答 `null`；现在语法拒绝它无法寻址的 `:`，壳也点名拒绝
  这个构造，而带引号的方括号仍然是一个键。`pyq get --json` 对单条匹配用带缩进的排版、对多条匹配把
  每一条写在一行上，于是 `.servers` 与 `.servers[*]` 对这个标志的含义并不一致；现在每条匹配都得到
  同样的渲染。在 `--as jq|yq` 下，一条文档并不携带的路径打印 `null` 并以 0 退出，和两个真实工具
  一样，而原生 `pyq get` 仍然报告未命中并以 1 退出。
- **此前悄然漂移的文档已经改正** — 本地化的 API 页把 `parse_file`、`parse_all_docs`、`safe_load`
  与 `safe_loads` 记成一到两个参数，而随包的桩文件声明的是五个和六个；`honor_version_directive`
  在任何面向用户的页面上都没有出现过；而 `reparse()` 被记成一个本扩展并不使用的 `schema` 默认值；
  现在签名门禁在全部四种语言里都经 `ast` 比较两侧。`safe_dumps_async` 可以导入，却没有出现在
  `pyrs_yaml.__all__` 里。基准测试页面说 CodSpeed 以 `WallTime` 运行，而它自加入起跑的一直是
  `mode: simulation`，于是那些毫秒表格被标注为在一台机器上记录的墙钟读数、而不是 CodSpeed 的输出 —
  其中包括「small」那个 Rust 文档实为 19 B，而 Python 表格的是 95 B，所以同名的行之间并不可比。

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
<summary>严格 `load_json` 与外部值对拍 · JSONC/JSON5 贯通中枢、包与 CLI · TOML 1.1 与 toml-test 要求
的拒绝 · 往返留住的注释与拼写 · TOML/INI 进中枢 · `pyrs-yaml` 与 `pyq` · 原生引擎核 · 解释器被中止 ·
快路径</summary>

#### 新增

- **`load_json`（严格 RFC 8259）补齐 `load_*` 家族** — `pyrs_yaml.load_json(s)` 只接受严格文法，
  对 `//` 与 `/* … */` 注释、尾逗号、单引号字符串、裸 `Infinity` / `NaN` 和 `0x…` 回答带类型的
  `YamlParseError`。它的快路径与 `load_jsonc` 共用 `json_fast::try_load`，后者遇到任何非规范字节就
  退出；被它拒下的走严格 `from_json` AST 解析器，绝不走 `from_jsonc`。它从 `pyrs_yaml.__init__`
  再导出进 `__all__` 并重新生成了 `.pyi`，于是 CLI 够到的每个格式（`to/from json|jsonc|json5|toml`）
  都有了一个 `load_*` 兄弟。
- **引擎的回答现在由它不拥有的读者打分** — PyYAML 与 ruamel.yaml 在 20 个规范 YAML Core 1.2 文档
  × 5 个对拍面 − 2 个文档化分歧（重复键严格性、YAML 1.1 遗留布尔的 schema 作用域）上值对拍 = 122
  次比较；pyrs / `tomlkit` / `tomllib` 三方在 11 个规范构造的 24 次比较上一致，`>i64` 钉为严格按
  规范（TOML v1.0 §Integers：64 位带符号），`-2^63` 仍穿过 `tomlkit`；
  `pyrs_yaml.load_json(doc) == orjson.loads(doc)` 覆盖 16 个文档。这些库此前只在计时里并排过，从未
  比过值；orjson 是权威，因为标准库 `json.loads` 在 `allow_nan=True` 下接受 `NaN` / `Infinity`。
- **JSONC 与 JSON5 通到核心、包与 CLI** — `pyrs_yaml_core::json::from_jsonc` 与 `from_json5` 坐在
  `from_json_with_options(text, JsonParseOptions)` 上，`JsonParseOptions` 一次拨一个轴：
  `allow_trailing_commas`、`allow_single_quoted`、`allow_unquoted_keys`、`allow_comments`，并出厂
  `STRICT`、`JSONC`、`JSON5` 三个预设。JSONC 在空白合法之处接受 `// line` 与 `/* block */`
  （即 `tsconfig.json` / `settings.json` 那一档），其余一律不收——它是 RFC 8259 的严格超集，
  `from_json` 仍守 RFC 8259；`to_json_text` / `to_json_text_pretty` 即便 AST 带着注释也忽略它们，
  所以保留是选用式的。`pyrs_yaml.from_jsonc` / `load_jsonc` / `from_json5` / `load_json5` 与
  `YamlDocument.to_jsonc()` / `to_json5()` 都改走原生引擎而不是 `json.dumps`（此前
  `pyrs_yaml.from_jsonc(...)` 抛 `AttributeError`），`emit_root_leading` 把根的
  `leading_comment` 写在最外层 `{`/`[` 之前，`pyq from-json` 得到 `--jsonc` / `--json5`。
- **JSON5 接受 JSON 拒绝的词法与数字，并把两者都写回** — 非引号键取完整 Unicode
  `ID_Start`/`ID_Continue` 集（新依赖 `unicode-ident`）；分隔符在 RFC 8259 的四个之外加上垂直制表、
  换页、NBSP U+00A0、所有 Unicode `Zs` 空格、U+2028/U+2029 与 ZWNBSP U+FEFF；字符串加上行终止符前的
  反斜杠、双引号内的 `\'`、`\v` 与 `\0`；数字加上 `0xDECAF`、`0XFF`、`-0x1F`、`.5`、`5.`、`+7`、
  `+.25`、`07`、`Infinity`、`NaN` 与 `-Infinity`，每个都在 plain 标量上保住自己的源文本。以上都以
  JSON5 为门：严格 `from_json` 与 `from_jsonc` 逐字节不变地仍要求这些键加引号并拒掉这些字节，
  `\uXXXX` 里的孤立 UTF-16 代理项继续被拒——Rust 的 `String` 装不下它。载入时 `Schema::Json5`
  把 `0x1F` 答成 `31`、`Infinity` 答成 `float('inf')`，`to_json5_text` / `to_json5_text_pretty`
  在同一个写手 `Mode`（Json / Jsonc / Json5）下还原拼写：任何标量为
  `ScalarStyle::SingleQuoted` 就用单引号、裸数字词元、注释归一为 `//`、键恒加引号。
- **TOML 拒掉 toml-test 标为无效的文档** — 官方语料经一个类型标签适配器重放到 toml-test 的
  `{"type": ..., "value": ...}` 形式并按字节读取，此前一处 `read_text()` 在解析器看到之前把裸 CR
  改写成了 LF。数字 UTC 偏移按范围检查（时 00..=23、分 00..=59），于是 `+12:60` / `+24:00` 失败；
  C0 控制码与 DEL U+007F 在 basic、literal 与多行字符串内被拒——只有 tab，以及多行形式里的换行合法
  ——注释体里同样被拒，CRLF 对之外的裸 `0x0D` 由解析入口的一次中心扫描拦下；由点键隐式创建的表对
  后续 `[header]` 关闭，`[tbl]` 之后 `[[tbl]]` 失败，在 1.0 与 1.1 下都无效，而合法的隐式超表照旧
  解析（30 → 21）；前导零（`01`、`007`、`-01`、`01.5`）、给进制前缀整数加号（`+0x1F`、`-0b101`、
  `+0o644`——`signed-int` 只包十进制整数）以及边缘或成对下划线（`1_`、`1__0`、`0x1_`）现在都是拒绝
  （71 → 48）；内联表拒绝等于、延伸或被已定义路径遮蔽的点键（`{ a = 1, a.b = 2 }`、
  `{ a.b = 1, a = 2 }`），而 `{ a.b = 1, a.c = 2 }` 仍合法（48 → 39）。
- **TOML 1.1.0 文法，以及它的日期时间终于能解码** — `from_toml` 解析 2025-12-18 发布的 v1.1.0
  规范：内联表可以跨行并以尾逗号结束（`{\n a = 1,\n b = 2, \n}`），`\xHH` 覆盖 0x00..=0xFF，`\e`
  拼出 U+001B（ESC），秒可省略，于是 `t = 14:15` 与 `dt = 2010-02-03 14:15` 字节稳定地往返。为钉住
  1.0.0 的使用者保留 `TomlDialect::V1_0` 与 `from_toml_v1_0`，每个 1.0.0 文档在两种模式下解析完全
  一致，并修掉让 1.0 模式认不出无 `T` 日期时间的 `space_time_sep` 差一错误。纯日期
  （`1979-05-27`）与纯时间（`07:32:00`）现在带各自 `!date` / `!time` 标签（本地与带偏移的日期时间
  仍用 `!timestamp`），并路由到 `date.fromisoformat` / `time.fromisoformat`，不再在*合法* TOML 上抛
  裸 `ValueError`；`!time` 为 3.8 地板补齐省略的秒，`!timestamp` 规范大小写 `t`/`z`，两者把任意精度
  的小数秒（TOML 允许 `.6`，而 3.11 前的 `fromisoformat` 只收 3 或 6 位）归一到微秒，`...Z` 变成
  `+00:00`，因为 `datetime.fromisoformat` 从 3.11 起才认 `Z`。
- **TOML 往返留住它的注释、形状与拼写** — 解析器把键或表头之上的独立 `# ...` 与
  `key = value # ...` / `[name] # ...` 的行内注释捕获到 `NodeMeta::comment`，
  `to_toml(from_toml(src))` 原位重发，于是 `pyq edit` 与 `YamlDocument.set()` 不再剥掉注释；文档开头
  的 `# note` 也活下来，无注释文档不再多出空行。内联表内部的注释落到成员自己的槽位，未装饰的表保持
  紧凑的 `{ a = 1, b = 2 }` 一行，数组里被装饰的表提升为多行内联形式，顶层被装饰的表成为
  `[section]`，修复还暴露出 `skip_all_blank` 把独立注释自身的结束换行算成了空行。多行字符串
  （`"""…"""` 或 `'''…'''`）投影为 `ScalarStyle::Literal` 的 YAML 块标量并重发为 `"""` 块，不再是
  一行转义文本，而单行的 `"a\nb"` 仍单行；除文档第一个键之前，`to_toml` 复现源文的空行分组
  （`a = 1\n\nb = 2` 字节稳定）；十六进制（`0xDEADBEEF`）、八进制（`0o755`）与指数浮点（`1e10`、
  `-3.14e-2`、`6.02E23`）保住进制前缀，而下划线分隔符、显式 `+` 与二进制（`0b101`）规范成十进制，
  因为 YAML Core 对它们没有规则。
- **一个独立注释，无论哪个引擎写的** — `NodeMeta` 长出 `leading_comment: Option<Comment>` 与
  `blank_before: bool`（两者都排除在结构 `Hash`/`PartialEq` 之外），于是一个节标题或表数组元素既带它
  上方的注释也带 `]` 之后的行内注释，JSONC 成员同样两者都带（上方 `// ...` 进 `leading_comment`，
  `... value // trailing` 进 `comment`）——这是单槽模型表达不了的形状。granit-parser 接收器是最后一个
  从 `comment(standalone = true)` 迁到 `NodeMeta::decor.leading_comment` 的引擎，`NodeMeta::eq` /
  `Hash` 让同一概念无论落在哪个槽都等价，因此解析出的 YAML 仍与每个手搭的夹具相等。
  `CustomNode::set_leading_comment` / `remove_leading_comment` / `remove_comment` 原子地清掉两个槽，
  写手在需要时回落到 `comment` 且 `standalone = true`，于是手搭节点照旧渲染、严格 JSON 仍不发 `//`；
  `to_yaml(toml_ast)` / `to_yaml(json_ast)` 保住前导注释，Python 侧在 `Node.remove_comment()` 旁边得
  到 `Node.leading_comment`。
- **TOML 与 INI 进中枢，文档自己能写 TOML** — `from_toml(toml: str) -> str` 与
  `to_toml(yaml: str, schema=...) -> str` 在 TOML ⇄ YAML 文本之间转换，
  `load_toml(toml: str) -> dict` 通过内置 `!timestamp` 插件生成 Python 值且不重解析 TOML 字符串
  （`"true"` 值仍是字符串），`load_ini(text: str) -> dict` 经标准库 configparser 读 INI——严格、保
  大小写、只读，因为 INI 没有官方文法。TOML 输出对表达不了的形状（null 值、非表根、别名、非标量键）
  以稳定消息拒绝，注释/锚点/拼接的往返编辑按设计仍是 YAML 专属；`YamlDocument.to_toml()` 从 AST 一次
  原生调用发出，而不是 `to_toml(doc.to_yaml())`，字节相同且比 `tomli_w` 快约 4.3x。
- **可选类型、可选设置来源，以及一个漏掉的导出** — `!duration`（`pendulum.Duration`）、`!arrow`
  （`arrow.Arrow`）与 `!ulid`（`ulid.ULID`）在对应库装好时自动注册，各占一个不同标签，所以
  `!timestamp` / `!date` / `!uuid` 不受影响，而标准库 `timedelta` 永不被 `!duration` 命中。
  `PyrsYamlConfigSettingsSource` 可直接替下 `pydantic_settings.YamlConfigSettingsSource`，用
  pyrs-yaml 的 YAML 1.2 core schema 而非 PyYAML 解析，并惰性导出，因此 `import pyrs_yaml` 绝不要求
  pydantic-settings（`pip install "pyrs-yaml[settings]"`，Python 3.10+）；`dump_pydantic` 与
  `parse_as` 用同一个模块级 `__getattr__` 模式。`YamlStream`——`YAML().load_stream*()` 一直返回的
  那个类——现在可以导入了。
- **`pyrs-yaml`，命令行** — `pip install "pyrs-yaml[cli]"`（Python 3.10+）加上 18 条纯 Python 命令，
  建在 Cyclopts 上，于是基础安装保持零额外依赖与 Python 3.8 支持：`fmt`（保留注释、锚点与顺序的
  往返重排）、`get`（JSONPath，配 `--format yaml|json|text`）、`set` / `delete` / `rename`
  （`--inplace`、`--string`、`--create-missing`）、`sort-keys`、`move`（目的地必须已存在）、
  `frontmatter`（Markdown front matter，可选切出正文）、`validate`（对 CI 友好的退出码，
  `validate --schema <name>` 与 `--schema-file <path>` 互斥）、`compliance`（YAML Test Suite 报告，
  `--json`）以及 `to-json` / `from-json`，全部经 `-` 读 stdin 并默认写 stdout。
  `to-toml` / `from-toml`、`to-jsonc` / `from-jsonc` 与 `to-json5` / `from-json5` 与 `to-json` /
  `from-json` 对齐，`-A/--all-docs` 覆盖每条编辑与转换动词，其中 `move` 作用于两条路径都能解析的
  每个文档。
- **`pyq`，运行时不含 Python 的原生 Rust CLI** — `crates/pyrs-yaml-cli`（基于 clap，
  `cargo install --path crates/pyrs-yaml-cli`）把 `pyrs-yaml-core` 放到 jq/yq 风格的命令行之后：
  `fmt`、`get <path>`（JSONPath-lite——点键、`[n]`/`[-n]`、方括号键、`*`；`--json`/`--raw`）、
  `set` / `delete`（`--create-missing`、`-i/--inplace`）重新序列化以保住注释与值的风格、
  `rename` / `move` / `append` / `insert`、任意路径上的 `sort-keys`（根写 `$`）、`validate` 可只作
  语法检查或按 schema 语言规则校验（`--schema rules.yaml`）、`frontmatter --body-out`、
  `to-json` / `to-toml`、导入侧 `from-json` / `from-toml` / `from-ini`、按扩展名定格式（`--input`
  覆盖）、经 `-` 读 stdin，以及带核心稳定错误文本的非零退出。
- **`pyq` 的过滤器是旗标，不是表达式语言** — `--select 'PATH OP LITERAL'`、`--sort-by PATH` /
  `--desc`、`--unique`、`--first` / `--last`、`--skip N` / `--take N` 与 `--join SEP` 按固定次序
  `select -> sort -> unique -> slice` 再 `join` 执行；因为是旗标而非表达式，混合比较种类答 `false`
  （一处文档化的与 jq 分歧）；`pyq completion` 经 `clap_complete` 打印 bash、zsh、fish 或
  PowerShell 脚本。`-A/--all-docs` 覆盖每条编辑命令与 `to-json -A`，每个文档只对自己那段流拼接
  （`MultiDocEditor` + `DirtyUnit::shifted`），未触碰的文档与每个 `---` 分隔符保持原字节；接上它时
  暴露出 `move_path` 只返回目的地的 INSERT 单元，从而复制了被移动的子树。

#### 变更

- **granit-parser 1.1 → 1.3，以及升级悄悄付出的严格性** — 1.2.0 为怪异文档上限加了可选 `Options`
  字段，1.2.1 把若干解析结果向 YAML 规范收紧，1.3.0 加了两个带默认值的 `Input` 方法
  （`fetch_block_scalar_line`、`take_quoted_scalar_ascii_chunk`）让扫描器更快地跨过块与引号标量的
  字节；本项目经 `Parser::new_from_str` 消费解析器，只实现 `EventReceiver` /
  `SpannedEventReceiver`，从不实现 `Input`，因此无需改源码。只有一个判定移动了：1.3 开始*接受*一个
  续行缩进不超过其所属块键的多行流集合（yaml-test-suite `9C9N`：`flow: [a,` 然后 0 列的 `b,`），
  把严格性从 `405/406 → 404/406`——仍在语料 ≥95% 门限内，所以 CI 报绿。树内一处解析后防护如今拒掉
  缩进不足的续行，恢复 `405/406`，且只读解析器已经算好的跨度。
- **原生 JSON 与 TOML 引擎核；`serde_json` 与 `toml_edit` 已移除** — `pyrs-yaml-core` 出厂一个
  RFC 8259 引擎（字节级扫描器、逐字的数字拼写以保 `from_json → to_json` 字节稳定且对大整数与 double
  保精度、带行列的类型化错误，并对尾逗号、前导零、孤立代理项、未转义控制字符与多根文档给出拒绝），
  以及覆盖全文法的 TOML 1.0 引擎——裸/引号/点键，basic/literal/多行字符串及每种转义形式，十/十六/八/
  二进制整数与下划线分隔，含 `inf` / `nan` 与指数形式的浮点，以及带偏移与本地的 date、time、
  date-time——每次拒绝都以 granit-parser 同风格的 `ParseError::Syntax` 携 0 基 `line`/`col` 浮出。
  CLI、绑定与打包面未变。
- **`YamlDocument.validate()` 缓存已编译的校验器** — 对同一 schema（JSON 文本*或* dict）首次成功的
  校验保住它编译好的 `jsonschema` 校验器，后续调用因此跳过 schema 解析、元 schema 检查与校验器构造；
  dict schema 以对象身份为键，背后有一层深拷贝快照守卫，能用 `==` 察觉就地修改并透明重编译。缓存路径
  抛 `exceptions.best_match(validator.iter_errors(instance))`，与 `jsonschema.validate()` 语义相同。
  WSL 墙钟：`document_validate` −98%（660µs → 13µs）。
- **与出厂构建不一致的文档已更正** — numpy 指南的 0-D 一节声称 0-D 数组"重塑成单元素列表"
  （`assert data == [42]`），出厂行为序列化的是裸标量（`assert data == 42`），四个 locale 的
  `docs/{en,zh,ja,ko}/guides/numpy.md` 都已更正并为 0-D `bool` → `1.0` 的 rust-numpy 怪癖加了警告。
  跨 locale 页面里 `saphyr-parser` 改成 `granit-parser`，YAML 合规 98.1% 改为 99.75%（405/406 例），
  ABI3 支持 3.9–3.13 改为 3.8–3.15，基准表换上当前 CodSpeed 数字（解析比 PyYAML 快 21–43x，序列化快
  55–177x），Rust 基准小节从 Criterion 迁到 divan。

#### 修复

- **恶意文本不再中止解释器** — 定宽转义读取按字节偏移切 `&self.text[pos..pos+width]`，假设每字符
  一字节，于是一个 JSON `\u` 或 `\x`、或 TOML 的 `\xHH` / `\uXXXX` / `\UXXXXXXXX` 后面跟着多字节字符
  时，切片落在字符中间并把进程 panic 掉；读取器现在切字节缓冲再做 UTF-8 校验，畸形转义返回干净的
  解析错误，而 TOML 的 basic 与多行 basic 字符串解析器——此前按字节前进、会在 U+00A0 或 U+0251 上把
  `&str` 切在字符中间——改为整字消费。另一类中止是 `0xC00000FD` STACK_OVERFLOW：TOML 解析器没有嵌套
  预算（不像 JSON 的 `DEFAULT_MAX_DEPTH` 与 YAML `parse` 的 `max_depth`），`parse_value` →
  `parse_array` 与 `parse_inline_table` 无限递归，`load_toml("a = " + "["*5000 + "]"*5000)` 在没有
  任何 Python 异常的情况下杀掉解释器。现在一个 `depth` 计数器在超过 1000 时回答带类型的
  `ParseError::MaxDepthExceeded`，与 JSON 对齐。
- **`<<` 只在源能合并时才合并，于是字面 `<<` 键活下来** — `safe_dump({"<<": None})` → `<<: null` →
  `load` 返回 `{}` 丢了键（`<<: 1`、`<<: "x"`，或 `<<:` 与其他键并列时同样），因为解析器把每个 `<<`
  都当合并键，把 Null 或标量源消费掉却什么都没合并。按 YAML，该键只对指向映射的别名、内联映射或它们
  的序列才是合并，所以 Null 或 plain 标量的 `<<`，以及不产生任何合并的无别名 `<<`（`<<: []`、
  `<<: [1, 2]`、`<<: {}`）是普通键，现在整趟往返存活；带别名的路径则恒被消费——`<<: [*a, *a]` 无法二
  次展开。合规仍是 `405/406`。
- **锚点守卫按路径作用域，于是自引用合并终止、重复别名能解析** — 一个展开指回自身锚点的 `<<`
  （含 `b: {<<: *a}` 的 `a: &a`）在 `resolve_merge_keys` 里无限展开并杀掉进程（Windows 退出
  `0xC00000FD`），因为环守卫只在*收集*被合并的对时生效，从不覆盖*走*展开。锚点名现在在其展开被走完
  之前留在递归路径上，解析回该路径上某祖先的合并以空展开结束；又因无环 AST 装不下 PyYAML 的循环
  dict，自引用合并最终落在 `{}`。同一次修复定了另外两条语义：`<<: {x: 1}` 现在会合并，合并序列里的
  非别名元素（`<<: [*a, {y: 2}]`）保住它的内联映射。`to_dict()` 有镜像缺陷：别名在一个*全局*
  已访问锚点集之后展开，而那集从未清空，所以只有第一个引用产出值、其后的每个都退化成 `None`
  （`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}` 给了 `b` 值而 `c` 一个 `None`）；守卫现在为一次展开
  压入、之后弹出，真正的环仍终止。
- **NumPy 序列化不再在无 GIL 时读 Python 内存** — ndarray 写手经 `unsafe { as_slice() }` 借数组缓
  冲，并在 `py.detach` *内部*、即释放 GIL 之后迭代这个借用；`&[T]` 不论来源都是 `Send`，所以借用检查
  器看不见那块内存归 Python 所有、别的线程可能同时改变数组大小或写入。缓冲现在在仍持 GIL 时被快照进
  Rust 拥有的内存（`slice.to_vec()`），只有标量→节点的转换在线程外跑。这是绑定层唯一的 `unsafe` 缓冲
  借用，其余每个 `py.detach` 位点都经审计为只触碰 Rust 拥有的状态（AST、源文本、
  `BufWriter<File>`）。
- **被丢弃、错置、重复或未修剪的注释** — JSONC/JSON5 值之前的文件首 `// note` 被分类为*行内*注释
  （空白扫描的启发式里 offset 0 之前没有换行），于是被第一个对象成员认领，而不是
  `emit_root_leading` 标注过的根容器，在空 `{}` 或根标量上干脆消失；JSON/JSONC/JSON5 **与 TOML**
  写手逐字发出注释体，而每个解析器都以修剪后的形式存储，所以 `text = " "` 在多次往返之间抖动尾随
  空白；只有注释的 TOML 文档（`# note` 没有键来消费它）把空根序列化成 `""`；YAML 解析器对所有在建
  容器共用一个注释槽，于是嵌套容器开始时会冲刷掉尚未进 AST 的独立表头注释——槽处理现在是一栈一容器；
  以及一个再生文本自带该对独立注释的拼接跳过了旧注释行，让两者都留下，放宽到注释行的范围后不再如此。
  五个格式现在都到达字节稳定的前导注释不动点；手搭的非空 TOML 根同时带独立注释与首元素注释时，渲染
  成两行相邻 `#`，那是任何 TOML 文本都无法重新归属、真实解析也不会产生的形状。
- **序列化器在任何深度只发可再解析的 YAML** — 嵌套键下的 literal 或 folded 标量（`a:` 然后 `b: |` 再
  正文）把正文发在距 0 列一个固定缩进步的位置，而不是 `b: |` 表头下一格，于是每个嵌套形状序列化的文
  本要么重解析为错误要么得到错值；标量写手现在把一个 `block_base`（父行的列）穿过每个发出位点，七种
  嵌套形状全部逐字正确。五种拼写倒在同一道门下：被强制单行的映射值会预发子节点的锚点/标签，即使子节
  点（标量、null、流容器）自己写表头，于是 `A: !a` 之后二次盖章 `!a null`；流集合内的块标量（`[|`、
  `{k: >}`）或键位置的块标量降级为双引号，因为流语法容不下缩进正文；自成一行的流容器丢了缩进，而复杂
  键（`? …`）把它的值标记 `:` 发在 0 列，闭合了任何外层集合；正文带独立注释或标签的复杂键序列化出歧
  义文本（行中的 `? # c`），于是注释移到 `?` 标记之上、正文各占深一格的新行；边缘带空白的 plain 标量
  （`" %"`、`" "`）或流集合内含嵌入式流指示符（`,` `[` `]` `{` `}`）的标量被加引号。带标签的空块容器
  把表头折进 `{}` / `[]` 那一行，而紧凑的 `- key: value` 条目拒绝内联带独立注释的值。
- **TOML 接受合法的最小 i64** — `from_toml` / `load_toml` 在 `-9223372036854775808`
  （`i64::MIN`，按 TOML 定义在范围内）上失败：带符号路径剥掉 `-`，把 `9223372036854775808` 读成无
  符号 i64，在取负之前就已溢出。现在符号与数字一起解析（`i64::from_str` 以负向累加），带符号浮点取负
  时保住指数拼写，`pyrs_yaml.load_toml("a = -9223372036854775808")` 返回 `{"a": -(2**63)}`。它来自
  TOML/JSON/JSONC/JSON5 对标准库 `json` / `tomllib` / `pyjson5` 权威、按类型严格相等的属性测试，那些
  测试也钉住两处已知的 AST 歧义拼写：裸 JSON5 的 `Infinity`/`NaN` 字面量，以及超出 i64 的位串在一个
  值必须解析成的数字与作为字符串保留其文本之间无法区分。

#### 性能

- **`safe_load` 在事件流上一趟生成 Python 对象** — `safe_load`、`safe_loads` 与
  `YAML().safe_load*` 不再先构造完整 `CustomNode` AST 再在 `convert.rs` 里走第二遍；plain 标量经同一
  条 schema 链解析，映射键保住裸文本语义，重复键错误报告方式一致，而带锚点、带标签、合并与多文档形状
  留在 AST 管线上，前面是零成本的预判（一次 `&` 字节扫描、一次分隔行扫描、一次负载级 `<<` 检查）。
  WSL 墙钟：`safe_load_scalar_types` −21~25%，`safe_load` 家族 −13~18%，回落路径未变。基于事件跨度的
  锚点重放变体曾实现并实测：带锚点文档 +48%，已回退。
- **JSON 在两个方向上都留在快路径上** — `load_jsonc` 把规范严格 JSON（对象与数组、`i64` 整数、布尔、
  `null`、无转义字符串）直接解析成 Python 对象，跳过中间 `CustomNode` AST：比 AST 路线快约 5-6x 并
  领先标准库 `json.loads`，任何非规范输入（浮点、转义、注释、越界整数、尾逗号、非法词元）回落到通用
  路径，因此值与错误报告不变。浮点（与 CPython `float` 一致的正确舍入解析）与八个简单双字节转义现在
  也进这条道——日志型负载上快约 15x，含 `\u` 或任何非法转义仍走 AST 路径。
  `YamlDocument.to_json` 用原生引擎而非 `to_dict()` + `json.dumps`，ASCII 字节相同且快约 10x（1200
  项文档从约 1450µs 降到约 120µs），映射键直接进输出缓冲，不再每键经 `key_text` 造一个 `String`
  （约 60µs，又约 2x），无需转义的字符串以一次 `push_str` 成块拷贝（约 35%，41 → 27 ns/项，字节
  相同）。JSON 序列化在安装环境的排名是 #2，只逊于 orjson。一处行为变化：`to_json` 像 `to_jsonc` /
  `to_json5` 那样把非 ASCII 发成裸 UTF-8 而不再是 `\uXXXX` 转义——仍是合法 JSON，`json.loads` 解析
  结果相同。
- **容器、接收器与错误契约被单一来源化** — 映射与序列渲染共用一个 `write_container_node` 骨架（输出
  字节相同，`serialize_*` 中位数 −5~11%），解析入口共用一个 `load_ast` 错误契约，schema 链共用
  `bool_word`/`numeric_tail`，`register_anchor` 与独立-行内分类法（`is_standalone_placement`）在各
  接收器之间单一来源，路径编辑方法委托给 `apply_metadata_edit`，重复率降到 5.25% → 3.45% → 2.60%。
- **读取器跳过它们用不上的工作** — `extract_anchors` 在单次 `&` 字节包含检查后返回空，于是无锚点文档
  跳过逐字符的引号状态机（`parse_large` 31.2µs → 26.3µs，扫描 1.5µs → 38ns，Rust `parse_*` 中位数
  11–18%）；`parse_stream` / `load_stream` 每事件发出的固定键（`line`、`column`、`type`、`value`、
  `style`、`anchor`、`tag`）经 `pyo3::intern!` 复用驻留对象（WSL：`parse_stream` −34%、
  `parse_stream_multidoc` −39%、`load_stream` −22%）；`on_document_end` 移动完成的文档而不再深拷贝
  （`parse_all_docs` −9.7%、多文档 `safe_loads` −9.5%、`YAML().safe_loads` −6.7%）；
  `direct_dump_into` 把每个文档写进复用的 `String`，`dump_iterable` 在文本以单个换行结尾时跳过
  `normalize_doc` 的再拷贝（`dump_stream_multi_doc` −27.2%、`dump_stream` −4.4%）；
  `unescape_double_quoted` 对不含反斜杠的引号字符串提前返回，`detect_chomping` 惰性拉行
  （`to_dict` 家族 −4~9%、`safe_load_scalar_types[strings/numbers]` −3~4%）；序列化器的缩进缓存现在
  也服务块标量表头写入。

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
