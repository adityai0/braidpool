use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::error::{DBErrors, StratumErrors};

/// Identifies the component that originated a [`Status`] update.
///
/// Each variant contains a channel to the main coordinator, and optionally a component ID
/// (e.g. a downstream connection ID or peer ID).
///This will basically sense the originator of error and notify further
#[derive(Debug, Clone)]
pub enum StatusSender {
    /// A specific downstream miner connection in the Stratum server.
    DownstreamMiner {
        connection_id: u32,
        peer_addr: SocketAddr,
        tx: mpsc::UnboundedSender<Status>,
    },
    /// The Stratum server listener.
    StratumServer(mpsc::UnboundedSender<Status>),
    /// The Stratum notifier task (mining.notify).
    StratumNotifier(mpsc::UnboundedSender<Status>),
    /// The P2P network swarm handler.
    SwarmHandler(mpsc::UnboundedSender<Status>),
    /// The peer manager subsystem.
    PeerManager(mpsc::UnboundedSender<Status>),
    /// The IBD (Initial Block Download) manager.
    IBDManager(mpsc::UnboundedSender<Status>),
    /// The database query handler.
    DatabaseHandler(mpsc::UnboundedSender<Status>),
    //IPC error handler
    IPCHandler(mpsc::UnboundedSender<Status>),
    // /// The IPC (Inter-Process Communication) listener for Bitcoin Core.
    // IPCListener(mpsc::UnboundedSender<Status>),
    // /// The IPC template consumer.
    // IPCConsumer(mpsc::UnboundedSender<Status>),
    /// The RPC server.
    RPCServer(mpsc::UnboundedSender<Status>),
}
//Sending the status received to the main task that will further trigger the corresponding notification of shutdown
impl StatusSender {
    /// Sends a [`Status`] update to the main coordinator.
    ///
    /// Returns an error if the channel is closed (main loop has exited).
    pub async fn send(&self, status: Status) -> Result<(), mpsc::error::SendError<Status>> {
        match self {
            Self::DownstreamMiner {
                connection_id,
                peer_addr,
                tx,
            } => {
                debug!(
                    connection_id = connection_id,
                    peer = %peer_addr,
                    state = ?status.state,
                    "Sending status from DownstreamMiner"
                );
                tx.send(status)
            }
            Self::StratumServer(tx) => {
                debug!(state = ?status.state, "Sending status from StratumServer");
                tx.send(status)
            }
            Self::StratumNotifier(tx) => {
                debug!(state = ?status.state, "Sending status from StratumNotifier");
                tx.send(status)
            }
            Self::SwarmHandler(tx) => {
                debug!(state = ?status.state, "Sending status from SwarmHandler");
                tx.send(status)
            }
            Self::PeerManager(tx) => {
                debug!(state = ?status.state, "Sending status from PeerManager");
                tx.send(status)
            }
            Self::IBDManager(tx) => {
                debug!(state = ?status.state, "Sending status from IBDManager");
                tx.send(status)
            }
            Self::DatabaseHandler(tx) => {
                debug!(state = ?status.state, "Sending status from DatabaseHandler");
                tx.send(status)
            }
            // Self::IPCListener(tx) => {
            //     debug!(state = ?status.state, "Sending status from IPCListener");
            //     tx.send(status)
            // }
            // Self::IPCConsumer(tx) => {
            //     debug!(state = ?status.state, "Sending status from IPCConsumer");
            //     tx.send(status)
            // }
            Self::IPCHandler(tx) => {
                debug!(state = ?status.state, "Sending status from IPCHandler");
                tx.send(status)
            }
            Self::RPCServer(tx) => {
                debug!(state = ?status.state, "Sending status from RPCServer");
                tx.send(status)
            }
        }
    }

