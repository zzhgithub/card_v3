# 卡牌对战引擎 API 设计文档

## 第1章 概述

本文档描述卡牌对战游戏引擎当前公开 API 的设计与约束。
文档基于 Rust Workspace 多 crate 架构编写。
核心领域类型定义在 `card-core` crate。
网络协议类型定义在 `card-protocol` crate。
本文档面向引擎开发者、客户端开发者与脚本作者。
本文档以“公开可见类型与接口”为主线，强调序列化边界与协议边界。

### 1.1 文档目标

- 统一术语，避免客户端与服务端语义偏差。
- 明确结构体、枚举、trait 的职责边界。
- 规定跨进程和跨网络传输的数据形态。
- 为后续 `ClientApi` 与脚本系统联调提供标准输入输出。
- 记录当前实现与规划接口之间的一致性关系。

### 1.2 适用范围

- `card-core/src/types/*`
- `card-core/src/effect/mod.rs`
- `card-core/src/rules/mod.rs`
- `card-protocol/src/message.rs`
- `card-client` 中待实现的 `ClientApi` 设计接口
- JSON 卡牌定义数据约定与解析映射

### 1.3 术语约定

- “定义 ID”：卡牌模板层面的静态标识（`CardId`）。
- “实例 ID”：对局运行时对象标识（`InstanceId`）。
- “效果键”：卡牌效果集合中的键（`EffectKey`）。
- “公开类型”：通过模块 `pub use` 导出的类型。
- “协议消息”：WebSocket JSON 消息（`ClientMessage` / `ServerMessage`，定义于 `card-server::websocket_server`）。

## 第2章 标识符类型（card-core/types）

### 2.1 CardId（卡片定义 ID）

`CardId` 结构定义：

```rust
pub struct CardId(pub String);
```

格式规范：

- 规范格式：`S[卡包号]-[类型]-[卡编号]`
- 示例：`S001-C-001`
- 卡包号：`S` + 三位数字
- 类型位：`C` / `S` / `I` / `L`
- 卡编号：三位数字

构造与解析：

- `CardId::new(s: impl Into<String>) -> Self`
- `FromStr for CardId`：解析并校验格式
- `Display for CardId`：序列化为字符串

辅助读取方法：

- `pack() -> Option<&str>`
- `card_type() -> Option<&str>`
- `card_number() -> Option<&str>`

错误类型：

```rust
pub enum CardIdParseError {
    InvalidFormat,
    InvalidPackNumber,
    InvalidCardType,
    InvalidCardNumber,
}
```

序列化说明：

- `CardId` 可被 `serde` 序列化。
- 传输层中表现为字符串值。

### 2.2 InstanceId（牌局内实例 ID）

`InstanceId` 结构定义：

```rust
pub struct InstanceId(pub u32);
```

用途说明：

- 标识一次具体召唤或生成后的卡片实例。
- 同一 `CardId` 在同局可对应多个 `InstanceId`。
- 所有玩家操作命令以 `InstanceId` 指向对象。

### 2.3 EffectKey（效果标识符）

`EffectKey` 结构定义：

```rust
pub struct EffectKey(pub String);
```

用途说明：

- 作为 `CardDefinition.effects` 的键。
- 在协议命令中用于指定激活哪个效果。
- 在 `GameEvent::EffectActivated` 中用于广播效果来源。

### 2.4 PlayerId（玩家 ID）

`PlayerId` 枚举定义：

```rust
pub enum PlayerId {
    Player1,
    Player2,
}
```

方法：

- `opponent() -> PlayerId`：返回对手 ID。

语义约束：

- 全局仅允许两个玩家。
- 协议层与规则层共享同一枚举定义。

## 第3章 卡片属性枚举（card-core/types）

### 3.1 CardType（卡片类型）

```rust
pub enum CardType {
    Character,
    Strategy,
    Item,
    Legendary,
}
```

语义说明：

