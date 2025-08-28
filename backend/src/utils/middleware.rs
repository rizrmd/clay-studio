use salvo::prelude::*;
use crate::utils::AppState;

// Auth utilities are in utils/auth.rs, re-export them
pub use crate::utils::auth::{self, client_scoped, get_current_client_id, is_current_user_root};

pub struct StateInjector {
    state: AppState,
}

impl StateInjector {
    pub fn new(state: AppState) -> Self {
        StateInjector { state }
    }
}

#[async_trait]
impl Handler for StateInjector {
    async fn handle(&self, _req: &mut Request, depot: &mut Depot, _res: &mut Response, _ctrl: &mut FlowCtrl) {
        depot.inject(self.state.clone());
    }
}

pub fn inject_state(state: AppState) -> StateInjector {
    StateInjector::new(state)
}