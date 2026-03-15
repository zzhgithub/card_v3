//! Effect system AST type definitions.
//!
//! Pure data structures describing card effects as an abstract syntax tree.
//! No execution logic — Lua scripts define effects using these types,
//! and the Rust engine interprets and executes them.

use serde::{Deserialize, Serialize};

use crate::types::{
    CardFilter, CardId, CardRef, CardType, EffectKey, InstanceId, PlayerRef, Property, Zone,
};

#[cfg(test)]
mod tests;

pub mod evaluator;
pub use evaluator::{evaluate_condition, resolve_value};

// ─── Core Effect Structure ───────────────────────────────────────────────────

/// A complete effect definition on a card.
///
/// Maps to a value in `CardDefinition.effects` (keyed by `EffectKey`).
/// Describes trigger timing, conditions, target selection, actions, and costs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    /// When this effect can trigger (phase-based, action-based, or event-based).
    pub trigger: Trigger,
    /// If `true`, the player is asked whether to activate.
    /// If `false`, the effect activates automatically when triggered.
    pub optional: bool,
    /// Limits on how often this effect can activate.
    pub activation_limit: Option<ActivationLimit>,
    /// Conditions that must hold for the effect to be activatable.
    pub conditions: Option<Condition>,
    /// Targets the player must choose before resolution.
    pub choices: Vec<Choice>,
    /// Actions performed when the effect resolves (executed in order).
    pub actions: Vec<Action>,
    /// Additional costs required to activate (paid before entering the chain).
    pub costs: Option<CostRequirement>,
}

// ─── Trigger System ──────────────────────────────────────────────────────────

/// Defines when an effect can trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    // Phase triggers
    /// At the start of the turn.
    TurnStart,
    /// During the draw phase.
    DrawPhase,
    /// During the recovery phase.
    RecoveryPhase,
    /// During the controlling player's main phase.
    OwnMainPhase,
    /// During the opponent's main phase.
    OpponentMainPhase,
    /// During either player's main phase.
    BothMainPhase,
    /// During the battle phase.
    BattlePhase,
    /// At the end of the turn.
    TurnEnd,

    // Self action triggers
    /// When this card is summoned to the field.
    OnSummon,
    /// When this card declares an attack.
    OnAttack,
    /// When this card is exposed (flipped face-up).
    OnExpose,
    /// When this card is destroyed.
    OnDestroy,

    // Event-based triggers
    /// When a specific game event occurs.
    OnEvent(EventTrigger),
}

/// Event-based trigger conditions for reactive effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventTrigger {
    /// A player's RealPoint value changed.
    RealPointChanged { player: PlayerRef },
    /// A player's RealPoint exceeded the maximum (6), triggering an overflow event.
    RealPointOverflow { player: PlayerRef },
    /// A player's HP changed.
    HpChanged { player: PlayerRef },
    /// A card matching the filter was destroyed.
    CardDestroyed { filter: CardFilter },
    /// A card matching the filter was summoned.
    CardSummoned { filter: CardFilter },
    /// An effect was activated on a card matching the filter.
    EffectActivated { filter: CardFilter },
    /// A card matching the filter was exposed.
    CardExposed { filter: CardFilter },
    /// A player drew a card.
    CardDrawn { player: PlayerRef },
}

/// Limits on effect activation frequency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivationLimit {
    /// This specific card instance can activate this effect once per turn.
    OncePerTurn,
    /// All cards with the same name share a once-per-turn activation limit.
    OncePerTurnSameName,
}

// ─── Condition AST ───────────────────────────────────────────────────────────

/// Boolean condition expression tree for effect activation requirements.
///
/// Supports `And`/`Or`/`Not` composition, enabling Lua scripts to define
/// arbitrarily complex conditions that are parsed into this AST.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Condition {
    /// All sub-conditions must be true.
    And(Vec<Condition>),
    /// At least one sub-condition must be true.
    Or(Vec<Condition>),
    /// The sub-condition must be false.
    Not(Box<Condition>),
    /// Compare two numeric values.
    Compare {
        left: ValueExpr,
        op: CompareOp,
        right: ValueExpr,
    },
    /// Check whether a specific card is currently on the field.
    CardIsOnField { card: CardRef },
}

/// Comparison operators for [`Condition::Compare`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareOp {
    /// Greater than (`>`).
    Gt,
    /// Less than (`<`).
    Lt,
    /// Greater than or equal (`>=`).
    Ge,
    /// Less than or equal (`<=`).
    Le,
    /// Equal (`==`).
    Eq,
    /// Not equal (`!=`).
    Ne,
}

/// Value expressions that resolve to integers at runtime.
///
/// Used in [`Condition::Compare`] and other numeric contexts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValueExpr {
    /// A constant integer value.
    Literal(i32),
    /// Number of cards in a player's hand.
    HandCount(PlayerRef),
    /// Number of cards in a player's cost zone.
    CostZoneCount(PlayerRef),
    /// Number of cards with a specific property in a player's cost zone.
    CostZonePropertyCount {
        player: PlayerRef,
        property: Property,
    },
    /// Number of cards in a player's front field.
    FrontFieldCount(PlayerRef),
    /// Number of cards in a player's back field.
    BackFieldCount(PlayerRef),
    /// A player's current RealPoint value.
    RealPoint(PlayerRef),
    /// A player's current HP.
    Hp(PlayerRef),
    /// The highest cost among cards on a player's field.
    HighestCostOnField(PlayerRef),
    /// A specific card's current attack power.
    AttackPower(CardRef),
}

