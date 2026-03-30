//! Field cursor state management for TUI navigation.
//! Pure data layer — no ratatui dependency.

/// Represents a position on the game field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldZone {
    OpponentBack(usize),     // 对手后场 slot 0-4
    OpponentFront(usize),    // 对手前场 slot 0-4
    MyFront(usize),          // 己方前场 slot 0-4
    MyBack(usize),           // 己方后场 slot 0-4
    MyCostZone(usize),       // 己方费用区 slot 0-5
    MyHand(usize),           // 己方手卡 index
    OpponentCostZone(usize), // 对手费用区
}

/// Cursor state for navigating the game field.
#[derive(Debug, Clone)]
pub struct FieldCursor {
    pub zone: FieldZone,
    pub active: bool,
}

impl Default for FieldCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldCursor {
    pub fn new() -> Self {
        Self {
            zone: FieldZone::MyFront(0),
            active: false,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn jump_to(&mut self, zone: FieldZone) {
        self.zone = zone;
    }

    pub fn current_zone(&self) -> &FieldZone {
        &self.zone
    }

    pub fn slot_index(&self) -> usize {
        match &self.zone {
            FieldZone::OpponentBack(i)
            | FieldZone::OpponentFront(i)
            | FieldZone::MyFront(i)
            | FieldZone::MyBack(i)
            | FieldZone::MyCostZone(i)
            | FieldZone::MyHand(i)
            | FieldZone::OpponentCostZone(i) => *i,
        }
    }

    fn zone_max(&self) -> usize {
        match &self.zone {
            FieldZone::MyCostZone(_) | FieldZone::OpponentCostZone(_) => 5,
            FieldZone::MyHand(_) => 19,
            _ => 4,
        }
    }

    pub fn move_right(&mut self) {
        let max = self.zone_max();
        let new_idx = (self.slot_index() + 1) % (max + 1);
        self.zone = self.with_index(new_idx);
    }

    pub fn move_left(&mut self) {
        let max = self.zone_max();
        let idx = self.slot_index();
        let new_idx = if idx == 0 { max } else { idx - 1 };
        self.zone = self.with_index(new_idx);
    }

    fn with_index(&self, idx: usize) -> FieldZone {
        match &self.zone {
            FieldZone::OpponentBack(_) => FieldZone::OpponentBack(idx),
            FieldZone::OpponentFront(_) => FieldZone::OpponentFront(idx),
            FieldZone::MyFront(_) => FieldZone::MyFront(idx),
            FieldZone::MyBack(_) => FieldZone::MyBack(idx),
            FieldZone::MyCostZone(_) => FieldZone::MyCostZone(idx),
            FieldZone::MyHand(_) => FieldZone::MyHand(idx),
            FieldZone::OpponentCostZone(_) => FieldZone::OpponentCostZone(idx),
        }
    }

    pub fn move_up(&mut self) {
        let idx = self.slot_index();
        self.zone = match &self.zone {
            FieldZone::MyHand(_) => FieldZone::MyCostZone(idx.min(5)),
            FieldZone::MyCostZone(_) => FieldZone::MyBack(idx.min(4)),
            FieldZone::MyBack(_) => FieldZone::MyFront(idx),
            FieldZone::MyFront(_) => FieldZone::OpponentFront(idx),
            FieldZone::OpponentFront(_) => FieldZone::OpponentBack(idx),
            FieldZone::OpponentBack(_) => FieldZone::OpponentCostZone(idx.min(5)),
            FieldZone::OpponentCostZone(_) => FieldZone::MyHand(idx.min(19)),
        };
    }

    pub fn move_down(&mut self) {
        let idx = self.slot_index();
        self.zone = match &self.zone {
            FieldZone::OpponentBack(_) => FieldZone::OpponentFront(idx),
            FieldZone::OpponentFront(_) => FieldZone::MyFront(idx),
            FieldZone::MyFront(_) => FieldZone::MyBack(idx),
            FieldZone::MyBack(_) => FieldZone::MyCostZone(idx.min(5)),
            FieldZone::MyCostZone(_) => FieldZone::MyHand(idx.min(19)),
            FieldZone::MyHand(_) => FieldZone::OpponentCostZone(idx.min(5)),
            FieldZone::OpponentCostZone(_) => FieldZone::OpponentBack(idx.min(4)),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default_position() {
        let c = FieldCursor::new();
        assert_eq!(c.zone, FieldZone::MyFront(0));
        assert!(!c.active);
    }

    #[test]
    fn test_move_right_increments_index() {
        let mut c = FieldCursor::new(); // MyFront(0)
        c.move_right();
        assert_eq!(c.zone, FieldZone::MyFront(1));
    }

    #[test]
    fn test_move_left_wraps_at_zero() {
        let mut c = FieldCursor::new(); // MyFront(0)
        c.move_left();
        assert_eq!(c.zone, FieldZone::MyFront(4));
    }

    #[test]
    fn test_move_right_wraps_at_max() {
        let mut c = FieldCursor {
            zone: FieldZone::MyFront(4),
            active: false,
        };
        c.move_right();
        assert_eq!(c.zone, FieldZone::MyFront(0));
    }

    #[test]
    fn test_move_up_from_my_front() {
        let mut c = FieldCursor::new(); // MyFront(0)
        c.move_up();
        assert_eq!(c.zone, FieldZone::OpponentFront(0));
    }

    #[test]
    fn test_move_down_from_my_front() {
        let mut c = FieldCursor::new(); // MyFront(0)
        c.move_down();
        assert_eq!(c.zone, FieldZone::MyBack(0));
    }

    #[test]
    fn test_jump_to() {
        let mut c = FieldCursor::new();
        c.jump_to(FieldZone::MyHand(3));
        assert_eq!(c.zone, FieldZone::MyHand(3));
    }

    #[test]
    fn test_activate_deactivate() {
        let mut c = FieldCursor::new();
        c.activate();
        assert!(c.active);
        c.deactivate();
        assert!(!c.active);
    }
}