- `Character`：人物卡，具备战斗定位。
- `Strategy`：策略卡，使用 `StrategyKind` 细分。
- `Item`：物品卡，使用 `ItemKind` 细分。
- `Legendary`：传奇卡，受规则上限约束。

### 3.2 StrategyKind（策略卡子类型）

```rust
pub enum StrategyKind {
    Normal,
    Trick,
    Instant,
}
```

### 3.3 ItemKind（物品卡子类型）

```rust
pub enum ItemKind {
    Normal,
    Persistent,
}
```

### 3.4 Property（卡片属性）

```rust
pub enum Property {
    Rational,
    Divine,
    Spiritual,
}
```

规则说明：

- 每张卡仅允许一种 `Property`。
- `Property` 可用于效果筛选与免疫判定。

### 3.5 Category（卡片范畴）

```rust
pub enum Category {
    Math,
    Science,
    Literature,
    Philosophy,
    Mystery,
}
```

## 第4章 区域与位置（card-core/types）

### 4.1 Zone（区域）

```rust
pub enum Zone {
    Deck,
    Hand,
    Front(usize),
    Back(usize),
    CostZone,
    Grave,
}
```

区域语义：

- `Deck`：卡组。
- `Hand`：手牌。
- `Front(usize)`：前场槽位，索引语义由规则层限制。
- `Back(usize)`：后场槽位，索引语义由规则层限制。
- `CostZone`：费用区。
- `Grave`：墓地。

### 4.2 ZoneLocation（带玩家的区域定位）

```rust
pub struct ZoneLocation {
    pub player: PlayerId,
    pub zone: Zone,
}
```

构造方法：

- `ZoneLocation::new(player: PlayerId, zone: Zone) -> Self`

### 4.3 PlayerRef（玩家引用）

```rust
pub enum PlayerRef {
    Self_,
    Opponent,
}
```

解析方法：

- `resolve(current_player: PlayerId) -> PlayerId`

用途说明：

- 在效果 AST 中支持“相对玩家”表达。
- 便于脚本在不硬编码 `Player1/Player2` 的情况下复用。

### 4.4 CardRef（卡片引用）

当前实现：

```rust
pub enum CardRef {
    This,
    ByInstanceId(InstanceId),
    BySlot(Zone, usize),
}
```

语义说明：

- `This`：效果来源卡自身。
- `ByInstanceId`：按实例 ID 定位。
- `BySlot`：按区域槽位定位。

### 4.5 TargetRef（目标引用）

```rust
pub enum TargetRef {
    Player(PlayerRef),
    Card(CardRef),
    Zone(PlayerRef, Zone),
    SlotInZone(PlayerRef, Zone, usize),
}
```

辅助方法：

- `resolve_player(current_player: PlayerId) -> Option<PlayerId>`

## 第5章 CardDefinition（卡片定义）

### 5.1 CardDefinition 结构

当前实现：

```rust
pub struct CardDefinition {
    pub id: CardId,
    pub name: String,
    pub card_type: CardType,
    pub property: Property,
    pub category: Category,
    pub tags: Vec<String>,
    pub cost: u32,
    pub attack: Option<u32>,
    pub strategy_kind: Option<StrategyKind>,
    pub item_kind: Option<ItemKind>,
    pub effects: HashMap<EffectKey, serde_json::Value>,
}
```

字段说明：

- `id`：卡定义唯一标识。
- `name`：展示名称。
- `card_type`：主类型。
- `property`：属性轴。
- `category`：范畴轴。
- `tags`：标签集合，用于扩展筛选。
- `cost`：基础费用，当前类型为 `u32`。
- `attack`：攻击力，非战斗类卡可为空。
- `strategy_kind`：策略子类。
- `item_kind`：物品子类。
- `effects`：效果键到 JSON 值映射。

说明：

- 现阶段 `effects` 使用 `serde_json::Value`。
- 规则执行阶段可将 JSON 转换为 `Effect` AST。
- 文档中“CardDefinition”既代表数据结构，也代表脚本到引擎的核心映射对象。

