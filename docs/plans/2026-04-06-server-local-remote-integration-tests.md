# Server Local/Remote Integration Tests Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为当前项目建立一套可重复执行的测试体系，验证 server 同时连接一个本地 client 和一个远程 client 时，用户选择、连锁消费与效果执行都能正确工作。

**Architecture:** 测试分四层推进：先压实 `card-core` 规则与链解析，再验证 `card-server` 协议与连接，再做“一个本地脚本 client + 一个远程 TCP client”的混合联调，最后补超时、断线、非法输入等破坏性场景。只在跨层联调和复杂链式场景中加入入侵式日志，并把日志输出到可分析的文件，避免所有测试都被日志噪音污染。

**Tech Stack:** Rust, cargo test, tokio, TCP, Bevy-less scripted clients, tracing/tracing-subscriber, 当前 crates: `card-core`, `card-server`, `card-client`, `card-protocol`

---

### Task 1: 建立测试总目录与命名约定

**Files:**
- Create: `docs/testing/server-local-remote-test-matrix.md`
- Create: `crates/card-core/tests/engine_flow_tests.rs`
- Create: `crates/card-server/tests/network_protocol_tests.rs`
- Create: `crates/card-server/tests/mixed_local_remote_session_tests.rs`

**Step 1: 写失败的占位测试**

在 3 个测试文件中各写 1 个最小占位测试，函数名分别为：
- `engine_chain_resolution_smoke`
- `network_handshake_smoke`
- `mixed_session_smoke`

**Step 2: 运行测试并确认能被 test harness 发现**

Run:
```bash
cargo test --package card-core engine_chain_resolution_smoke -- --nocapture
cargo test --package card-server network_handshake_smoke -- --nocapture
cargo test --package card-server mixed_session_smoke -- --nocapture
```

Expected: 测试先失败或 `todo!()`，但能被正确发现并执行。

**Step 3: 写测试矩阵文档**

在 `docs/testing/server-local-remote-test-matrix.md` 中列出 4 层测试：
- 引擎层
- 协议层
- 混合联调层
- 破坏性场景层

并给每层写：目标、输入、断言对象、是否需要日志文件。

**Step 4: 再次运行占位测试**

Expected: 文件结构与命名固定下来，后续实现不再改动路径。

---

### Task 2: 实现引擎层脚本化客户端测试（不加日志文件）

**Files:**
- Modify: `crates/card-core/tests/engine_flow_tests.rs`
- Reference: `crates/card-core/src/engine/game_engine.rs`
- Reference: `crates/card-core/src/engine/phase.rs`
- Reference: `crates/card-core/src/engine/chain.rs`

**Step 1: 写失败测试：基础选择与效果执行**

增加这些测试：
- `play_card_moves_card_to_expected_zone`
- `declare_attack_applies_expected_result`
- `effect_execution_updates_event_log_and_final_state`

测试里先定义一个最小 `ScriptedPhaseClient`，按脚本顺序返回 `PhaseAction`。

**Step 2: 运行并确认失败原因正确**

Run:
```bash
cargo test --package card-core play_card_moves_card_to_expected_zone -- --nocapture
```

Expected: 如果前置构造不完整，则失败点应落在断言或局面构造，而不是网络相关错误。

**Step 3: 扩展 `ScriptedPhaseClient` 支持复杂选择**

支持：
- `choose_action`
- `choose_recovery_cards`

如果后续 `card-core` 还需要目标或卡牌选择，则在测试内部继续扩展脚本队列，不要先做过度抽象。

**Step 4: 补连锁与消费测试**

增加：
- `chain_resolves_in_filo_order`
- `cost_payment_moves_cards_into_cost_zone`
- `chain_pass_ends_chain_resolution`

断言必须同时检查：
- `GameResult.final_state`
- `GameResult.event_log`

**Step 5: 跑完整引擎层测试**

