use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionInput {
    MoveUp,
    MoveDown,
    Confirm,
    Escape,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryInput {
    MoveUp,
    MoveDown,
    Toggle,
    Confirm,
    Escape,
    Noop,
}

pub fn map_action_key(key: KeyEvent) -> ActionInput {
    match key.code {
        KeyCode::Up => ActionInput::MoveUp,
        KeyCode::Down => ActionInput::MoveDown,
        KeyCode::Enter => ActionInput::Confirm,
        KeyCode::Esc => ActionInput::Escape,
        _ => ActionInput::Noop,
    }
}

pub fn map_recovery_key(key: KeyEvent) -> RecoveryInput {
    match key.code {
        KeyCode::Up => RecoveryInput::MoveUp,
        KeyCode::Down => RecoveryInput::MoveDown,
        KeyCode::Char(' ') => RecoveryInput::Toggle,
        KeyCode::Enter => RecoveryInput::Confirm,
        KeyCode::Esc => RecoveryInput::Escape,
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
