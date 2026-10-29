# Ti 语言概览

Ti 定义业务对象及其允许发生的变化，不描述通用程序指令序列。可信宿主按照明确的身份、并发和失败规则执行已校验模型。

**规范权威：**[Ti 语言标准——草案 0.1](ti-language-standard.zh-CN.md)。本页为说明性概览，不建立第二套契约。

## 核心概念

| 构造 | 含义 |
| --- | --- |
| entity | 持久业务字段和一个状态机 |
| invariant | 与操作者无关、每次写入检查的约束 |
| policy | 对一个实体和可信操作者求值的纯授权谓词 |
| create | 授权创建并初始化必填字段 |
| action | 明确选择、原子修改一个记录的状态转换 |
| event | 与状态一起持久化的有类型业务事实 |

## 示例

```text
action Reject(reason: String) on LeaveRequest {
    from Pending to Rejected;
    allow Reviewer(self);
    require nonempty(input.reason) else "rejection_reason_required";
    set rejectionReason = trim(input.reason);
}
```

完整[请假审批模型](../examples/approval.ti)涵盖创建、草稿编辑、提交、批准、拒绝与撤回。它是说明性源码，尚不可运行。

赋值表达式都读取旧记录，然后同时应用。不变量和事件载荷读取候选记录。业务状态、审计、操作结果和 outbox 必须经宿主原子提交。未知结果须核对原操作，不得盲目重放。

## 初始边界

approval-core/0.1 支持 Bool、String、Int64、名义 ID、可空标量字段、有界纯表达式、单状态机和单记录动作。不包含通用函数、算术、金额／日期、跨记录读取、循环、外部 I/O、计时器和长流程编排。

EBNF、精确类型、求值上下文、限制、诊断、演进及宿主要求见标准；声称支持前需要的证据见[符合性矩阵](conformance.zh-CN.md)。
