//! Connection state machine for tunnel lifecycle

use crate::types::TunnelState;
use tokio::sync::watch;

/// State machine for connection lifecycle
///
/// Manages QUIC tunnel connection states with validated transitions.
/// Uses watch channels for efficient state change broadcasting.
///
/// # States
///
/// - `Disconnected`: Initial state, no connection
/// - `Connecting`: Attempting to establish connection
/// - `Connected`: Connection established and healthy
/// - `Reconnecting`: Attempting to reconnect after failure
/// - `Failed`: Connection failed, awaiting retry
///
/// # Thread Safety
///
/// This struct is clone-safe and can be shared across threads.
/// Each clone gets its own receiver for state changes.
///
/// # Example
///
/// ```rust,no_run
/// use quicunnel::state::ConnectionStateMachine;
/// use quicunnel::types::TunnelState;
///
/// let sm = ConnectionStateMachine::new();
/// assert!(matches!(sm.current(), TunnelState::Disconnected));
///
/// // Transition to connecting
/// sm.transition(TunnelState::Connecting {
///     since: std::time::Instant::now(),
/// });
/// ```
pub struct ConnectionStateMachine {
    state: watch::Sender<TunnelState>,
    state_rx: watch::Receiver<TunnelState>,
}

impl ConnectionStateMachine {
    /// Create a new state machine in Disconnected state
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(TunnelState::Disconnected);
        Self {
            state: tx,
            state_rx: rx,
        }
    }

    /// Transition to a new state
    ///
    /// Validates that the transition is legal before applying it
    pub fn transition(&self, new_state: TunnelState) {
        let old_state = self.state_rx.borrow().clone();

        // Validate transition
        let valid = match (&old_state, &new_state) {
            (TunnelState::Disconnected, TunnelState::Connecting { .. }) => true,
            (TunnelState::Connecting { .. }, TunnelState::Connected { .. }) => true,
            (TunnelState::Connecting { .. }, TunnelState::Failed { .. }) => true,
            (TunnelState::Connected { .. }, TunnelState::Reconnecting { .. }) => true,
            (TunnelState::Connected { .. }, TunnelState::Disconnected) => true,
            (TunnelState::Reconnecting { .. }, TunnelState::Connected { .. }) => true,
            (TunnelState::Reconnecting { attempt: a1, .. }, TunnelState::Reconnecting { attempt: a2, .. })
                if *a2 == *a1 + 1 => true,
            (TunnelState::Reconnecting { .. }, TunnelState::Failed { .. }) => true,
            (TunnelState::Failed { .. }, TunnelState::Connecting { .. }) => true,
            (TunnelState::Disconnected, TunnelState::Disconnected) => true,
            _ => false,
        };

        if valid {
            tracing::debug!("State transition: {:?} -> {:?}", old_state, new_state);
            let _ = self.state.send(new_state);
        } else {
            tracing::warn!(
                "Invalid state transition attempted: {:?} -> {:?}",
                old_state,
                new_state
            );
        }
    }

    /// Get current state
    pub fn current(&self) -> TunnelState {
        self.state_rx.borrow().clone()
    }

    /// Subscribe to state changes
    pub fn subscribe(&self) -> watch::Receiver<TunnelState> {
        self.state_rx.clone()
    }
}

impl Default for ConnectionStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_state_machine_creation() {
        let sm = ConnectionStateMachine::new();
        assert!(matches!(sm.current(), TunnelState::Disconnected));
    }

    #[test]
    fn test_valid_transitions() {
        let sm = ConnectionStateMachine::new();

        // Disconnected -> Connecting
        sm.transition(TunnelState::Connecting {
            since: Instant::now(),
        });
        assert!(matches!(sm.current(), TunnelState::Connecting { .. }));

        // Connecting -> Connected
        sm.transition(TunnelState::Connected {
            since: Instant::now(),
            latency_ms: 50,
        });
        assert!(matches!(sm.current(), TunnelState::Connected { .. }));
    }

    #[test]
    fn test_invalid_transition() {
        let sm = ConnectionStateMachine::new();

        // Disconnected -> Connected (invalid, must go through Connecting first)
        let current = sm.current();
        sm.transition(TunnelState::Connected {
            since: Instant::now(),
            latency_ms: 50,
        });

        // State should not have changed
        assert_eq!(sm.current(), current);
    }

    #[test]
    fn test_reconnect_transition() {
        let sm = ConnectionStateMachine::new();

        // Disconnected -> Connecting -> Connected -> Reconnecting (attempt 1)
        sm.transition(TunnelState::Connecting {
            since: Instant::now(),
        });
        sm.transition(TunnelState::Connected {
            since: Instant::now(),
            latency_ms: 50,
        });
        sm.transition(TunnelState::Reconnecting {
            attempt: 1,
            last_error: "Connection reset".to_string(),
        });

        assert!(matches!(
            sm.current(),
            TunnelState::Reconnecting { attempt: 1, .. }
        ));

        // Reconnecting -> Reconnecting (attempt 2, valid)
        sm.transition(TunnelState::Reconnecting {
            attempt: 2,
            last_error: "Connection reset".to_string(),
        });

        assert!(matches!(
            sm.current(),
            TunnelState::Reconnecting { attempt: 2, .. }
        ));
    }

    #[test]
    fn test_subscribe() {
        let sm = ConnectionStateMachine::new();
        let mut rx = sm.subscribe();

        // Initial state
        assert!(matches!(*rx.borrow_and_update(), TunnelState::Disconnected));

        // Transition
        sm.transition(TunnelState::Connecting {
            since: Instant::now(),
        });

        // Subscriber should see new state
        assert!(matches!(*rx.borrow_and_update(), TunnelState::Connecting { .. }));
    }
}