    /// Returns a human-readable name for this sender component.
    pub fn component_name(&self) -> &'static str {
        match self {
            Self::DownstreamMiner { .. } => "DownstreamMiner",
            Self::StratumServer(_) => "StratumServer",
            Self::StratumNotifier(_) => "StratumNotifier",
            Self::SwarmHandler(_) => "SwarmHandler",
            Self::PeerManager(_) => "PeerManager",
            Self::IBDManager(_) => "IBDManager",
            Self::DatabaseHandler(_) => "DatabaseHandler",
            Self::RPCServer(_) => "RPCServer",
            Self::IPCHandler(_) => "IPCHandler",
        }
    }
}

/// The type of event or error being reported by a component.
#[derive(Debug, Clone)]
pub enum State {
    /// Component encountered a non-fatal error.
    Error {
        component: String,
        error: TaskError,
        timestamp: SystemTime,
    },
    /// Downstream miner disconnected or encountered an error.
    DownstreamShutdown {
        connection_id: u32,
        peer_addr: SocketAddr,
        reason: TaskError,
    },
    /// Stratum server listener shut down.
    StratumServerShutdown { reason: TaskError },
    /// Stratum notifier shut down.
    StratumNotifierShutdown { reason: TaskError },
    /// P2P swarm handler shut down.
    SwarmShutdown { reason: TaskError },
    /// Peer manager shut down.
    PeerManagerShutdown { reason: TaskError },
    /// IBD manager shut down.
    IBDManagerShutdown { reason: TaskError },
    /// Database handler shut down.
    DatabaseShutdown { reason: TaskError },
    /// RPC server shut down.
    RPCServerShutdown { reason: TaskError },
    /// IPC error occurred.
    IPCTaskShutdown { reason: TaskError },
}

/// Categories of errors that tasks can report.
#[derive(Debug, Clone)]
pub enum TaskError {
    /// A downstream miner connection error this will not be specific to all.
    DownstreamDisconnected {
        connection_id: u32,
        peer_addr: SocketAddr,
        details: String,
    },
    /// Stratum server error rigid error (shutdownAll).
    StratumServerError {
        error: String,
    },
    /// Database operation error rigid error (shutdownAll).
    DatabaseError {
        error: String,
    },
    /// P2P network error.
    NetworkError {
        details: String,
    },
    /// IPC communication error with Bitcoin Core rigid error (shutdownAll).
    IPCError {
        details: String,
    },
    /// IBD (Initial Block Download) error.
    IBDError {
        details: String,
    },
    /// RPC server error .
    RPCError {
        details: String,
    },
    //Notifier error
    StratumNotifierError {
        details: String,
    },
    //Peer manager error
    PeerManagerError {
        details: String,
    },
}
//Since we have different error types we will have to have functions in place to convert them into TaskError
//Which will then be wrapped in Status and sent to main task and the notifiying tasks on the basis of Status received
impl TaskError {
    /// Creates a `TaskError` from a `StratumErrors`.
    pub fn from_stratum_error(error: StratumErrors) -> Self {
        Self::StratumServerError {
            error: error.to_string(),
        }
    }

    /// Creates a `TaskError` from a `DBErrors`.
    pub fn from_db_error(error: DBErrors) -> Self {
        Self::DatabaseError {
            error: error.to_string(),
        }
    }
}

/// A message reporting the current [`State`] of a component.
#[derive(Debug, Clone)]
pub struct Status {
    pub state: State,
}
/// Represents a message that can trigger shutdown of various system components.
#[derive(Debug, Clone)]
pub enum ShutdownMessage {
    /// Shutdown all components immediately
    ShutdownAll,
    /// Shutdown all downstream connections
    DownstreamShutdownAll,
    /// Shutdown a specific downstream connection by ID
    DownstreamShutdown(String),
    ///Fallback in case of status sender reporting an error can be resumed
    ComponentFallback,
}

