use aper::data_structures::Atom;
use aper::data_structures::List;
use aper::StateMachine;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Instant;
use uuid::Uuid;

use crate::models::ws_board::WsBoard;

pub struct Document {
    pub last_accessed: Instant,
    pub wsboard: Arc<WsBoard>,
}

impl Document {
    pub fn new(wsboard: Arc<WsBoard>) -> Self {
        Self {
            last_accessed: Instant::now(),
            wsboard,
        }
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        self.wsboard.kill();
    }
}

/// The shared state of the server, accessible from within request handlers.
#[derive(Clone)]
pub struct ServerState {
    /// Concurrent map storing in-memory documents.
    pub documents: Arc<DashMap<String, Document>>,
}

/// Statistics about the server, returned from an API endpoint.
#[derive(Serialize, Deserialize)]
pub struct Stats {
    /// System time when the server started, in seconds since Unix epoch.
    pub start_time: u64,
    /// Number of documents currently tracked by the server.
    pub num_documents: usize,
}

/// Represents a document persisted in database storage.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PersistedDocument {
    pub sheets: Vec<Sheet>,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sheet {
    pub number: u64,
    pub name: String,
}

#[derive(StateMachine, Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SheetInfo {
    pub number: Atom<u64>,
    pub name: Atom<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub position: (u32, u32, u32),
    pub cell_type: String,
    pub content: String,
}

#[derive(StateMachine, Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CellInfo {
    pub position: Atom<(u32, u32, u32)>,
    pub cell_type: Atom<String>,
    pub content: Atom<String>,
}

/// Shared state involving multiple users, protected by a lock.
pub struct State {
    pub sheets: List<SheetInfo>,
    pub cells: List<CellInfo>,
    pub sheets_ids: HashMap<u64, Uuid>,
    pub cell_ids: HashMap<(u32, u32, u32), Uuid>,
    pub users: HashMap<u64, UserInfo>,
    pub cursors: HashMap<u64, CursorData>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sheets: List::default(),
            sheets_ids: Default::default(),
            cells: List::default(),
            cell_ids: Default::default(),
            users: Default::default(),
            cursors: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub user_id: String,
    pub hue: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorData {
    pub cursors: (u32, u32, u32),
}

/// A message received from the client over WebSocket.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    ClientInfo(UserInfo),
    CursorData(CursorData),
    CreateSheet(Sheet),
    DeleteSheet(Sheet),
    RenameSheet(Sheet),
    WriteCell(Cell),
    DeleteCell(CursorData),
}

/// A message sent to the client over WebSocket.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMsg {
    /// Informs the client of their unique socket ID.
    Identity(u64),
    /// Broadcasts a user's information, or `None` on disconnect.
    UserInfo {
        id: u64,
        info: Option<UserInfo>,
    },
    /// Broadcasts a user's cursor position.
    UserCursor {
        id: u64,
        data: CursorData,
    },
    /// Broadcasts sheet.
    CreateSheet {
        id: u64,
        sheet: Sheet,
    },
    DeleteSheet {
        id: u64,
        sheet: Sheet,
    },
    RenameSheet {
        id: u64,
        sheet: Sheet,
    },
    GetSheet {
        sheet: Sheet,
    },
    WriteCell {
        id: u64,
        cell: Cell,
    },
    DeleteCell {
        id: u64,
        cursor: CursorData,
    },
    GetCell {
        cell: Cell,
    },
}
