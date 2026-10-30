# 编译器分层与实现分包

状态：设计修订 1。ARC 条款规定目标接口，以及[实施计划](implementation-plan.zh-CN.md)中的行为不变工程拆分。[项目／包语法](project-and-packages.zh-CN.md)仍属于下一语言版次草案，其中加载／导入／离线求值切片已实现。

## 1. 分开语言、构建与执行权限

**ARC-001.** 语言实现包含七个逻辑层：

| 层 | 输入 → 输出 | 所有职责 | 禁止事项 |
| --- | --- | --- | --- |
| Project loader | 显式根／manifest → 不可变源码快照 | 文件、依赖位置、字节限制、SourceId | 认证用户或激活模型 |
| 语法 | 源码单元 → 未校验 AST／span | Token、语法、源码内语法诊断 | 把 AST 当可执行结果 |
| 语义分析／链接 | AST 图 → 已校验 IR 或诊断 | 名称、类型、可见性、效果、循环、实体操作组 | 获取依赖或查询数据 |
| 制品构建 | 已校验编译结果／构建来源 → 冻结制品 | 带版本序列化、摘要、依赖闭包、能力 | 授权或加载未校验 IR |
| 纯求值 | 已校验 IR／固定上下文／记录／输入 → 候选及事件 | 确定性语义、共享预算、不变量 | 读取文件／时钟／网络／数据库 |
| 宿主适配器 | 精确活动制品／已认证命令 → 原操作结果 | Admission、当前权限、事务、结果披露 | 重解释语言规则或换新键重试 |
| CLI／工具 | 用户意图 → 调用上述接口 | 输出、退出状态、显式本地 I/O | 复制语义和权限检查 |

项目加载在编译器外进行。编译器接收不可变字节和可信构建配置，CLI 与未来编辑器因此可以共用它，不必授予环境文件系统访问。解析／检查失败不产生可执行对象，候选结果也不是已提交记录。

**ARC-002.** 物理分包按已经独立的职责划分，不是每个阶段一个 crate。当前 workspace 为六个有真实实现的包。AST、语义检查和求值器先保留在 `ti-language`，只有 resolved IR 真正具备独立公开契约后再继续拆。这使已校验模型构造器保持私有，避免为了跨 crate 而公开未校验可执行数据。

```text
ti-lang (compatibility library + ti executable shell)
  ├── ti-language
  ├── ti-artifact ───────→ ti-language
  ├── ti-project ────────→ ti-language
  ├── ti-cli ────────────→ ti-language, ti-artifact, ti-project
  └── ti-host-mensoria ──→ ti-language, ti-artifact  [optional]
```

箭头表示编译依赖。叶子包不能反向依赖 `ti-lang`，语言／制品／CLI 库不依赖 Mensoria 适配器。SurrealDB、Tokio 属于宿主包和资格化 harness，不属于语言编译器。不新增 runtime、调度器、通用插件引擎或空的宿主 trait 包。

## 2. 分包契约

**ARC-003.** `ti-language` 拥有标量／nominal 值、源码诊断、语法、语义校验、已校验模型构造、纯求值和规范 wire 编解码／分帧。其外部接口保持简洁：

```rust,ignore
compile(source: &str) -> Result<Compiled>
compile_bytes(source: &[u8]) -> Result<Compiled>
Compiled::evaluate(entity, action, actor, inputs, old, new_id) -> Result<Candidate>
Compiled::validate_record(entity, record) -> Result<()>
Compiled::module_name() -> &str
Compiled::model_version() -> i64
Compiled::ir_bytes() -> Result<Vec<u8>>
```

既有签名和 harness 编码保持兼容。AST／模型字段与构造器保持私有，`Compiled` 不能从 JSON 反序列化。元数据访问不是校验绕过入口；既有公开元数据字段仅为兼容保留，只有冻结制品可供宿主使用，后续移除需显式修订 Rust 接口。

内部 `syntax` 拥有词法／解析，`model` 拥有现有表达式／模型表示，`check` 拥有名称／类型／效果／状态校验，`evaluator` 拥有候选产生，`wire` 拥有精确编码及相等性分帧。它们是有明确职责的真实模块，不是独立 crate 的空壳。现有已校验表示仍是字符串绑定的 IR schema 0.1，不能宣称已经成为下一版 fully resolved typed IR。

**ARC-004.** `ti-artifact` 拥有 `Artifact::freeze`、manifest 序列化、不可变标识和 registry 发布。只通过已校验模型的只读接口及规范 IR 字节访问编译结果，不读取私有 AST 字段，也不通过反序列化制品构造模型。编译指纹覆盖实际语义／解析／求值／codec 源码和声明锁文件；消费方工作区的真实依赖解析仍需独立构建证据。源码移动合理地改变编译器标识。

