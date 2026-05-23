/// Errors from building or mutating a high-level dock layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    EmptyLayout,
    DuplicatePanelId(String),
    DuplicatePaneName(String),
    UnknownActivePanel {
        pane: String,
        panel: String,
    },
    InvalidWeights {
        expected: usize,
        got: usize,
    },
    UnknownPanel(String),
    UnknownPane(String),
    InvalidTarget,
    OperationFailed(&'static str),
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyLayout => write!(f, "layout tree is empty"),
            Self::DuplicatePanelId(id) => write!(f, "duplicate panel id: {id}"),
            Self::DuplicatePaneName(name) => write!(f, "duplicate pane name: {name}"),
            Self::UnknownActivePanel { pane, panel } => {
                write!(f, "unknown active panel {panel} in pane {pane}")
            }
            Self::InvalidWeights { expected, got } => {
                write!(f, "expected {expected} weights, got {got}")
            }
            Self::UnknownPanel(id) => write!(f, "unknown panel: {id}"),
            Self::UnknownPane(name) => write!(f, "unknown pane: {name}"),
            Self::InvalidTarget => write!(f, "invalid pane target"),
            Self::OperationFailed(op) => write!(f, "operation failed: {op}"),
        }
    }
}

impl std::error::Error for LayoutError {}