### 5.2 CardDefinition 构建方法

- `new(id, name, card_type, property, category, cost) -> Self`
- `with_attack(attack) -> Self`
- `with_strategy_kind(kind) -> Self`
- `with_item_kind(kind) -> Self`
- `with_tags(tags) -> Self`
- `with_effect(key, effect_json) -> Self`

### 5.3 CardFilter（卡片过滤器）

当前实现：

```rust
pub struct CardFilter {
    pub card_type: Option<CardType>,
    pub property: Option<Property>,
    pub category: Option<Category>,
    pub tags: Option<Vec<String>>,
    pub min_cost: Option<u32>,
    pub max_cost: Option<u32>,
}
```

构建方法：

- `new() -> Self`
- `with_card_type(card_type) -> Self`
- `with_property(property) -> Self`
- `with_category(category) -> Self`
- `with_tags(tags) -> Self`
- `with_cost_range(min, max) -> Self`

匹配方法：

- `matches(card: &CardDefinition) -> bool`

匹配规则：

- 已设置字段必须全部满足。
- `tags` 采用“过滤器标签均被卡片包含”的全包含判定。
- `min_cost` 与 `max_cost` 共同约束费用区间。

## 第6章 效果系统（card-core/effect）

### 6.1 Effect（效果完整定义）

```rust
pub struct Effect {
    pub trigger: Trigger,
    pub optional: bool,
    pub activation_limit: Option<ActivationLimit>,
    pub conditions: Option<Condition>,
    pub choices: Vec<Choice>,
    pub actions: Vec<Action>,
    pub costs: Option<CostRequirement>,
}
```

执行顺序说明：

- 先判定 `trigger`。
- 再判定 `conditions`。
- 再收集 `choices`。
- 然后执行 `actions`。
- `costs` 在入链前支付。

### 6.2 Trigger（触发时点）

```rust
pub enum Trigger {
    TurnStart,
    DrawPhase,
    RecoveryPhase,
    OwnMainPhase,
    OpponentMainPhase,
    BothMainPhase,
    BattlePhase,
    TurnEnd,
    OnSummon,
    OnAttack,
    OnExpose,
    OnDestroy,
    OnEvent(EventTrigger),
}
```

### 6.3 EventTrigger（事件触发）

```rust
pub enum EventTrigger {
    RealPointChanged { player: PlayerRef },
    RealPointOverflow { player: PlayerRef },
    HpChanged { player: PlayerRef },
    CardDestroyed { filter: CardFilter },
    CardSummoned { filter: CardFilter },
    EffectActivated { filter: CardFilter },
    CardExposed { filter: CardFilter },
    CardDrawn { player: PlayerRef },
}
```

### 6.4 ActivationLimit（发动次数限制）

```rust
pub enum ActivationLimit {
    OncePerTurn,
    OncePerTurnSameName,
}
```

### 6.5 Condition AST（条件表达式）

```rust
pub enum Condition {
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    Compare { left: ValueExpr, op: CompareOp, right: ValueExpr },
    CardIsOnField { card: CardRef },
}
```

```rust
pub enum CompareOp {
    Gt,
    Lt,
    Ge,
    Le,
    Eq,
    Ne,
}
```

### 6.6 ValueExpr（数值表达式）

```rust
pub enum ValueExpr {
    Literal(i32),
    HandCount(PlayerRef),
    CostZoneCount(PlayerRef),
    CostZonePropertyCount { player: PlayerRef, property: Property },
    FrontFieldCount(PlayerRef),
    BackFieldCount(PlayerRef),
    RealPoint(PlayerRef),
    Hp(PlayerRef),
    HighestCostOnField(PlayerRef),
    AttackPower(CardRef),
}
```

### 6.7 Action（效果动作）