**ARC-005.** `ti-host-mensoria` 拥有原生命令、回执、SessionBinding、角色映射、固定动作／投递事务、维护安装／激活和当前结果检查。接收连接，不为动作创建 root 连接。通过 facade 保持 `MensoriaHost::bind/execute`、投递和关闭状态激活的源码兼容。故障钩子仍只在 `host-qualification` feature 下存在，普通宿主构建不包含它们。SQL 与宿主同包并参与其源码指纹；纯求值调用不能到达宿主代码。

**ARC-006.** `ti-cli` 拥有参数分派、有界文件读取、序列化、stdout／stderr 和退出状态。`run(args)` 可由根可执行壳调用。整个 workspace 只有一个 `ti` binary target，不因 CLI 库而产生两个同名输出。首轮拆分保持 `check`、`compile`、`freeze`、`eval` 及既有错误／退出行为；项目命令只有在相应语言／project 实现完成后才能加入。

**ARC-007.** 根 `ti-lang` 是兼容 facade，重新导出既有语言类型／函数、`artifact`／`wire` 模块路径及 feature 控制的 `mensoria` 路径。现有 Mensoria 源码接入仍依赖它。Facade 不拥有解析、校验、事务或表达式逻辑；普通构建不引入原生宿主依赖。根集成测试与消费方跨越相同外部接口。

## 3. 编译器数据契约

**ARC-008.** loader 现已输出不可变 `Project` 快照（计划中 `ProjectSnapshot` 的实现）：已验证 manifest 语义、精确源码单元 `(SourceId, 相对路径, 不可变 UTF-8 字节)`、锁定依赖边及源码／图摘要。文件系统检查后快照不可变。编辑器未保存缓冲区须作为带内容摘要的显式快照替换，不能假设等同磁盘。编译器不获取可自行打开的路径或隐式环境配置。`ti-project::lock/open` 拥有 manifest 解析、Unix 描述符相对安全读取、锁比较和原子写锁；`Project::compile` 把源码快照传给 `ti-language::project::compile_packages`。导入名称降为有碰撞检查的内部 handle，并保留完整定义方符号元组。开发格式 `ti.project-ir/0.2-dev.1` 复用已校验表达式求值；完整 span、最终 typed IR ABI 和可部署全限定 ID codec 仍属于后续门槛。

**ARC-009.** 下一版语法 AST 节点携带半开 span 和未解析名称，解析成功不代表类型／效果正确。语义阶段依次收集声明、解析导入导出、分配 nominal 类型、链接操作组、检查表达式／类型／上下文、检测策略循环及限制，之后才能产生 `CheckedModel`。Resolved IR 使用稳定 symbol／type／operation handle、已校验操作数／调用类型、源码 span、确定性操作／事件顺序及带版本 intrinsic 表。只有验证构造器能签发 checked handle，公开可编辑 AST 或解码 IR 在求值／冻结前必须重新校验。

**ARC-010.** IR 序列化与源码映射是分别带版本的格式，序列化顺序按稳定符号标识确定，不按哈希表、文件枚举或分配顺序。源码字节、包图、编译配置、限制及编译器／求值器标识均参与构建标识。即使语义 IR 相同，移动源码也可改变制品。确定性构建不意味着不同编译器版本产生相同制品标识。

**ARC-011.** 语义和运行时错误使用结构化诊断，渲染归 CLI／编辑器。下一版诊断集合按 `(SourceId, byte_start, code)` 确定性排序，包含主／关联 span 和 phase。运行时公开错误还需在披露规则下包含可观察原操作引用及核对提示。拆分期间保留当前行列诊断，不声称已满足完整 span；不能把所有错误包装为泛化包错误来假装完成诊断引擎。

## 4. 执行与依赖纪律

**ARC-012.** 宿主先解析当前可信作用域／身份和原键，再选择适用的不可变制品或核对原结果，对所选动作求值，然后通过固定原生事务提交。提交和结果释放时重新检查 admission／权限。包与链接元数据不能扩大角色成员关系、数据可见性或外部副作用权限。拆分后纯求值仍保持同时赋值、候选事件读取和操作级共享预算。

**ARC-013.** 显式传入依赖，语言／制品包无需数据库即可编译与测试。任何 workspace 包都不能在编译／build script 中执行网络或数据库操作。后端下载和原生验证 harness 是显式开发／CI 任务。未来第二个真实后端需要同等宿主行为资格化，届时才抽取通用适配接口。Mensoria SessionBinding 不是可移植认证对象，不能从公开动作 JSON 构造。

**ARC-014.** 证据跟随接口变化：编译／求值／codec 检查属于语言证据，源码适配器编译属于接入构建证据，真实数据库和 TCP 故障测试属于宿主证据。移动文件改变源码和实现指纹，新报告必须枚举全部 crate 源码和 Cargo manifest，保留历史失败／成功运行，并重新核验实际 Mensoria 消费工作区。目录移动不会把旧本地结果升级为下一语言版次或生产资格化。