/// Action to take in response to an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// Disconnect the specific downstream connection.
    DisconnectDownstream,
    /// Attempt to reconnect or fallback to alternative.
    Fallback,
    /// Initiate system-wide shutdown.
    Shutdown,
}
/// Determines the appropriate action for a given task error.
pub fn determine_action(error: &TaskError) -> ErrorAction {
    match error {
        //Only miner specific error for cleaning is required
        TaskError::DownstreamDisconnected { .. } => ErrorAction::DisconnectDownstream,
        //Complete shutdown is required
        TaskError::DatabaseError { .. } => ErrorAction::Shutdown,
        TaskError::IPCError { .. } => ErrorAction::Shutdown,
        TaskError::StratumServerError { .. } => ErrorAction::Shutdown,
        TaskError::StratumNotifierError { .. } => ErrorAction::Shutdown,
        //Fallback or some local shutdown of task maybe required definitely not complete shutdown though
        TaskError::NetworkError { .. } => ErrorAction::Fallback,
        TaskError::IBDError { .. } => ErrorAction::Fallback,
        TaskError::RPCError { .. } => ErrorAction::Fallback,
        TaskError::PeerManagerError { .. } => ErrorAction::Fallback,
    }
}

/// Sends a status update based on the error and determined action.
///
/// Returns `true` if the error requires component shutdown, `false` otherwise.
async fn send_status(sender: &StatusSender, error: TaskError) -> bool {
    let action = determine_action(&error);

    match action {
        ErrorAction::DisconnectDownstream => {
            if let TaskError::DownstreamDisconnected {
                connection_id,
                peer_addr,
                details,
            } = error
            {
                let state = State::DownstreamShutdown {
                    connection_id,
                    peer_addr,
                    reason: TaskError::DownstreamDisconnected {
                        connection_id,
                        peer_addr,
                        details,
                    },
                };

                if let Err(e) = sender.send(Status { state }).await {
                    error!(
                        "Failed to send downstream shutdown status from {:?}: {:?}",
                        sender, e
                    );
                }
                matches!(sender, StatusSender::DownstreamMiner { .. })
                    || matches!(sender, StatusSender::StratumNotifier { .. })
            } else {
                warn!(
                    "DisconnectDownstream action for non-downstream error: {:?}",
                    error
                );
                false
            }
        }

        ErrorAction::Fallback => {
            warn!(
                "Fallback action triggered from {:?} due to error: {:?}",
                sender, error
            );
            let state = match sender {
                StatusSender::SwarmHandler(_) => State::SwarmShutdown { reason: error },
                StatusSender::PeerManager(_) => State::PeerManagerShutdown { reason: error },
                StatusSender::IBDManager(_) => State::IBDManagerShutdown { reason: error },
                StatusSender::RPCServer(_) => State::RPCServerShutdown { reason: error },
                _ => State::Error {
                    component: sender.component_name().to_string(),
                    error,
                    timestamp: SystemTime::now(),
                },
            };

            if let Err(e) = sender.send(Status { state }).await {
                error!("Failed to send fallback status from {:?}: {:?}", sender, e);
            }
            false
        }

        ErrorAction::Shutdown => {
            error!(
                "Shutdown action triggered from {:?} due to critical error: {:?}",
                sender, error
            );

            let state = match sender {
                StatusSender::StratumServer(_) => State::StratumServerShutdown { reason: error },
                StatusSender::StratumNotifier(_) => {
                    State::StratumNotifierShutdown { reason: error }
                }
                StatusSender::DatabaseHandler(_) => State::DatabaseShutdown { reason: error },
                // StatusSender::IPCListener(_) => State::IPCListenerShutdown { reason: error },
                // StatusSender::IPCConsumer(_) => State::IPCConsumerShutdown { reason: error },
                StatusSender::IPCHandler(_) => State::IPCTaskShutdown { reason: error },
                _ => State::Error {
                    component: sender.component_name().to_string(),
                    error,
                    timestamp: SystemTime::now(),
                },
            };

            if let Err(e) = sender.send(Status { state }).await {
                error!("Failed to send shutdown status from {:?}: {:?}", sender, e);
                // If we can't send shutdown signal, abort immediately
                std::process::abort();
            }
            true
        }
    }
}
pub async fn handle_error(sender: &StatusSender, error: TaskError) -> bool {
    send_status(sender, error).await
}
