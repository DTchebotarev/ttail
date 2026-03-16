#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Tab,
    Enter,
    Backspace,
    Esc,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub ctrl: bool,
}

pub enum Event {
    /// A complete line of text (pipe mode).
    Line(String),
    /// Raw bytes from pty master (pty mode). Event loop parses lines + buffers.
    PtyOutput(Vec<u8>),
    /// A key event from the real terminal.
    Key(KeyEvent),
    /// Input source has closed (pipe EOF or child exited).
    InputDone,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Collapsed,
    Expanded,
}
