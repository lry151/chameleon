# Domain Docs

工程技能在探索代码库时应如何消费本仓库的领域文档。

## 探索前先读这些

- 仓库根部的 **`CONTEXT.md`**，或
- 若存在则先读 **`CONTEXT-MAP.md`**——它指向每个上下文各一份 `CONTEXT.md`。读与你主题相关的每一份。
- **`docs/adr/`**——读你即将工作区域所触及的 ADR。多上下文仓库还需查看 `src/<context>/docs/adr/` 里的上下文级决定。

若这些文件不存在，**静默继续**。不要标记它们的缺失，不要建议先创建。`/domain-modeling` 技能（经 `/grill-with-docs` 与 `/improve-codebase-architecture` 触达）在术语或决定真正落定时才惰性创建它们。

## 文件结构

单上下文仓库（大多数仓库）：

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-event-sourced-orders.md
│   └── 0002-postgres-for-write-model.md
└── src/
```

多上下文仓库（根部存在 `CONTEXT-MAP.md`）：

```
/
├── CONTEXT-MAP.md
├── docs/adr/                          ← 系统级决定
└── src/
    ├── ordering/
    │   ├── CONTEXT.md
    │   └── docs/adr/                  ← 上下文级决定
    └── billing/
        ├── CONTEXT.md
        └── docs/adr/
```

chameleon 为**单上下文**：仓库根部一个 `CONTEXT.md` + `docs/adr/`。

## 使用术语表的词汇

当你的输出命名一个领域概念（在 issue 标题、重构提案、假设、测试名里出现时），使用 `CONTEXT.md` 中定义的术语。不要漂移到术语表明确规避的同义词。

若你需要的概念还不在术语表里，那是个信号——要么你在发明项目不用的语言（重新考虑），要么存在真实缺口（留给 `/domain-modeling` 记下）。

## 标记 ADR 冲突

若你的输出与现有 ADR 矛盾，显式提出来，而不是静默覆盖：

> _与 ADR-0007（event-sourced orders）矛盾——但值得重开，因为…_