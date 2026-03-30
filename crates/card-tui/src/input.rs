use crossterm::event::{KeyCode, KeyEvent};

use crate::cursor::FieldZone;

/// Input mode determines how key events are interpreted.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Main menu / selection lists
    Menu,
    /// Field browsing (cursor)
    FieldBrowse,
    /// Action selection overlay
    ActionSelect,
    /// Recovery selection overlay
    RecoverySelect,
    /// Log scrolling
    LogScroll,
    /// Deck editor
    DeckEditor,
}

/// Unified input action across all modes.
/// Replaces the old ActionInput and RecoveryInput enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    // Navigation
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    // Actions
    Confirm,
    Escape,
    Toggle,

    // Field browse specific
    QuickJump(FieldZone),
    MinimizeOverlay,
    ToggleFieldBrowse,
    FocusLog,

    // Global
    Quit,

    // No operation
    Noop,
}

/// Legacy action input for action selection overlays.
/// Kept for backward compatibility - maps to InputAction.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionInput {
    MoveUp,
    MoveDown,
    Confirm,
    Escape,
    Noop,
}

/// Legacy recovery input for recovery selection overlays.
/// Kept for backward compatibility - maps to InputAction.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryInput {
    MoveUp,
    MoveDown,
    Toggle,
    Confirm,
    Escape,
    Noop,
}

/// Unified key mapping function.
/// Maps a key event to an input action based on the current input mode.
pub fn map_key(mode: InputMode, key: KeyEvent) -> InputAction {
    match mode {
        InputMode::Menu => map_menu_key(key),
        InputMode::FieldBrowse => map_field_browse_key(key),
        InputMode::ActionSelect => map_action_select_key(key),
        InputMode::RecoverySelect => map_recovery_select_key(key),
        InputMode::LogScroll => map_log_scroll_key(key),
        InputMode::DeckEditor => map_deck_editor_key(key),
    }
}

fn map_menu_key(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Up => InputAction::MoveUp,
        KeyCode::Down => InputAction::MoveDown,
        KeyCode::Enter => InputAction::Confirm,
        KeyCode::Esc => InputAction::Escape,
        KeyCode::Char('q') | KeyCode::Char('Q') => InputAction::Quit,
        _ => InputAction::Noop,
    }
}

fn map_field_browse_key(key: KeyEvent) -> InputAction {
    match key.code {
        // Directional navigation
        KeyCode::Up => InputAction::MoveUp,
        KeyCode::Down => InputAction::MoveDown,
        KeyCode::Left => InputAction::MoveLeft,
        KeyCode::Right => InputAction::MoveRight,

        // Quick jump to my front slots (1-5)
        KeyCode::Char('1') => InputAction::QuickJump(FieldZone::MyFront(0)),
        KeyCode::Char('2') => InputAction::QuickJump(FieldZone::MyFront(1)),
        KeyCode::Char('3') => InputAction::QuickJump(FieldZone::MyFront(2)),
        KeyCode::Char('4') => InputAction::QuickJump(FieldZone::MyFront(3)),
        KeyCode::Char('5') => InputAction::QuickJump(FieldZone::MyFront(4)),

        // Quick jump to my back slots (!@#$% = Shift+1-5)
        KeyCode::Char('!') => InputAction::QuickJump(FieldZone::MyBack(0)),
        KeyCode::Char('@') => InputAction::QuickJump(FieldZone::MyBack(1)),
        KeyCode::Char('#') => InputAction::QuickJump(FieldZone::MyBack(2)),
        KeyCode::Char('$') => InputAction::QuickJump(FieldZone::MyBack(3)),
        KeyCode::Char('%') => InputAction::QuickJump(FieldZone::MyBack(4)),

        // Quick jump to other zones
        KeyCode::Char('h') | KeyCode::Char('H') => InputAction::QuickJump(FieldZone::MyHand(0)),
        KeyCode::Char('c') | KeyCode::Char('C') => InputAction::QuickJump(FieldZone::MyCostZone(0)),
        KeyCode::Char('f') | KeyCode::Char('F') => {
            InputAction::QuickJump(FieldZone::OpponentFront(0))
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            InputAction::QuickJump(FieldZone::OpponentBack(0))
        }

        // Mode toggles
        KeyCode::Tab => InputAction::ToggleFieldBrowse,
        KeyCode::Char('l') | KeyCode::Char('L') => InputAction::FocusLog,
        KeyCode::Char('m') | KeyCode::Char('M') => InputAction::MinimizeOverlay,

        // Exit
        KeyCode::Esc => InputAction::Escape,

        _ => InputAction::Noop,
    }
}