Run:
```bash
cargo test --package card-core --test engine_flow_tests -- --nocapture
```

Expected: 这一层不需要额外日志文件，靠测试断言和 `event_log` 就能定位。

---

### Task 3: 实现协议层 TCP 测试（局部内存日志）

**Files:**
- Modify: `crates/card-server/tests/network_protocol_tests.rs`
- Reference: `crates/card-server/src/network.rs`
- Reference: `crates/card-server/src/remote_client.rs`
- Reference: `crates/card-protocol/src/message.rs`
- Reference: `crates/card-protocol/src/codec.rs`

**Step 1: 写失败测试：握手成功路径**

增加：
- `server_accepts_two_players_and_sends_game_start`
- `version_mismatch_is_rejected`
- `invalid_deck_is_rejected`

这里写一个 `ProtocolTestClient`，直接通过 `TcpConnection` 连 server，按协议脚本发：
- `Hello`
- `DeckSubmit`
- 后续响应

**Step 2: 运行并确认失败在协议断言层**

Run:
```bash
cargo test --package card-server server_accepts_two_players_and_sends_game_start -- --nocapture
```

Expected: 如果失败，应表现为收发消息顺序不符，而不是测试基础设施起不来。

**Step 3: 实现消息序列断言工具**

为 `ProtocolTestClient` 增加：
- `recv_expect_hello_ack()`
- `recv_expect_deck_accepted()`
- `recv_expect_game_start()`
- `recv_expect_disconnect(reason_substr)`

**Step 4: 补请求/响应类测试**

增加：
- `request_action_round_trip_returns_command_response`
- `request_card_selection_round_trip_returns_card_response`
- `request_target_selection_round_trip_returns_target_response`

**Step 5: 跑完整协议层测试**

Run:
```bash
cargo test --package card-server --test network_protocol_tests -- --nocapture
```

Expected: 这一层默认不落日志文件，只在失败时打印消息序列。

---

### Task 4: 实现混合联调测试（需要入侵式日志）

**Files:**
- Modify: `crates/card-server/tests/mixed_local_remote_session_tests.rs`
- Modify: `crates/card-server/src/network.rs`（仅在必要时增加测试期日志）
- Modify: `crates/card-server/src/session.rs`（仅在必要时增加测试期日志）
- Modify: `crates/card-server/src/remote_client.rs`（仅在必要时增加测试期日志）
- Create: `logs/test-runs/`（运行时输出日志目录，可 gitignore）

**Step 1: 写失败测试：一个本地脚本 client + 一个远程 TCP client**

增加：
- `mixed_local_remote_session_reaches_first_action_request`
- `mixed_local_remote_session_can_finish_one_turn`

这里的“本地 client”不要用 Bevy UI，而是同进程的 `ScriptedPhaseClient` / `ClientApi` 适配器；“远程 client”用真实 TCP。

**Step 2: 为复杂联调引入测试期日志初始化**

只在这个测试文件里初始化 `tracing_subscriber`，把日志写到：
- `logs/test-runs/mixed-session-<test-name>.log`

日志至少包含：
- server 启动与 bind 地址
- 两端握手完成
- deck 校验结果
- RequestAction / RequestCardSelection / RequestTargetSelection 发出时刻
- CommandResponse / CardResponse / TargetResponse 收到时刻
- turn/phase 变化
- chain start / link / resolve / complete

**Step 3: 跑测试并保存日志**

Run:
```bash
cargo test --package card-server --test mixed_local_remote_session_tests mixed_local_remote_session_reaches_first_action_request -- --nocapture
```

Expected: 若失败，先不要直接改逻辑，先打开对应日志文件分析卡在哪一层。

**Step 4: 增加链与消费的混合场景**

增加：
- `mixed_session_chain_activation_and_chain_pass`
- `mixed_session_cost_payment_is_applied_before_effect_resolution`
- `mixed_session_effect_execution_matches_remote_notifications`