```rust
pub enum Action {
    Draw { player: PlayerRef, count: u8 },
    Damage { player: PlayerRef, amount: u8 },
    Destroy { target: CardRef },
    SummonFromZone { card_id: CardId, from_zones: Vec<Zone>, to_zone: Zone, no_cost: bool },
    ReturnToHand { target: CardRef },
    SendToGrave { target: CardRef },
    ModifyAttack { target: CardRef, amount: i16 },
    GainRealPoint { player: PlayerRef, amount: u8 },
    HealHp { player: PlayerRef, amount: u8 },
    Discard { player: PlayerRef, count: u8 },
    ApplyModifier { target: CardRef, modifier: Modifier, duration: ModifierDuration },
    RemoveModifier { target: CardRef, modifier_key: String },
    Special { key: EffectKey },
}
```

### 6.8 Choice/TargetType/TargetFilter（目标选择）

```rust
pub struct Choice {
    pub choice_id: u8,
    pub target_type: TargetType,
    pub filter: TargetFilter,
    pub count: ChoiceCount,
}
```

```rust
pub enum ChoiceCount {
    Exactly(u8),
    UpTo(u8),
    All,
}
```

```rust
pub enum TargetType {
    Player,
    Card,
    Zone,
}
```

```rust
pub struct TargetFilter {
    pub player: Option<PlayerRef>,
    pub card_filter: Option<CardFilter>,
    pub zone_filter: Option<Vec<Zone>>,
}
```

### 6.9 CostRequirement（额外发动费用）

```rust
pub enum CostRequirement {
    SendFieldCardToGrave { count: u8, filter: Option<CardFilter> },
    DiscardHand { count: u8 },
    SendCostZoneToGrave { count: u8, filter: Option<CardFilter> },
}
```

### 6.10 Modifier 与持续时间

```rust
pub enum Modifier {
    AttackBoost(i16),
    ImmuneToCardType(CardType),
    ImmuneToProperty(Property),
    ImmuneToDestruction,
    ImmuneToTargeting,
    CanAttackDirectly,
    ExtraAttackPerTurn(u8),
    CannotAttack,
    Special { key: String },
}
```

```rust
pub struct AppliedModifier {
    pub modifier: Modifier,
    pub source: InstanceId,
    pub duration: ModifierDuration,
}
```

```rust
pub enum ModifierDuration {
    WhileSourceOnField,
    UntilEndOfTurn,
    Permanent,
    TurnCount(u32),
}
```

## 第7章 游戏规则（card-core/rules）

### 7.1 CardRegistry（注册表访问接口）

```rust
pub trait CardRegistry {
    fn contains_card(&self, card_id: &CardId) -> bool;
    fn get_card_type(&self, card_id: &CardId) -> Option<char>;
}
```

职责说明：

- 为规则校验提供最小可用查询能力。
- 隔离校验逻辑与存储实现。

### 7.2 GameRules（可配置规则）

```rust
pub struct GameRules {
    pub initial_hp: u8,
    pub max_hp: u8,
    pub max_hand: usize,
    pub max_cost_zone: usize,
    pub max_real_point: u8,
    pub deck_size_range: (usize, usize),
    pub max_same_card: usize,
    pub max_legendary: usize,
    pub initial_hand_size: usize,
    pub first_player_draws: bool,
    pub operation_timeout: Duration,
    pub total_game_timeout: Duration,
}
```

默认值语义：

- 初始 HP：5。
- 最大 HP：6。
- 手牌上限：20。
- 费用区上限：6。
- RealPoint 上限：6。
- 卡组范围：40 到 60。
- 同名卡上限：3。
- 传奇卡上限：5。
- 初始手牌：5。
- 先手首回合不抽卡。
- 单次操作超时：30 秒。
- 全局对局超时：3600 秒。

### 7.3 validate_deck（卡组校验）

```rust
pub fn validate_deck(
    deck: &[CardId],
    rules: &GameRules,
    registry: &dyn CardRegistry,
) -> Result<(), String>
```

校验维度：

- 卡组总数是否在范围内。
- 同 ID 卡片数量是否超限。
- 传奇卡数量是否超限。
- 每张卡是否存在于注册表。

