# Decisions — tui-refactor

## 2026-03-24 Session Start

### Architecture Decisions
- Effect→NL 模块放 card-core（所有客户端复用）
- TUI 测试策略：仅 effect/text.rs 写单元测试，TUI 渲染通过 Agent QA
- app.rs 拆分为模块目录，不创建新 trait 抽象
- 半透明弹窗：不用 Clear，用深色背景 Block 模拟
- 快捷键映射：1-5前场, h手牌, c费用, f对手前场, b对手后场

### Module Structure Decision
- card-tui/src/ui/ directory 包含子模块（Task 1 创建）
- card-tui/src/network.rs — 网络相关函数
- card-tui/src/game_setup.rs — 游戏构建函数
- 旧 card-tui/src/ui.rs 删除（内容迁移到 ui/ 目录）
