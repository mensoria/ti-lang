# Rust 实现与原生宿主验证

状态：已有可运行参考实现和可选 Mensoria 事务宿主，2026-10-30。44 个编号回归场景均有可执行检查；这不等于完整语言标准、完整平台或生产认证。[验证回执](qualification/latest.json)记录精确源码哈希、数据库版本和测试结果。

## 运行语言

使用 Rust 1.96.0 和仓库 Cargo.lock。

```sh
cargo run --locked -- check examples/approval.ti
cargo run --locked -- compile examples/approval.ti
cargo run --locked -- freeze examples/approval.ti
cargo run --locked -- eval examples/approval.ti examples/create-request.json
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

`check` 校验源码；`compile` 输出可检查 IR；`freeze` 输出冻结制品清单；`eval` 只返回候选结果。离线 harness 中的 actor 是测试上下文，不是经过认证的网络身份。

冻结制品绑定源码、IR、编译器源码指纹、声明锁文件、版次／profile、依赖、确定性资源上限及宿主能力。相同模块版本不能覆盖。指纹标识源码／构建输入，不是二进制证明；消费方工作区需要另外记录实际依赖锁和构建证据。

求值器使用显式工作栈，共享 100000 节点预算。测试覆盖恰好 100000 个节点与多一个、深层无环策略、同时赋值、旧／新不变量、候选事件、静态限制及输入／事件载荷 128 KiB 边界。`wire` 实现重复键拒绝、标准标量／事件标签、规范 Int64 和附录 C 指纹。内部 harness ID 对象不能作为公开 nominal ID 输入。

## 执行全部编号场景

```sh
python3 scripts/setup_native_backend.py
TI_SURREAL_BIN="$PWD/.cache/native-backend/surreal" python3 scripts/verify_conformance.py
```

安装脚本校验固定 SHA-256。缺少原生数据库时验证失败，不会跳过 H 层。每个宿主测试启动独占的本机回环 SurrealDB 3.3.0 内存实例，只终止自身进程。[夹具清单](../tests/fixtures/mensoria/manifest.json)固定了 Mensoria 五份迁移文件；源基线没有提交，因此以逐文件哈希标识。设置 `MENSORIA_SOURCE_ROOT` 可复核外部检出中的对应文件，漂移会导致失败。

时间戳日志与 JSON 报告保存在 `.cache/qualification/`，失败运行也保留。脚本按实际测试结果映射 TC-001 至 TC-044，一个案例可以有多个检查。CI 编译宿主并在 Linux 上执行相同场景；本地证据只证明回执记录的系统／后端／源码组合。

## Mensoria 事务契约

启用 `mensoria-host`。动作使用 record 用户连接和环境范围的 `data_issuer` 连接，不使用 root。`SessionBinding` 只能来自已核验 ApiSession／SafetySnapshot，不是请求 DTO。`Command::decode` 在求值前拒绝重复键和未知成员；输入不能提供 actor、租户或角色权限。

当前绑定支持原生角色别名、所有者或指定读者可见性、动作写者和独立事件读者角色集合。获得读取权限意味着可读取完整模型快照，尚不支持字段掩码或外部策略引擎。应用业务租户需要进入可信作用域与显式策略，不能从用户输入推导可信租户。

固定原生事务重新检查 guard incarnation／security sequence、用户 active／role／epoch、会话族 generation／expiry、活动模型和记录 revision，并竞争 guard／会话族。一次事务同时写入：

- `ti_record`：候选记录及一次版本推进；
- `ti_audit`：作用域、身份域、actor、动作、制品、目标与前后版本／状态；
- 既有 `operation`：原始回执、指纹和保留键墓碑；
- 既有 `outbox`：稳定事件信封和持久投递状态。

`input_digest` 按附录 C 生成。创建时还把逻辑 ID 分配保留在 operation 中，同键更换分配目标会冲突。操作键绑定可信作用域、incarnation、身份域／ID。先查原操作，再核对当前活动制品，因此换版后仍可取回原制品的原结果。释放快照前检查当前写／读权限；回执过期只移除载荷，保留键和指纹。

单次原生查询的观察期限为三秒。超时、断连、取消和未分类数据库错误不能证明回滚；调用方必须核对原键，不能生成新键重试。业务求值失败发生在事务 admission 前。当前宿主不提供异步 accepted／pending 提交队列，只有获得完整原生中止响应时才区分确定拒绝。

普通用户的 CRUD、SQL、事务／批量插入和删除不能修改 Ti 表。Mensoria 通用集合仍使用既有 `p1_` 名称空间，不放宽所有者写入规则。Root 修复和导入属于独立的特权维护协议。

## 激活与投递

`install`、`activate_closed` 只供持有既有维护租约且已关闭 admission 的特权调用方使用，不在普通启动／动作请求中执行。安装保留已有表种类、Schema 和权限谓词，仅加入环境范围服务分支。激活校验已有记录、围栏活动 head、保留历史制品，拒绝不兼容切换。失败保持关闭，由 SafetyManager 决定重开。当前迁移扫描最多 1000 条记录，每页一条；更大环境需要另行验证的迁移工具。

事件适配器只向当前事件角色披露标准标签载荷。`begin_delivery` 持久记录尝试；`finish_delivery` 记录 Delivered、显式 RetrySafe 或 Unknown。只有具备幂等外部效果契约的适配器才能选择 RetrySafe。中断或未知尝试必须核对，不能盲目重放。当前不安装邮件／HTTP 副作用或自动平台 worker。

其他宿主上限：键 128 字节、ID 256 字节、64 个输入、256 个原生角色映射／别名和 actor 角色、角色名 128 字节、每条查询三秒；还执行语言标量／聚合限制。复用现有平台连接，不另建调度器。

## DataFacade 接入

受版本管理的适配器源码在 `integrations/mensoria/ti_language.rs`。

```sh
python3 scripts/install_mensoria_adapter.py /path/to/surrealdb-rust-ai-baas
# 在 Mensoria 工作区：
cargo check -p baas-adapters --features ti-language
```

`DataFacade::execute_ti_action(project, environment, access_token, artifact, bytes)` 复用不透明 access session、原生身份／会话族校验、ControlGate admission 和 SafetySnapshot，执行后再次校验身份。提交后的发布失败归为未知。artifact 由服务端 registry 选择。该 feature 不注册 HTTP 路由；安装脚本只改 Cargo 和模块源码，不执行数据库 DDL。

当前本地适配器已在实际 Mensoria 工作区编译通过。独立夹具尚未端到端执行完整管理面／会话／HTTP 路径，部署时需要按平台原流程验证目标迁移栈和该路径。

## 证据边界

原生测试包括真实表写入失败、三个事务中止点、并发审批／撤回、回执保留、角色／epoch／会话族／security 撤销、不兼容激活和旧制品核对。回环 TCP 代理真正丢弃原生提交响应，或持有响应后取消客户端任务，再核对持久回执；另一个 `fault=4` 仅模拟客户端观察丢失，不算网络故障证据。

尚待补充：每个语义表达式的精确源码 span、所有公开错误的 phase／operation／retry 元数据、超出这些回归场景的可移植穷尽夹具、持久化已受理拒绝回执、大环境迁移、字段级披露、完整 HTTP／通用 API 绕写验收，以及针对完整部署栈的分布式／恢复／撤销验证。Incarnation 围栏测试不等于物理恢复演练，本地通过不代表生产、HA、备份或外部副作用已获认证。
