# 语言项目、包与模块

状态：`package-model/0.2` 标准提案，设计修订 1。英文为本提案规范源文，中文版保留相同 PKG 编号、语法和示例。目标版次是 `ti/0.2`，profile 为 `approval-package/0.2`；仅项目入口接受这些语法，既有 `ti/0.1` 单文件入口仍拒绝它们。Manifest／lock 加载、模块导入和离线求值已实现，0.2 部署／制品协议尚未实现。既有[单源码标准](ti-language-standard.zh-CN.md)保持不变，支持范围见[实现状态](implementation.zh-CN.md)。

## 1. 概念与单位

**PKG-001.** Project 是文件系统工作区；package 是带版本的源码与依赖单位；module 是包内声明名称空间；model 是拥有稳定逻辑名和整数修订号的宿主激活单位。Rust crate 是实现单位，不能作为 Ti 导入目标。源码文件不构成部署、事务、身份或权限单位。

**PKG-002.** 每个包恰有一个 `Ti.toml`、一个 `src/` 源码根和一个根模块。`src/` 下所有普通 `.ti` 文件均属于该包，包括根模块没有导入的模块；每个文件均需校验，不能把无效或特权声明藏在未使用文件中。本 profile 拒绝嵌套 manifest、生成源码发现和条件编译。工作区可有多个包，但构建、激活均针对显式包根独立进行。

```text
leave/
  Ti.toml
  Ti.lock
  src/
    leave.ti
    leave/model.ti
    leave/policies.ti
    leave/actions.ti
    leave/events.ti
```

## 2. Manifest 与版本域

**PKG-003.** Manifest 是 UTF-8 TOML，schema 为 `ti.project/1`；未知字段、重复字段、必需项缺失及别名冲突均拒绝。不支持可执行 manifest、构建脚本或环境变量展开。首版完整形状如下：

```toml
schema = "ti.project/1"

[package]
name = "acme.leave"
version = "0.1.0"
edition = "ti/0.2"
profile = "approval-package/0.2"
root = "leave"
kind = "model"

[model]
name = "leave"
revision = 1

[dependencies]
approval_events = { package = "acme.approval_events", version = "1.2.0", path = "../approval-events" }
```

`kind` 为 `model` 或 `library`。模型包必须有 `[model]`；库包禁止该节，不能直接激活。库可以包含完整实体及纯声明，但导入不会赋予存储权限。`edition`、`profile` 必须是受支持的精确字符串，不能使用范围。`root` 是单个标识符；首版固定源码根和文件命名，不支持可配置 glob。

**PKG-004.** 包名由至少两个小写 ASCII 段以点连接，每段匹配 `[a-z][a-z0-9_]*`，最多 63 字节，总长最多 191 字节。依赖别名和模块段使用区分大小写的 Ti 标识符，最多 63 ASCII 字节，不含关键字。可移植源码路径只用小写；大小写折叠冲突即使在大小写敏感系统上也拒绝。依赖别名不能与本地根模块相同。包版本是精确稳定的 `major.minor.patch` 十进制三元组，不允许前导零、预发布或构建元数据，每段最多 2147483647；首版不支持范围或一个解析图内同一规范包的多个版本。

**PKG-005.** 语言版次、profile、包发布版本、模型修订、制品标识和运行时 incarnation 是独立域。模型修订为 1–2147483647，隶属宿主中的稳定模型名。发布包 0.2.0 不会激活模型修订 2。既有模型修订不能被不同制品覆盖，需按不可变契约提升模型修订；已发布包的同名同版本内容也不能替换。

## 3. 源码路径与名称空间

**PKG-006.** `src/leave.ti` 声明 `module leave;`，`src/leave/policies.ti` 声明 `module leave.policies;`。逻辑模块路径必须与去掉 `.ti` 的源码路径精确一致。一文件一模块，不能跨文件重新打开同一模块。源码映射使用相对路径、`/` 分隔符及 UTF-8 字节排序。拒绝符号链接、`..`、绝对源码路径、大小写冲突、包根外文件、无效 UTF-8／BOM／单独 CR。注释、CRLF 遵守基线词法；源码哈希不归一化换行。