fn map_action_select_key(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Up => InputAction::MoveUp,
        KeyCode::Down => InputAction::MoveDown,
        KeyCode::Enter => InputAction::Confirm,
        KeyCode::Esc => InputAction::Escape,
        KeyCode::Char('m') | KeyCode::Char('M') => InputAction::MinimizeOverlay,
        _ => InputAction::Noop,
    }
}

fn map_recovery_select_key(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Up => InputAction::MoveUp,
        KeyCode::Down => InputAction::MoveDown,
        KeyCode::Char(' ') => InputAction::Toggle,
        KeyCode::Enter => InputAction::Confirm,
        KeyCode::Esc => InputAction::Escape,
        KeyCode::Char('m') | KeyCode::Char('M') => InputAction::MinimizeOverlay,
        _ => InputAction::Noop,
    }
}

fn map_log_scroll_key(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Up => InputAction::MoveUp,
        KeyCode::Down => InputAction::MoveDown,
        KeyCode::Esc => InputAction::Escape,
        _ => InputAction::Noop,
    }
}

fn map_deck_editor_key(key: KeyEvent) -> InputAction {
    match key.code {
        KeyCode::Up => InputAction::MoveUp,
        KeyCode::Down => InputAction::MoveDown,
        KeyCode::Tab => InputAction::Toggle,
        KeyCode::Char('a') | KeyCode::Char('A') => InputAction::Confirm,
        KeyCode::Char('r') | KeyCode::Char('R') => InputAction::Toggle,
        KeyCode::Char('s') | KeyCode::Char('S') => InputAction::Confirm,
        KeyCode::Esc => InputAction::Escape,
        _ => InputAction::Noop,
    }
}

/// Maps a key event to an action input for action selection overlays.
#[allow(dead_code)]
pub fn map_action_key(key: KeyEvent) -> ActionInput {
    match map_key(InputMode::ActionSelect, key) {
        InputAction::MoveUp => ActionInput::MoveUp,
        InputAction::MoveDown => ActionInput::MoveDown,
        InputAction::Confirm => ActionInput::Confirm,
        InputAction::Escape => ActionInput::Escape,
        _ => ActionInput::Noop,
    }
}

/// Maps a key event to a recovery input for recovery selection overlays.
#[allow(dead_code)]
pub fn map_recovery_key(key: KeyEvent) -> RecoveryInput {
    match map_key(InputMode::RecoverySelect, key) {
        InputAction::MoveUp => RecoveryInput::MoveUp,
        InputAction::MoveDown => RecoveryInput::MoveDown,
        InputAction::Toggle => RecoveryInput::Toggle,
        InputAction::Confirm => RecoveryInput::Confirm,
        InputAction::Escape => RecoveryInput::Escape,
        _ => RecoveryInput::Noop,
    }
}

pub fn move_cursor_up(index: &mut usize, len: usize) {
    if len == 0 {
        *index = 0;
        return;
    }
    if *index == 0 {
        *index = len - 1;
    } else {
        *index -= 1;
    }
}

