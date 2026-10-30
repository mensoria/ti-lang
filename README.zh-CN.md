# Ti Lang

**面向 Mensoria 的声明式业务语言。**

[English](README.md) · [中英文文档官网](https://mensoria.github.io/ti-lang/)

Ti 声明业务实体、不变量、授权条件、状态转换和事件；Mensoria 提供可信身份、持久化、受控事务、原操作核对和事件投递。

Ti 的名称取意于元素符号为 Ti 的钛。它是品牌意象，不是技术保证。

## 标准与阅读

- [中文标准草案](docs/ti-language-standard.zh-CN.md) · [英文规范源文](docs/ti-language-standard.md)
- [符合性矩阵](docs/conformance.zh-CN.md)：44 个已有可执行检查的编号场景。
- [Mensoria 集成边界](docs/mensoria-integration.zh-CN.md)
- [实施阶段与验收](docs/roadmap.zh-CN.md)
- [请假审批源码](examples/approval.ti)

在线书每种语言 21 章，含 54 条规范、完整 EBNF、指纹编码、审批示例、集成与验收参考。支持逐章语言切换和搜索。

## 当前状态

草案 0.1，语言版次 ti/0.1。已有 Rust 编译器、求值器、严格 wire 编解码、冻结制品和可选 Mensoria 事务宿主。44 个编号回归场景均可执行；验证范围见[实现说明](docs/implementation.zh-CN.md)，尚不声明完整标准或生产符合性。

第一版聚焦单对象、单级审批。受管对象的公开写入只走动作，包含创建和草稿编辑。通用函数、任意 SQL、循环、外部 I/O、多对象动作和长流程编排不在首版范围。

## 本地构建

```sh
python3 scripts/bootstrap_mdbook.py
python3 scripts/build_book.py
python3 -m http.server 8000 --bind 127.0.0.1 --directory site
```

打开 `http://127.0.0.1:8000/`。编写、构建与 GitHub Pages 发布说明见[出版指南](docs/publishing.md)。文档构建不能证明语言执行或生产符合性。

实现许可证尚未选定；本仓库没有复制 Rip 上游代码。

## 运行参考内核

```sh
cargo run --locked -- check examples/approval.ti
cargo run --locked -- eval examples/approval.ti examples/create-request.json
```

独立 `eval` 只返回候选结果；真实事务使用可选宿主。执行全部编号场景：

```sh
python3 scripts/setup_native_backend.py
TI_SURREAL_BIN="$PWD/.cache/native-backend/surreal" python3 scripts/verify_conformance.py
```

接入和边界见[实现说明](docs/implementation.zh-CN.md)。