## 第8章 网络协议（card-protocol/message）

### 8.1 CostPayment（费用支付）

```rust
pub struct CostPayment {
    pub hand_cards: Vec<InstanceId>,
    pub real_point: u8,
}
```

### 8.2 AttackTarget（攻击目标）

```rust
pub enum AttackTarget {
    FrontSlot(usize),
    DirectAttack,
}
```

### 8.3 Phase（阶段）

```rust
pub enum Phase {
    TurnStart,
    Draw,
    Recovery,
    Main1,
    Battle,
    Main2,
    TurnEnd,
}
```

### 8.4 GameOverReason（结束原因）

```rust
pub enum GameOverReason {
    HpZero,
    DeckOut,
    Surrender,
    Timeout,
}
```

### 8.5 Command（客户端到主机）

```rust
pub enum Command {
    PlayCard { instance_id: InstanceId, target_zone: Zone, cost_payment: CostPayment },
    ActivateEffect { instance_id: InstanceId, effect_key: EffectKey, cost_payment: CostPayment },
    DeclareAttack { attacker: InstanceId, target: AttackTarget },
    DirectAttackRealPoint { real_point_used: u8 },
    ChainActivate { instance_id: InstanceId, effect_key: EffectKey, cost_payment: CostPayment },
    ChainPass,
    SelectRecoveryCards { instance_ids: Vec<InstanceId> },
    SelectTargets { targets: Vec<TargetRef> },
    SelectCards { instance_ids: Vec<InstanceId> },
    Surrender,
}
```

### 8.6 AvailableAction（可执行动作）

```rust
pub enum AvailableAction {
    PlayCard { instance_id: InstanceId },
    ActivateEffect { instance_id: InstanceId, effect_key: EffectKey },
    DeclareAttack { attacker: InstanceId, possible_targets: Vec<AttackTarget> },
    ChainActivate { instance_id: InstanceId, effect_key: EffectKey },
    ChainPass,
    SelectRecoveryCards { required_count: usize, candidates: Vec<InstanceId> },
    Surrender,
}
```

### 8.7 GameEvent（服务端广播事件）

`GameEvent` 是客户端动画、提示与重放的主数据源。
下列枚举项构成当前公开事件集合：

```rust
pub enum GameEvent {
    CardMoved { instance_id: InstanceId, from: ZoneLocation, to: ZoneLocation },
    CardSummoned { instance_id: InstanceId, to: ZoneLocation },
    CardDestroyed { instance_id: InstanceId },
    HpChanged { player: PlayerId, old_hp: u8, new_hp: u8 },
    RealPointChanged { player: PlayerId, old_rp: u8, new_rp: u8 },
    RealPointOverflow { player: PlayerId },
    AttackModified { instance_id: InstanceId, old_attack: i32, new_attack: i32 },
    PhaseChanged { new_phase: Phase },
    TurnChanged { new_active_player: PlayerId, turn_number: u32 },
    DrawCard { player: PlayerId, instance_id: InstanceId },
    EffectActivated { instance_id: InstanceId, effect_key: EffectKey },
    ChainStarted,
    ChainLink { instance_id: InstanceId, effect_key: EffectKey },
    ChainResolving { link_index: usize },
    ChainComplete,
    RequestAction { player: PlayerId, available_actions: Vec<AvailableAction>, timeout_secs: u64 },
    RequestTargetSelection { player: PlayerId, prompt: String, candidates: Vec<TargetRef>, count: usize, timeout_secs: u64 },
    RequestCardSelection { player: PlayerId, prompt: String, candidates: Vec<InstanceId>, count: usize, timeout_secs: u64 },
    GameOver { winner: Option<PlayerId>, reason: GameOverReason },
}
```

补充说明：

- `GameEvent` 与规则执行日志可以一一对应。
- 客户端应将 `GameEvent` 视为权威事实流。
- `GameEvent` 中包含请求型事件，用于驱动交互回合。