// ─── Actions ─────────────────────────────────────────────────────────────────

/// Concrete actions performed when an effect resolves.
///
/// Actions execute in order. Target references (`CardRef`) are resolved
/// at runtime — `CardRef::This` refers to the effect's source card.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    /// A player draws cards from their deck.
    Draw { player: PlayerRef, count: u8 },
    /// Deal damage to a player's HP.
    Damage { player: PlayerRef, amount: u8 },
    /// Destroy a card and send it to the graveyard.
    Destroy { target: CardRef },
    /// Summon a specific card from designated zones without (optionally) paying its cost.
    SummonFromZone {
        card_id: CardId,
        from_zones: Vec<Zone>,
        to_zone: Zone,
        no_cost: bool,
    },
    /// Return a card to its owner's hand.
    ReturnToHand { target: CardRef },
    /// Send a card to its owner's graveyard (without "destroying" it).
    SendToGrave { target: CardRef },
    /// Modify a card's attack power by a signed amount.
    ModifyAttack { target: CardRef, amount: i16 },
    /// A player gains RealPoint.
    GainRealPoint { player: PlayerRef, amount: u8 },
    /// A player recovers HP.
    HealHp { player: PlayerRef, amount: u8 },
    /// A player discards cards from their hand (player chooses which).
    Discard { player: PlayerRef, count: u8 },
    /// Apply a persistent modifier to a card.
    ApplyModifier {
        target: CardRef,
        modifier: Modifier,
        duration: ModifierDuration,
    },
    /// Remove a modifier from a card by its key.
    RemoveModifier {
        target: CardRef,
        modifier_key: String,
    },
    /// A special/unique action identified by key, handled by custom runtime logic.
    Special { key: EffectKey },
}

// ─── Choice / Targeting ──────────────────────────────────────────────────────

/// Describes a target selection the player must make before an effect resolves.
///
/// Each choice has a unique `choice_id` within the effect. The runtime presents
/// the choice to the player and maps the result to concrete targets for actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Unique identifier for this choice within the effect.
    pub choice_id: u8,
    /// What kind of game object is being targeted.
    pub target_type: TargetType,
    /// Constraints on valid targets.
    pub filter: TargetFilter,
    /// How many targets must or can be selected.
    pub count: ChoiceCount,
}

/// How many targets a [`Choice`] requires.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChoiceCount {
    /// The player must select exactly this many targets.
    Exactly(u8),
    /// The player may select up to this many targets (including zero).
    UpTo(u8),
    /// All valid targets are automatically selected.
    All,
}

/// The kind of game object being targeted by a [`Choice`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetType {
    Player,
    Card,
    Zone,
}

/// Filters to constrain valid targets for a [`Choice`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetFilter {
    /// Restrict targets to a specific player's objects.
    pub player: Option<PlayerRef>,
    /// Filter cards by their attributes.
    pub card_filter: Option<CardFilter>,
    /// Restrict to specific zones.
    pub zone_filter: Option<Vec<Zone>>,
}

// ─── Cost Requirement ────────────────────────────────────────────────────────

/// Additional costs beyond the card's base cost, required to activate an effect.
///
/// These are paid before the effect enters the chain and cannot be rolled back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostRequirement {
    /// Send cards from the field to the graveyard.
    SendFieldCardToGrave {
        count: u8,
        filter: Option<CardFilter>,
    },
    /// Discard cards from hand.
    DiscardHand { count: u8 },
    /// Send cards from the cost zone to the graveyard.
    SendCostZoneToGrave {
        count: u8,
        filter: Option<CardFilter>,
    },
}

// ─── Modifiers ───────────────────────────────────────────────────────────────

/// A modifier that alters a card's behavior or stats.
///
/// Applied via [`Action::ApplyModifier`] and tracked as [`AppliedModifier`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Modifier {
    /// Increase or decrease attack power.
    AttackBoost(i16),
    /// Immune to effects from a specific card type.
    ImmuneToCardType(CardType),
    /// Immune to effects from cards with a specific property.
    ImmuneToProperty(Property),
    /// Cannot be destroyed by card effects.
    ImmuneToDestruction,
    /// Cannot be targeted by card effects.
    ImmuneToTargeting,
    /// Can attack the opponent directly even when they have front-field cards.
    CanAttackDirectly,
    /// Gains additional attacks per turn.
    ExtraAttackPerTurn(u8),
    /// Cannot declare attacks.
    CannotAttack,
    /// A custom modifier identified by key, handled by special runtime logic.
    Special { key: String },
}

/// A modifier instance that has been applied to a card, with source tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedModifier {
    /// The modifier effect.
    pub modifier: Modifier,
    /// The card instance that applied this modifier (for removal/tracking).
    pub source: InstanceId,
    /// How long this modifier lasts.
    pub duration: ModifierDuration,
}

/// How long a [`Modifier`] persists once applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModifierDuration {
    /// Lasts as long as the source card remains on the field.
    WhileSourceOnField,
    /// Expires at the end of the current turn.
    UntilEndOfTurn,
    /// Never expires (until explicitly removed).
    Permanent,
    /// Expires after a set number of turns.
    TurnCount(u32),
}
