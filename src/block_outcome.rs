#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockOutcome {
    Awaited,
    HandlerRemoved,
}
