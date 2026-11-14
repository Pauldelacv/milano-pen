//! Business logic. Functions take a connection (usually a transaction) and a
//! [`history::Recorder`]; [`workspace::Workspace`] wires them to transactions
//! and the undo stack.

pub mod backup;
pub mod benchmark;
pub mod boards;
pub mod cards;
pub mod export;
pub mod history;
pub mod search;
pub mod settings;
pub mod text;
pub mod trash;
pub mod workspace;
