# Ti 语言标准

<p class="ti-lead">面向 Mensoria 的声明式业务语言。</p>

Ti 用实体、不变量、策略、动作和事件描述业务对象及其合法变化。由 Mensoria 提供身份、持久化与受控事务，统一执行声明的业务规则。

<p class="ti-version">草案 0.1 <span>·</span> ti/0.1 <span>·</span> approval-core/0.1</p>

## 从这里开始

<dl class="ti-reading-path">
<dt><a href="overview.md">语言概览</a></dt>
<dd>了解核心概念、表达方式与首版边界。</dd>
<dt><a href="approval.md">审批示例</a></dt>
<dd>从请假申请出发，阅读创建、提交、批准与撤回的完整模型。</dd>
<dt><a href="01-purpose.md">阅读标准</a></dt>
<dd>按章节查阅语法、类型、授权条件和原子动作的精确定义。</dd>
</dl>

## 声明一次业务变化

下面节选自审批示例。动作声明来源状态、目标状态、授权条件与需要记录的业务事件。

```text
action Approve on LeaveRequest {
    from Pending to Approved;
    allow Reviewer(self);
    emit LeaveApproved {
        requestId = self.id;
        applicant = self.applicant;
        days = self.days;
    }
}
```

赋值读取旧记录，事件读取候选记录。业务状态、操作结果、审计和 outbox 由宿主原子提交。完整定义见[创建与原子动作](07-actions.md)。

## 规范参考

| 查阅内容 | 章节 |
| --- | --- |
| 语法与表达式 | [类型与值语义](04-types.md) · [表达式与求值上下文](05-expressions.md) · [完整 EBNF](a-grammar.md) |
| 执行与失败 | [原子动作](07-actions.md) · [事件与审计](08-events.md) · [幂等与结果](09-operations.md) |
| 平台接入 | [Mensoria 集成](integration.md) · [模型演进](10-evolution.md) |
| 实现验证 | [符合性矩阵](conformance.md) · [交付阶段](roadmap.md) |

## 文档状态

本书为标准草案，已有 Rust 参考校验器和纯求值器，数据库宿主尚未接入。首版聚焦单对象、单级审批；文档发布不代表运行时符合性已通过验证。

英文为规范源文，中文版使用相同条款编号与语法。两种版本共用一套模型，按对应章节阅读。