### 8.8 ~~NetworkMessage（TCP 外层封装）~~

> **已移除。** 原 Raw TCP 传输层使用 `NetworkMessage` + bincode 编码，现已废弃。
> 当前服务器使用 WebSocket + JSON，消息类型为 `ClientMessage` / `ServerMessage`
> （定义于 `card-server::websocket_server`）。
>
> 保留在 `card-protocol` 中的类型：`Command`、`AvailableAction`、`GameEvent`、
> `CostPayment`、`AttackTarget` 等，仍作为游戏语义层面的通用消息类型被
> `ClientApi` 与引擎使用。

## 第9章 ClientApi Trait（card-client，待实现）

### 9.1 设计目标

`ClientApi` 用于统一不同客户端形态。
目标是实现显示层与规则层彻底解耦。
`ClientApi` 允许多前端共享同一协议流程（当前实现为 Godot）。

### 9.2 规划接口

```rust
pub trait ClientApi: Send + Sync {
    async fn on_state_update(&mut self, state: VisibleGameState) -> Result<()>;
    async fn on_event(&mut self, event: GameEvent) -> Result<()>;
    async fn request_action(&mut self, actions: Vec<AvailableAction>, timeout: Duration) -> Result<Command>;
    async fn request_target(&mut self, prompt: &str, candidates: Vec<TargetRef>, count: usize, timeout: Duration) -> Result<Vec<TargetRef>>;
    async fn request_cards(&mut self, prompt: &str, candidates: Vec<InstanceId>, count: usize, timeout: Duration) -> Result<Vec<InstanceId>>;
}
```

### 9.3 VisibleGameState（规划）

建议可见状态至少包含：

- 自己的完整状态：手牌、费用区、前后场、HP、RP。
- 对手公开状态：场面信息与公开资源。
- 当前阶段与回合计数。
- 连锁栈可见信息。

### 9.4 ClientApi 与 GameEvent 关系

- `on_event` 接收离散 `GameEvent` 以驱动动画与提示。
- `on_state_update` 接收快照以修正显示一致性。
- 请求方法必须遵守超时输入约束。

## 第10章 JSON 卡牌定义格式规范

### 10.1 设计原则

- JSON 负责声明数据，不直接执行核心规则。
- Rust 引擎负责解释与执行。
- JSON 文件需稳定映射到 `CardDefinition`。
- 解析失败必须产生明确错误。

### 10.2 基础卡片模板（JSON）

```json
{
  "id": "S000-C-001",
  "name": "测试人物甲",
  "card_type": "Character",
  "property": "Rational",
  "category": "Math",
  "cost": 3,
  "attack": 1500,
  "tags": ["测试标签"],
  "effects": {
    "e1": {
      "trigger": "OnSummon",
      "optional": false,
      "actions": [
        { "type": "HealHp", "player": "Self_", "amount": 1 }
      ]
    }
  }
}
```

### 10.3 策略卡模板（JSON）

```json
{
  "id": "S000-S-001",
  "name": "测试策略甲",
  "card_type": "Strategy",
  "strategy_kind": "Normal",
  "property": "Rational",
  "category": "Science",
  "cost": 2,
  "effects": {
    "e1": {
      "trigger": "OwnMainPhase",
      "optional": true,
      "conditions": {
        "type": "Compare",
        "left": { "type": "HandCount", "player": "Opponent" },
        "op": "Gt",
        "right": { "type": "HandCount", "player": "Self_" }
      },
      "actions": [
        { "type": "Draw", "player": "Self_", "count": 1 }
      ]
    }
  }
}
```

### 10.4 带费用要求的 JSON 效果

```json
{
  "e1": {
    "trigger": "OwnMainPhase",
    "optional": true,
    "costs": {
      "type": "SendFieldCardToGrave",
      "count": 1,
      "filter": null
    },
    "actions": [
      { "type": "Damage", "player": "Opponent", "amount": 1 }
    ]
  }
}
```