pub fn move_cursor_down(index: &mut usize, len: usize) {
    if len == 0 {
        *index = 0;
        return;
    }
    *index = (*index + 1) % len;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn test_map_key_menu_mode() {
        assert!(matches!(
            map_key(InputMode::Menu, key(KeyCode::Up)),
            InputAction::MoveUp
        ));
        assert!(matches!(
            map_key(InputMode::Menu, key(KeyCode::Down)),
            InputAction::MoveDown
        ));
        assert!(matches!(
            map_key(InputMode::Menu, key(KeyCode::Enter)),
            InputAction::Confirm
        ));
        assert!(matches!(
            map_key(InputMode::Menu, key(KeyCode::Esc)),
            InputAction::Escape
        ));
        assert!(matches!(
            map_key(InputMode::Menu, key(KeyCode::Char('q'))),
            InputAction::Quit
        ));
    }

    #[test]
    fn test_map_key_field_browse_quick_jumps() {
        // MyFront slots
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('1'))),
            InputAction::QuickJump(FieldZone::MyFront(0))
        ));
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('5'))),
            InputAction::QuickJump(FieldZone::MyFront(4))
        ));

        // MyBack slots with shift
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('!'))),
            InputAction::QuickJump(FieldZone::MyBack(0))
        ));
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('%'))),
            InputAction::QuickJump(FieldZone::MyBack(4))
        ));

        // Other zones
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('h'))),
            InputAction::QuickJump(FieldZone::MyHand(0))
        ));
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('c'))),
            InputAction::QuickJump(FieldZone::MyCostZone(0))
        ));
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('f'))),
            InputAction::QuickJump(FieldZone::OpponentFront(0))
        ));
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('b'))),
            InputAction::QuickJump(FieldZone::OpponentBack(0))
        ));
    }

    #[test]
    fn test_map_key_field_browse_mode_toggles() {
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Tab)),
            InputAction::ToggleFieldBrowse
        ));
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('l'))),
            InputAction::FocusLog
        ));
        assert!(matches!(
            map_key(InputMode::FieldBrowse, key(KeyCode::Char('m'))),
            InputAction::MinimizeOverlay
        ));
    }

    #[test]
    fn test_map_key_action_select() {
        assert!(matches!(
            map_key(InputMode::ActionSelect, key(KeyCode::Up)),
            InputAction::MoveUp
        ));
        assert!(matches!(
            map_key(InputMode::ActionSelect, key(KeyCode::Enter)),
            InputAction::Confirm
        ));
        assert!(matches!(
            map_key(InputMode::ActionSelect, key(KeyCode::Char('m'))),
            InputAction::MinimizeOverlay
        ));
    }

    #[test]
    fn test_map_key_recovery_select() {
        assert!(matches!(
            map_key(InputMode::RecoverySelect, key(KeyCode::Up)),
            InputAction::MoveUp
        ));
        assert!(matches!(
            map_key(InputMode::RecoverySelect, key(KeyCode::Char(' '))),
            InputAction::Toggle
        ));
        assert!(matches!(
            map_key(InputMode::RecoverySelect, key(KeyCode::Enter)),
            InputAction::Confirm
        ));
        assert!(matches!(
            map_key(InputMode::RecoverySelect, key(KeyCode::Char('m'))),
            InputAction::MinimizeOverlay
        ));
    }

    #[test]
    fn test_legacy_map_action_key() {
        assert!(matches!(
            map_action_key(key(KeyCode::Up)),
            ActionInput::MoveUp
        ));
        assert!(matches!(
            map_action_key(key(KeyCode::Down)),
            ActionInput::MoveDown
        ));
        assert!(matches!(
            map_action_key(key(KeyCode::Enter)),
            ActionInput::Confirm
        ));
        assert!(matches!(
            map_action_key(key(KeyCode::Esc)),
            ActionInput::Escape
        ));
    }

    #[test]
    fn test_legacy_map_recovery_key() {
        assert!(matches!(
            map_recovery_key(key(KeyCode::Up)),
            RecoveryInput::MoveUp
        ));
        assert!(matches!(
            map_recovery_key(key(KeyCode::Down)),
            RecoveryInput::MoveDown
        ));
        assert!(matches!(
            map_recovery_key(key(KeyCode::Char(' '))),
            RecoveryInput::Toggle
        ));
        assert!(matches!(
            map_recovery_key(key(KeyCode::Enter)),
            RecoveryInput::Confirm
        ));
        assert!(matches!(
            map_recovery_key(key(KeyCode::Esc)),
            RecoveryInput::Escape
        ));
    }

    #[test]
    fn test_move_cursor_up() {
        let mut idx = 3;
        move_cursor_up(&mut idx, 5);
        assert_eq!(idx, 2);

        idx = 0;
        move_cursor_up(&mut idx, 5);
        assert_eq!(idx, 4);

        idx = 0;
        move_cursor_up(&mut idx, 0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_move_cursor_down() {
        let mut idx = 2;
        move_cursor_down(&mut idx, 5);
        assert_eq!(idx, 3);

        idx = 4;
        move_cursor_down(&mut idx, 5);
        assert_eq!(idx, 0);

        idx = 0;
        move_cursor_down(&mut idx, 0);
        assert_eq!(idx, 0);
    }
}