**PKG-007.** 实体、策略、事件共享所属模块的声明名称空间，导入别名也参与其中。名称仅从显式声明和导入解析；没有全局 prelude、文件名推断、递归父作用域查找或最后声明覆盖规则。文件／声明顺序不能改变解析结果。实体 ID 和动作选择器按下文规则解析，不能在运行时拼接名称字符串。

## 4. 导入与可见性

**PKG-008.** 声明默认模块私有，`pub` 才可导出。`use` 导入显式命名的公开声明，`pub use` 同时重新导出，两者均不执行代码。拒绝通配导入、整模块导入、动态路径、源码内文件系统路径、隐式导入和宿主／原生导入。

```text
module leave;
pub use leave.model::{LeaveRequest};
pub use leave.events::{LeaveApproved};
```

```text
module leave.actions;
use leave.model::{LeaveRequest};
use leave.policies::{Reviewer as CanReview};
use leave.events::{LeaveApproved};
```

模块路径以本地根模块或 manifest 依赖别名开头。访问依赖时，别名替换其根段，例如 `approval_events.events::{DecisionRecorded}` 映射到该依赖声明的根模块加 `.events`。目标必须是实际模块和公开声明。表达式不新增全限定调用，导入后的本地别名仍使用既有表达式语法。

**PKG-009.** 不能导入或重新导出私有名称。重新导出保留定义方符号标识，不能改变 nominal type 或增加权限。拒绝模块导入循环（含重新导出循环）、名称冲突、重复别名及隐式遮蔽。公开签名必须具有完整公开类型闭包：引用的实体、nominal ID、发出的事件类型均须导出且能被消费方解析。公开实体暴露字段／状态 Schema，这是编译期可见性，不是读取存储值的权限。

**PKG-010.** 导出实体 `E` 同时以相同可见性导出 nominal `EId`；导入 `E` 不自动导入 `EId`，两者均需显式列出。生成的 ID 名称冲突即拒绝。状态符号始终隶属实体，不能独立导出。既有可空、比较和不变量上下文规则继续生效。

**PKG-011.** 动作和创建声明隶属目标实体，不是全局函数，不能作为独立函数导入、别名或重新导出。定义方包的不同模块可在显式导入实体后，为该包实体声明操作。编译器链接完整操作组，检查每个实体恰有一个创建声明、动作名称唯一、终态没有出边，以及既有初始化／不变量／事件要求。

**PKG-012.** 只有定义方包可以增加实体的创建／动作，或改变字段、状态、不变量。导入／重新导出外部实体不能附加动作、覆盖授权或重新打开 Schema。本提案不支持继承、partial entity、trait、扩展方法或策略替换。库包也须通过完整实体操作组校验。

## 5. Nominal identity 与模型组合

**PKG-013.** 符号 ID 是 `(规范包名, 精确包版本, 模块路径, 声明种类, 声明名)`，以带版本的长度分帧元组编码。实体 ID 类型引用定义方符号，而不是导入别名、源码路径或消费方拼写。一个 lock 图只允许每包一个版本。包版本变化会改变 nominal symbol ID，即使名称不变也需显式分析兼容性／迁移。内容哈希区分不可变源码制品，但不代替 nominal identity，也不认证发布者。

**PKG-014.** 模型根模块公开导出的实体构成激活集合。本包未导出的实体仍需检查，但不能作为公开动作目标。本 profile 首版禁止根模块重新导出／激活依赖拥有的实体；它们可作为 nominal Schema／ID／策略依赖被导入，但不会在消费方注册集合或可执行动作。多模型组合、实体接纳以及标量／泛型策略参数需后续提案。本标准不承诺实体形参策略可复用于无关本地实体。共享事件声明、同包多模块在不削弱 nominal typing 的情况下即可发挥作用。

**PKG-015.** 导出策略保留既有单实体形参和纯表达式限制。依赖必须无环，共享策略不能反向依赖消费方实体包而形成循环。不引入隐式类型投影、结构实体兼容或跨记录查询。所有已链接策略调用消耗同一个操作级求值预算。只有已认证宿主提供 `actor`，导入代码不能提供角色成员关系或扩大宿主能力。

