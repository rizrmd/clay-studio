use salvo::prelude::*;
use crate::state::AppState;

pub mod auth;

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