这些测试必须落日志文件，并在断言外再检查日志中是否出现关键节点顺序：
- `RequestAction`
- `CommandResponse`
- `ChainStarted`
- `ChainLink`
- `ChainResolving`
- `ChainComplete`

**Step 5: 日志分析约定**

失败时按这个顺序分析日志：
1. 是否握手成功
2. 是否 deck 校验通过
3. 是否正确发出 Request*
4. 是否收到 Response*
5. 是否进入链
6. 是否完成效果解析

只在这一层和下一层保留文件日志；前两层不需要。

---

### Task 5: 破坏性场景测试（需要日志文件）

**Files:**
- Modify: `crates/card-server/tests/mixed_local_remote_session_tests.rs`
- Possibly Modify: `crates/card-server/src/network.rs`

**Step 1: 写失败测试：超时与断线**

增加：
- `remote_client_timeout_results_in_game_over`
- `remote_disconnect_during_chain_aborts_session_cleanly`
- `illegal_command_is_rejected_or_ignored_without_corrupting_state`

**Step 2: 运行并记录日志**

Run:
```bash
cargo test --package card-server --test mixed_local_remote_session_tests remote_client_timeout_results_in_game_over -- --nocapture
```

Expected: 生成对应日志文件，失败时能看到精确断点。

**Step 3: 补断言**

对每个失败路径同时断言：
- `GameOverReason` / `Disconnect.reason`
- server 没有挂死
- 对端收到的最后一个协议消息是合理的

**Step 4: 跑完整破坏性场景**

Run:
```bash
cargo test --package card-server --test mixed_local_remote_session_tests -- --nocapture
```

---

### Task 6: 统一回归入口与执行顺序

**Files:**
- Create: `scripts/run_integration_test_matrix.sh`
- Create: `docs/testing/how-to-read-integration-logs.md`

**Step 1: 写执行脚本**

脚本顺序固定为：
1. `cargo test --package card-core --test engine_flow_tests -- --nocapture`
2. `cargo test --package card-server --test network_protocol_tests -- --nocapture`
3. `cargo test --package card-server --test mixed_local_remote_session_tests -- --nocapture`

脚本失败即退出。

**Step 2: 写日志阅读文档**

文档说明：
- 哪些测试会产生日志文件
- 日志目录位置
- 如何按握手 → 请求 → 响应 → 链 → 效果顺序排查

**Step 3: 运行总脚本验证**

Run:
```bash
bash scripts/run_integration_test_matrix.sh
```

Expected: 全套测试可一键执行。

---

### Task 7: 最终验证与回归检查

**Files:**
- Verify: `crates/card-core/tests/engine_flow_tests.rs`
- Verify: `crates/card-server/tests/network_protocol_tests.rs`
- Verify: `crates/card-server/tests/mixed_local_remote_session_tests.rs`
- Verify: `scripts/run_integration_test_matrix.sh`

**Step 1: 跑每层测试**

Run:
```bash
cargo test --package card-core --test engine_flow_tests -- --nocapture
cargo test --package card-server --test network_protocol_tests -- --nocapture
cargo test --package card-server --test mixed_local_remote_session_tests -- --nocapture
```

**Step 2: 跑总脚本**

Run:
```bash
bash scripts/run_integration_test_matrix.sh
```

**Step 3: 人工抽查日志**

只抽查复杂场景日志文件，确认至少一条成功链路和一条失败链路的日志是可读、可定位的。

**Step 4: Commit**

仅在用户要求时执行：
```bash
git add crates/card-core/tests/engine_flow_tests.rs \
        crates/card-server/tests/network_protocol_tests.rs \
        crates/card-server/tests/mixed_local_remote_session_tests.rs \
        docs/testing/server-local-remote-test-matrix.md \
        docs/testing/how-to-read-integration-logs.md \
        scripts/run_integration_test_matrix.sh
git commit -m "test: add server local remote integration matrix"
```
