use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use zbus::zvariant;

use crate::screencast_thread::start_stream_on_thread;

const CURSOR_MODE_HIDDEN: u32 = 1;
const CURSOR_MODE_EMBEDDED: u32 = 2;
const CURSOR_MODE_METADATA: u32 = 4;

const SOURCE_TYPE_MONITOR: u32 = 1;
const SOURCE_TYPE_WINDOW: u32 = 2;
const SOURCE_TYPE_VIRTUAL: u32 = 4;

#[derive(zvariant::SerializeDict, zvariant::Type)]
#[zvariant(signature = "a{sv}")]
struct CreateSessionResult {
    session_id: String,
}

#[derive(zvariant::DeserializeDict, zvariant::Type)]
#[zvariant(signature = "a{sv}")]
struct SelectSourcesOptions {
    // Default: monitor
    types: Option<u32>,
    // Default: false
    multiple: Option<bool>,
    restore_data: Option<(String, u32, zvariant::OwnedValue)>,
    // Default: 0
    persist_mode: Option<u32>,
}

#[derive(zvariant::SerializeDict, zvariant::Type)]
#[zvariant(signature = "a{sv}")]
struct StartResult {
    streams: Vec<(u32, HashMap<String, zvariant::OwnedValue>)>,
    persist_mode: Option<u32>,
    restore_data: Option<(String, u32, zvariant::OwnedValue)>,
}

#[derive(Default)]
struct SessionData {
    thread_stop_tx: Option<pipewire::channel::Sender<()>>,
    closed: bool,
}

impl SessionData {
    fn close(&mut self) {
        if let Some(thread_stop_tx) = self.thread_stop_tx.take() {
            let _ = thread_stop_tx.send(());
        }
        self.closed = true
        // XXX Remove from hashmap?
    }
}

#[derive(Default)]
pub struct ScreenCast {
    sessions: Mutex<HashMap<zvariant::ObjectPath<'static>, Arc<Mutex<SessionData>>>>,
}

#[zbus::dbus_interface(name = "org.freedesktop.impl.portal.ScreenCast")]
impl ScreenCast {
    async fn create_session(
        &self,
        #[zbus(connection)] connection: &zbus::Connection,
        handle: zvariant::ObjectPath<'_>,
        session_handle: zvariant::ObjectPath<'_>,
        app_id: String,
        options: HashMap<String, zvariant::OwnedValue>,
    ) -> (u32, CreateSessionResult) {
        // TODO: handle
        let session_data = Arc::new(Mutex::new(SessionData::default()));
        self.sessions
            .lock()
            .unwrap()
            .insert(session_handle.to_owned(), session_data.clone());
        let destroy_session = move || session_data.lock().unwrap().close();
        connection
            .object_server()
            .at(&session_handle, crate::Session::new(destroy_session))
            .await
            .unwrap(); // XXX unwrap
        (
            crate::PORTAL_RESPONSE_SUCCESS,
            CreateSessionResult {
                session_id: "foo".to_string(), // XXX
            },
        )
    }

    async fn select_sources(
        &self,
        handle: zvariant::ObjectPath<'_>,
        session_handle: zvariant::ObjectPath<'_>,
        app_id: String,
        options: SelectSourcesOptions,
    ) -> (u32, HashMap<String, zvariant::OwnedValue>) {
        // TODO: XXX
        (crate::PORTAL_RESPONSE_SUCCESS, HashMap::new())
    }

    async fn start(
        &self,
        handle: zvariant::ObjectPath<'_>,
        session_handle: zvariant::ObjectPath<'_>,
        app_id: String,
        parent_window: String,
        options: HashMap<String, zvariant::OwnedValue>,
    ) -> (u32, StartResult) {
        let session_data = match self.sessions.lock().unwrap().get(&session_handle) {
            Some(session_data) => session_data.clone(),
            None => {
                return (
                    crate::PORTAL_RESPONSE_OTHER,
                    StartResult {
                        streams: vec![],
                        persist_mode: None,
                        restore_data: None,
                    },
                )
            }
        };

        let res = start_stream_on_thread().await;

        let (res, streams) = if let Ok((Some(node_id), thread_stop_tx)) = res {
            let mut session_data = session_data.lock().unwrap();
            session_data.thread_stop_tx = Some(thread_stop_tx);
            if session_data.closed {
                session_data.close();
                (crate::PORTAL_RESPONSE_OTHER, vec![])
            } else {
                (
                    crate::PORTAL_RESPONSE_SUCCESS,
                    vec![(node_id, HashMap::new())],
                )
            }
        } else {
            (crate::PORTAL_RESPONSE_OTHER, vec![])
        };
        (
            res,
            StartResult {
                // XXX
                streams,
                persist_mode: None,
                restore_data: None,
            },
        )
    }

    #[dbus_interface(property)]
    async fn available_source_types(&self) -> u32 {
        // XXX
        SOURCE_TYPE_MONITOR
    }

    #[dbus_interface(property)]
    async fn available_cursor_modes(&self) -> u32 {
        // XXX
        CURSOR_MODE_HIDDEN
    }

    #[dbus_interface(property, name = "version")]
    async fn version(&self) -> u32 {
        4
    }
}