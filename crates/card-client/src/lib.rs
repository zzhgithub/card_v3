// card-client: client API trait

pub mod api;
pub mod error;

pub use api::{
    game_state_to_visible, CardOption, CardPublicInfo, ClientApi, OpponentView, TargetOption,
    TestClient, VisibleGameState,
};
pub use error::ClientError;
