use anyhow::{Context, Result};
use aper::data_structures::Atom;
use aper::StateMachine;
use axum::extract::ws::{Message, WebSocket};
use futures_util::StreamExt;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{broadcast, Notify};

use crate::models::ws_types::{
    Cell, CellInfo, ClientMsg, PersistedDocument, ServerMsg, Sheet, SheetInfo, State,
};

/// The main object representing a collaborative session.
pub struct WsBoard {
    /// State modified by critical sections of the code.
    state: RwLock<State>,
    /// Incremented to obtain unique user IDs.
    count: AtomicU64,
    /// Used to notify clients of new operations.
    notify: Notify,
    /// Used to inform all clients of metadata updates.
    update: broadcast::Sender<ServerMsg>,
    /// Set to true when the document is destroyed.
    killed: AtomicBool,
}

impl Default for WsBoard {
    fn default() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            state: Default::default(),
            count: Default::default(),
            notify: Default::default(),
            update: tx,
            killed: AtomicBool::new(false),
        }
    }
}

impl From<ServerMsg> for Message {
    fn from(msg: ServerMsg) -> Self {
        let serialized = serde_json::to_string(&msg).expect("failed serialize");
        Message::Text(serialized)
    }
}

impl WsBoard {
    /// Handle a connection from a WebSocket.
    pub async fn on_connection(&self, socket: WebSocket) {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        println!("connection! id = {}", id);
        if let Err(e) = self.handle_connection(id, socket).await {
            println!("connection terminated early: {}", e);
        }
        println!("disconnection, id = {}", id);
        self.state.write().users.remove(&id);
        self.state.write().cursors.remove(&id);
        self.update
            .send(ServerMsg::UserInfo { id, info: None })
            .ok();
    }

    pub fn get_persist(&self) -> PersistedDocument {
        let mut sheets: Vec<Sheet> = Vec::new();
        let state = self.state.read();
        for sheet in state.sheets.iter() {
            sheets.push(Sheet {
                number: *sheet.value.number.value(),
                name: sheet.value.name.value().to_string(),
            });
        }

        let mut cells: Vec<Cell> = Vec::new();
        for cell in state.cells.iter() {
            cells.push(Cell {
                position: *cell.value.position.value(),
                cell_type: cell.value.cell_type.value().to_string(),
                content: cell.value.content.value().to_string(),
            });
        }

        PersistedDocument { sheets, cells }
    }

    pub fn set_persist(&self, persist: &PersistedDocument) {
        let mut state = self.state.write();
        for sheet in &persist.sheets {
            let (sheet_id, sheet_transition) = state.sheets.append(SheetInfo {
                number: Atom::new(sheet.number),
                name: Atom::new(sheet.name.clone()),
            });

            state.sheets_ids.insert(sheet.number, sheet_id);
            state.sheets.apply(sheet_transition);
        }
        for cell in &persist.cells {
            let (cell_id, cell_transition) = state.cells.append(CellInfo {
                position: Atom::new(cell.position.clone()),
                cell_type: Atom::new(cell.cell_type.clone()),
                content: Atom::new(cell.content.clone()),
            });

            state.cell_ids.insert(cell.position, cell_id);
            state.cells.apply(cell_transition);
        }
    }