## 6. 依赖解析与锁文件

**PKG-016.** 首版仅支持显式本地路径依赖，相对 `Ti.toml` 所在目录解析；依赖可在兄弟目录，但须落在调用方允许的工作区根内。拒绝规范路径解析中的符号链接穿越、网络路径、源码级文件系统穿越和 manifest 重定向。内存库调用方提供显式不可变包快照及可信依赖映射，不必授予文件系统权限。依赖解析属于 CLI／project loader，编译器自身无 I/O。CLI 默认仅允许根包目录，可重复的 `--workspace-root DIR` 显式授权更多根。从共同父目录使用示例兄弟依赖时需要 `--workspace-root .`。这是 loader 输入，不是包获得的权限。

**PKG-017.** `ti lock` 验证完整依赖图，只有成功后才以临时文件／rename 原子写入 `Ti.lock`，不执行网络获取。它是首版唯一更新依赖解析结果的命令。普通 `check`／`build`／CI 必须有锁文件，不能静默修复。Manifest 语义、源码包、别名、版本、内容摘要或边变化均使锁过期。依赖定位器移动到经过核验的相同内容，不改变锁标识。缺失／过期锁在语义编译前失败。工作区包不会因搜索路径优先级而自动受信或被选用。

**PKG-018.** `Ti.lock` 是 UTF-8 JSON，schema 为 `ti.lock/1`，拒绝重复／未知字段，保存排序后的完整包图。每个包记录包含 `name`、`version`、`edition`、`profile`、`root`、`kind`、`content_sha256`，以及从排序依赖别名到精确包名／版本／摘要的映射。包记录按 `(name, version)` 排序，每个引用对象包含 `name`、`version`、`content_sha256`，摘要为 64 位小写十六进制。`root_package` 使用相同引用形状。顶层包含 `schema`、`root_package`、`packages`、`graph_sha256`。不包含源码位置、绝对路径、时间戳、用户身份或缓存。源码位置保留在 manifest，由 loader 与锁记录交叉核对。

**PKG-019.** 包内容标识使用 SHA-256、域 `ti.package-source/1` 和既有 `F(tag, bytes)` 分帧：`F('d', domain)`、`F('m', manifest 规范语义)`，然后按排序路径逐项加入 `F('p', 相对路径) || F('s', 精确源码字节)`。Manifest 规范语义使用紧凑 UTF-8 JSON：递归按 ASCII 字段名排序、无无意义空白，含所有通过验证的语义字段，但排除依赖 `path` 定位字段；数组保序，别名为排序对象键。哈希前拒绝未知字段。图摘要使用 `F('d', "ti.package-graph/1") || F('g', root_package 与排序包记录的规范 JSON)`，排除 `graph_sha256` 自身。改变导入别名或源码路径会改变标识，移动内容相同的依赖目录不会。

**PKG-020.** 包的导入符号表绑定精确锁定的依赖图。即使根源码相同，传递依赖内容改变也必须产生不同构建／制品标识。锁和摘要检测差异，不是签名。缓存命中必须匹配工具链、edition／profile、限制、源码内容及完整依赖图，并校验缓存内容。缓存或未校验 IR 不能绕过语义检查和宿主激活。Registry、Git 源、semver 求解、签名和包发布不在首版范围内。

## 7. 构建、制品与激活

**PKG-021.** 提议流水线是 `加载 → 核对锁 → 解析所有文件 → 解析导入／导出 → 检查／链接 → 输出已校验 IR → 冻结制品`。不能按死代码消除跳过校验。制品记录根模型名／修订、包坐标、完整图摘要与依赖记录、源码映射／内容摘要、typed/resolved IR schema／版本、编译器与求值器指纹、profile／限制和必需能力。模型构建原子化：出错只产生诊断，不产生新的可运行制品。制品 JSON 不是已校验 IR 的可信构造器。

**PKG-022.** 构建不会激活。部署请求提供精确根制品和每个导出实体的显式宿主绑定。依赖模块不能自行激活，也不能获得事务、凭据、策略存储或外部副作用。既有单记录事务、revision CAS、审计、outbox、原键核对和维护激活规则继续生效。包依赖不是数据库关系，也不是分布式事务。

