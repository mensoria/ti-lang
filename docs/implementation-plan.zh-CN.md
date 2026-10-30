# 项目模型与编译器交付计划

状态：设计修订 1；A／B 已实施，C／D 中 manifest／lock／模块切片已实现。最终 span、可部署制品及 0.2 宿主激活仍独立推进。行为不变的 workspace 拆分与下一语言版次实现分别交付，范围由[包标准](project-and-packages.zh-CN.md)、[编译器架构](compiler-architecture.zh-CN.md)和[验收矩阵](package-conformance.zh-CN.md)定义。

## 决策

**MIG-001.** 工程拆分保留 `ti/0.1` 单源码语法，不实现半套 `use`、不从目录猜测包，也不把既有 75 个回归测试改称包系统资格化。新项目语法只属于 `ti/0.2`、`approval-package/0.2`。每阶段独立记录证据，可以单独交付，不能暗示后续阶段已经可用。

**MIG-002.** 拆分所有权如下：

| 现有实现 | 目标 | 接口约束 |
| --- | --- | --- |
| `src/lib.rs` 值／诊断／模型／检查／编译 | `crates/ti-language/src/{lib,model,check}.rs` | 已校验构造器保持私有 |
| `src/parser.rs` | `crates/ti-language/src/syntax.rs` | 保留 Token／AST 规则 |
| `src/evaluator.rs`、`src/wire.rs` | `crates/ti-language/src/` | 保留求值和编码字节 |
| `src/artifact.rs` | `crates/ti-artifact/src/lib.rs` | 使用 checked getter／IR 字节 |
| `src/mensoria*.rs`、固定 SQL | `crates/ti-host-mensoria/src/` | 不改事务文本／权限语义 |
| `src/main.rs` 分派 | `crates/ti-cli/src/lib.rs` | 根保留唯一 `ti` 可执行程序 |
| 根 library／binary | `src/{lib,main}.rs` | 仅重新导出和分派 |
| `tests/`、原生夹具、接入源码 | 保留根目录 | 消费方回归接口 |

不创建空的 lexer／type／IR／LSP crate。逻辑层先通过内部模块明确划分，只有独立消费方和表示契约需要时才进一步拆包。文件系统 project loader 现已成为第六个真实包 `ti-project`，依赖编译器，不由编译器反向调用。

## 交付门槛

**MIG-003.** A — 设计与出版：中英文 PKG／ARC／MIG／PC 编号对应，语法／示例一致，完整审批包目录，明确实现状态，导航／搜索入口，链接和双语检查。此阶段不执行包代码，也不证明新增语义。

**MIG-004.** B — workspace 拆分：创建四个实现 crate 和兼容 facade，统一 Cargo 依赖版本，保留根命令／feature，不弱化既有断言，只公开制品构建必需的已校验 IR 只读接口。验收包括 workspace 格式／lint、默认构建无宿主依赖、全部 44 编号场景、CLI 行为、原生动作／投递／故障测试和实际 Mensoria 适配器编译。检查依赖环和原生 SDK 泄漏，重建源码指纹／回执。此阶段仍不支持新语法和导入。

**MIG-005.** C — 源码基础设施与项目加载：实现 SourceId／span、manifest／lock schema、有界安全遍历、显式工作区根、快照编译、离线完整依赖图校验及原子写锁。不能联网获取依赖。先通过 PC-001–PC-008、PC-016–PC-022、PC-025、PC-027，再加入语义导入；旧编译入口和限制保持独立资格化。

**MIG-006.** D — 模块语法与语义链接：只在新版次实现 header／visibility／use、确定性 symbol ID、模块导入导出 DAG、可见性／公开闭包、nominal ID、封闭实体操作组和根激活导出。以 checked symbol／type handle 替代未解析名称。先通过 PC-009–PC-015、PC-023–PC-024、PC-026，再整体执行静态／求值包场景。任何不支持构造都须在制品生成前拒绝。

**MIG-007.** E — 确定性包制品：规定并冻结 IR／artifact／source-map／qualified-ID wire 精确格式及 golden bytes；实现当前保留的 0.2 codec 后才能声明 0.2 执行支持。加入 inspect／build、依赖闭包摘要、缓存验证和包／模型不可变发布。验证目录移动、发现顺序改变、传递依赖变动、manifest 修改、缓存损坏及伪造制品。新宿主绑定准备好之前不得激活。

**MIG-008.** F — 原生包激活：保留既有 admission 和维护所有权，把公开实体符号绑定到受管集合；用全限定类型／事件标识重新验证当前作用域／角色、历史操作保留、Schema 兼容、revision 竞争、审计／operation／outbox 原子性和通用绕写保护。验证回滚、真正响应丢失、取消及旧协议结果核对。在实际目标 Mensoria 迁移栈执行 PC-028–PC-030。编译成功不能激活包或迁移数据。

**MIG-009.** G — 工具：项目感知诊断／源码映射、编辑器快照、符号跳转、模型评审与激活 diff。SourceId／导入契约稳定后再加入真实 LSP 消费方。网络 registry、依赖求解、发布／签名、标量／泛型策略参数、跨记录执行、实体接纳、新日期／金额语义及其他数据库适配器均为后续独立提案。

## 兼容与恢复

**MIG-010.** 拆分保持相同输入下的 0.1 IR 序列化、载荷 codec、求值结果、错误码、原键行为和宿主 SQL 语义，但不承诺编译器／制品哈希不变：workspace 会改变指纹输入和 Cargo.lock。不能静默覆盖部署 head 以接受新宿主指纹。维护激活需显式评审／重新资格化新制品／修订；旧操作仍按原制品标识及当前结果权限读取。测试只使用独占原生环境，源码重组不会修改共享 DB Schema 或活动 head。

记录拆分前后源码清单、真实 Cargo 锁及原生后端版本，保留失败运行。源码回滚通过普通版本控制变更进行，不能在新规则下重新执行旧操作来“修复”失败发布。未来 0.1→0.2 升级必须有显式类型／存储／codec 迁移及双协议原结果读取，不能靠移动源码文件完成。