    /// Kill this object immediately, dropping all current connections.
    pub fn kill(&self) {
        self.killed.store(true, Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    /// Returns if this WsBoard object has been killed.
    pub fn killed(&self) -> bool {
        self.killed.load(Ordering::Relaxed)
    }

    async fn handle_connection(&self, id: u64, mut socket: WebSocket) -> Result<()> {
        let mut update_rx = self.update.subscribe();

        self.send_initial(id, &mut socket).await?;

        loop {
            // In order to avoid the "lost wakeup" problem, we first request a
            // notification, **then** check the current state for new revisions.
            // This is the same approach that `tokio::sync::watch` takes.
            let notified = self.notify.notified();
            if self.killed() {
                break;
            }

            tokio::select! {
                _ = notified => {}
                update = update_rx.recv() => {
                    socket.send(update?.into()).await?;
                }
                result = socket.next() => {
                    match result {
                        None => break,
                        Some(message) => {
                            self.handle_message(id, message?).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn send_initial(&self, id: u64, socket: &mut WebSocket) -> Result<()> {
        socket.send(ServerMsg::Identity(id).into()).await?;
        let mut messages = Vec::new();
        {
            let state = self.state.read();
            for (&id, info) in &state.users {
                messages.push(ServerMsg::UserInfo {
                    id,
                    info: Some(info.clone()),
                });
            }
            for (&id, data) in &state.cursors {
                messages.push(ServerMsg::UserCursor {
                    id,
                    data: data.clone(),
                });
            }
            for sheet in state.sheets.iter() {
                messages.push(ServerMsg::GetSheet {
                    sheet: Sheet {
                        number: *sheet.value.number.value(),
                        name: sheet.value.name.value().to_string(),
                    },
                });
            }
            for cell in state.cells.iter() {
                messages.push(ServerMsg::GetCell {
                    cell: Cell {
                        position: *cell.value.position.value(),
                        cell_type: cell.value.cell_type.value().to_string(),
                        content: cell.value.content.value().to_string(),
                    },
                });
            }
        }
        for msg in messages {
            socket.send(msg.into()).await?;
        }
        Ok(())
    }

    async fn handle_message(&self, id: u64, message: Message) -> Result<()> {
        if let Message::Text(_message) = message {
            println!("====================={:?}", _message);
            let msg: ClientMsg =
                serde_json::from_str(&_message).context("failed to deserialize message")?;
            match msg {
                ClientMsg::ClientInfo(info) => {
                    self.state.write().users.insert(id, info.clone());
                    let msg = ServerMsg::UserInfo {
                        id,
                        info: Some(info),
                    };
                    self.update.send(msg).ok();
                }
                ClientMsg::CursorData(data) => {
                    self.state.write().cursors.insert(id, data.clone());

                    let msg = ServerMsg::UserCursor { id, data };
                    self.update.send(msg).ok();
                }
                ClientMsg::CreateSheet(sheet) => {
                    let mut _state = self.state.write();
                    let (sheet_id, sheet_transition) = _state.sheets.append(SheetInfo {
                        number: Atom::new(sheet.number),
                        name: Atom::new(sheet.name.clone()),
                    });

                    _state.sheets_ids.insert(sheet.number, sheet_id);
                    _state.sheets.apply(sheet_transition);

                    let msg = ServerMsg::CreateSheet { id, sheet };
                    self.update.send(msg).ok();
                }
                ClientMsg::DeleteSheet(sheet) => {
                    let mut _state = self.state.write();
                    if let Some(sheet_id) = _state.sheets_ids.get(&sheet.number) {
                        let sheet_transition = _state.sheets.delete(*sheet_id);
                        _state.sheets_ids.remove(&sheet.number);
                        _state.sheets.apply(sheet_transition);

                        let msg = ServerMsg::DeleteSheet { id, sheet };
                        self.update.send(msg).ok();
                    }
                }
                ClientMsg::RenameSheet(sheet) => {
                    let mut _state = self.state.write();
                    if let Some(sheet_id) = _state.sheets_ids.get(&sheet.number) {
                        let sheet_transition = _state.sheets.map_item(*sheet_id, |it| {
                            it.map_name(|lbl| lbl.replace(sheet.name.clone()))
                        });

                        _state.sheets.apply(sheet_transition);

                        let msg = ServerMsg::RenameSheet { id, sheet };
                        self.update.send(msg).ok();
                    }
                }
                ClientMsg::WriteCell(cell) => {
                    let mut _state = self.state.write();
                    let cell_clone = cell.clone();

                    if let Some(cell_id) = _state.cell_ids.get(&cell.position) {
                        let cell_transition = _state.cells.map_item(*cell_id, |it| {
                            it.map_cell_type(|lbl| lbl.replace(cell_clone.cell_type));
                            it.map_content(|lbl| lbl.replace(cell_clone.content))
                        });

                        _state.cells.apply(cell_transition);
                    } else {
                        let (cell_id, cell_transition) = _state.cells.append(CellInfo {
                            position: Atom::new(cell_clone.position.clone()),
                            cell_type: Atom::new(cell_clone.cell_type),
                            content: Atom::new(cell_clone.content),
                        });
                        _state.cell_ids.insert(cell_clone.position, cell_id);
                        _state.cells.apply(cell_transition);
                    }

                    let msg = ServerMsg::WriteCell { id, cell };
                    self.update.send(msg).ok();
                }
                ClientMsg::DeleteCell(cursor) => {
                    let mut _state = self.state.write();
                    if let Some(cell_id) = _state.cell_ids.get(&cursor.cursors) {
                        let cell_transition = _state.cells.delete(*cell_id);

                        _state.cell_ids.remove(&cursor.cursors);
                        _state.cells.apply(cell_transition);
                    }

                    let msg = ServerMsg::DeleteCell { id, cursor };
                    self.update.send(msg).ok();
                }
            }
        }
        Ok(())
    }
}