**PKG-023.** 版次 0.2 使用新的 artifact／IR 和宿主协议版本；请求／事件的 nominal ID 标签引用定义方 symbol ID，制品包含带版本符号表。新的精确传输编解码和 golden byte 夹具是可执行 0.2 宿主资格化的前置条件；不能把 0.1 短名称标签和指纹用于全限定标识。0.1 宿主不能通过探测 feature 加载 0.2 制品。升级后既有操作结果仍保留原制品与协议。

## 8. 限制、诊断与命令

**PKG-024.** 初始确定性上限：含根包最多 32 包，每包 16 条依赖边，依赖深度 16（根深度 1），每包 128 源码文件，每文件 256 KiB，每包源码 2 MiB，全图源码 8 MiB，每包 256 声明、全图 1024 声明。Manifest 最多 64 KiB，lock 最多 256 KiB，每包公开导出和导入绑定分别最多 256 项。基线表达式／标量／动作限制和共享 100000 节点预算不变。文件枚举发现超限后即停止，不再读取其内容。源码遍历还限制目录深度 32、每包检查目录项 4096、每次加载依赖位置解析 512 次；8 MiB 读取预算也计入经重复定位器检查的源码字节。资源变化需具名 profile 修订。

**PKG-025.** 项目诊断包含 code、phase、净化消息、主 `(SourceId, byte_start, byte_end)` span，以及可归属时的相关导入／声明 span。字节范围为 UTF-8 半开区间，显示行列由源码映射计算。Manifest／lock 使用各自 SourceId。公开输出不暴露绝对路径或凭据。稳定类别：`TI_PROJECT`、`TI_PACKAGE`、`TI_LOCK_STALE`、`TI_IMPORT`、`TI_VISIBILITY`、`TI_DEPENDENCY_CYCLE`，以及既有语法／名称／类型／模型／限制错误。失败必须定位包／模块上下文，不能开始宿主工作。

**PKG-026.** 提议命令如下：

| 命令 | 输入与结果 | 写入或权限 |
| --- | --- | --- |
| `ti init DIR --name acme.leave` | 显式新建项目骨架 | 拒绝非空 DIR，不联网 |
| `ti lock --manifest Ti.toml` | 显式路径全图 → 已验证 Ti.lock | 只原子替换锁 |
| `ti check --manifest Ti.toml` | 已锁定快照 → 诊断 | 不修锁、不写制品或 DB |
| `ti build --manifest Ti.toml --out DIR` | 已锁定快照 → 制品／源码映射 | 原子生成输出，不激活 |
| `ti inspect ARTIFACT` | 校验结构／摘要 → 可读元数据 | 不执行，不能认证未受信 IR |

`lock` 和 `check --manifest` 已实现；`compile --manifest` 检查开发 IR，`eval --manifest` 执行离线候选求值。`init`、可部署 `build` 和 `inspect` 仍是设计契约。既有 `ti check/compile/freeze/eval FILE.ti` 保持 0.1 入口，不会自动把旧模块版本转换为包发布版本或模型修订。

## Appendix A. 包语法增量

以下增量只替换 0.1 源文件头；除上文显式变化外，保留其余基线构造。`pub` 只修饰实体／事件／策略，动作和创建按 PKG-011 隶属定义方包。

```ebnf
source       = "module", module_path, ";", { item } ;
module_path  = IDENT, { ".", IDENT } ;
item         = use_decl | [ "pub" ], ( entity | policy | event )
             | creation | action ;
use_decl     = [ "pub" ], "use", module_path, "::", "{", use_item,
               { ",", use_item }, "}", ";" ;
use_item     = IDENT, [ "as", IDENT ] ;
```

## Appendix B. 审批包示例

[包示例](../examples/package-design/README.md)给出全部源码、manifest 和预期链接结果。它现已是带生成锁文件的可运行项目编译示例，不会激活数据库模型。交付门槛见[包验收](package-conformance.zh-CN.md)和[编译器分层](compiler-architecture.zh-CN.md)。
