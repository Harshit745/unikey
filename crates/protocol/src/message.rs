#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Hello = 1,

    Ping = 2,

    Pong = 3,

    PairRequest = 4,

    PairResponse = 5,

    Clipboard = 6,

    FileMetadata = 7,

    FileChunk = 8,

    Disconnect = 9,

    /// Keyboard event forwarding (key press/release, modifiers).
    InputKeyEvent = 10,

    /// Mouse event forwarding (move, click, scroll, drag).
    InputMouseEvent = 11,

    /// Receiver → sender: a file chunk (or the whole file) verified OK.
    FileAck = 12,

    /// Either side → other: abort an in-flight file transfer.
    FileCancel = 13,

    /// Request / grant of cursor ownership for universal control.
    InputGrab = 14,

    /// Release of cursor ownership back to the current host.
    InputRelease = 15,
}

impl TryFrom<u8> for MessageType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Ping),
            3 => Ok(Self::Pong),
            4 => Ok(Self::PairRequest),
            5 => Ok(Self::PairResponse),
            6 => Ok(Self::Clipboard),
            7 => Ok(Self::FileMetadata),
            8 => Ok(Self::FileChunk),
            9 => Ok(Self::Disconnect),
            10 => Ok(Self::InputKeyEvent),
            11 => Ok(Self::InputMouseEvent),
            12 => Ok(Self::FileAck),
            13 => Ok(Self::FileCancel),
            14 => Ok(Self::InputGrab),
            15 => Ok(Self::InputRelease),
            _ => Err(()),
        }
    }
}
