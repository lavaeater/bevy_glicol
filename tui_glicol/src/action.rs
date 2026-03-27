use serde::{Deserialize, Serialize};
use strum::Display;

use crate::app::Mode;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    UpdateAudioCode(String),
    SpecialAudio,
    PlayAudio,
    StopAudio,
    // Graph actions
    GraphNextNode,
    GraphPrevNode,
    GraphNextCategory,
    GraphPrevCategory,
    GraphAddNode(String), // node_type
    GraphRemoveNode,
    GraphConnectNodes(String, String), // from_id, to_id
    GraphEditParam(String, usize, String), // node_id, param_index, value
    GraphStartEditing,
    GraphStopEditing,
    GraphNextParam,
    GraphPrevParam,
    GraphStartParamInput,
    GraphInputChar(char),
    GraphInputBackspace,
    GraphConfirmParam,
    GraphCancelParam,
    GraphShowError(String),
    GraphClearError,
    SwitchMode(Mode),
}