### 10.5 带修饰器的 JSON 效果

```json
{
  "e1": {
    "trigger": "OnSummon",
    "optional": false,
    "actions": [
      {
        "type": "ApplyModifier",
        "target": "This",
        "modifier": { "type": "ImmuneToCardType", "card_type": "Strategy" },
        "duration": "WhileSourceOnField"
      }
    ]
  }
}
```

### 10.6 JSON 到 CardDefinition 映射建议

- `id` 映射为 `CardId`。
- `card_type` 映射为 `CardType`。
- `strategy_kind`/`item_kind` 映射为对应子类型。
- `effects` 先映射为 `HashMap<EffectKey, serde_json::Value>`。
- 运行期将 JSON 解析为 `Effect` AST。

## 第11章 错误类型与边界（规划）

### 11.1 card-core 错误（规划名）

- `CoreError::InvalidCardId`
- `CoreError::CardNotFound`
- `CoreError::InvalidZone`
- `CoreError::RuleViolation`
- `CoreError::InvalidCommand`
- `CoreError::InvalidTarget`
- `CoreError::EffectError`
- `CoreError::GameOver`

### 11.2 card-script 错误（规划名）

- `ScriptError::ParseError`
- `ScriptError::CardNotFound`
- `ScriptError::InvalidCardDefinition`

### 11.3 card-protocol 错误（规划名）

- `ProtocolError::SerializationError`
- `ProtocolError::DeserializationError`
- `ProtocolError::InvalidMessage`
- `ProtocolError::VersionMismatch`

### 11.4 传递策略

- crate 内部使用精确错误枚举。
- 应用层使用 `anyhow::Result` 汇聚传播。
- 网络边界禁止回传不安全内部细节。

## 第12章 兼容性说明与实现差异

### 12.1 CardDefinition 字段差异

- 文档规划常见写法为 `cost: u8`。
- 当前实现为 `cost: u32`。
- 文档规划常见写法为 `attack: Option<i32>`。
- 当前实现为 `attack: Option<u32>`。

### 12.2 CardRef 形态差异

- 规划中常见 `Choice(u8)` 与 `SpecificInstance(InstanceId)`。
- 当前实现使用 `ByInstanceId` 与 `BySlot`。

### 12.3 effects 存储差异

- 规划中常见 `HashMap<EffectKey, Effect>`。
- 当前实现使用 `HashMap<EffectKey, serde_json::Value>`。
- 该差异是脚本层到执行层的过渡设计。

### 12.4 对 ClientApi 的影响

- `ClientApi` 不直接依赖 JSON 解析细节。
- `ClientApi` 只依赖可见状态、命令与 `GameEvent`。
- 客户端无需感知内部 AST 结构体的全部细节。

## 第13章 API 使用建议

### 13.1 客户端建议

- 以 `GameEvent` 为动画驱动主输入。
- 以快照状态为显示校正输入。
- 对 `RequestAction` 系列消息实现超时处理。

### 13.2 引擎建议

- 所有外部输入先做结构校验再做规则校验。
- 所有 `InstanceId` 引用在执行前必须存在性校验。
- 所有 `Zone` 索引在执行前必须边界校验。

### 13.3 脚本建议

- JSON 定义保持声明式与最小化。
- 避免在 JSON 中嵌入业务逻辑。
- 用统一键名降低脚本迁移成本。

## 第14章 关键字索引

- ClientApi
- CardDefinition
- GameEvent
- JSON
- CardId
- InstanceId
- EffectKey
- GameRules
- ~~NetworkMessage~~（已移除，原 TCP 传输封包）

## 第15章 小结

本文档给出当前公开 API 的统一视图。
文档覆盖类型定义、效果系统、规则配置、网络协议与客户端接口。
文档明确了 `CardDefinition`、`GameEvent`、`ClientApi`、`JSON` 四条核心边界。
后续实现可直接以本文件作为跨 crate 协作基线。
