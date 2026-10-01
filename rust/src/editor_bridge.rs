//! macOS Godot 4.7 editor-embedded WebView bridge.
//!
//! Godot 4.7 renders an embedded macOS game through a CAContext/CALayer owned by
//! the editor. The running game therefore has no NSView that WRY can use as a
//! parent. This module keeps WRY in the editor process and mirrors the public
//! WebView operations over a project-local Unix-domain socket.

use crate::godot_window::GodotWindow;
use godot::classes::{display_server::HandleType, Control, DisplayServer, EditorInterface, Engine, ProjectSettings};
use godot::obj::Singleton;
use godot::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use wry::WebViewExtMacOS;
use wry::dpi::{PhysicalPosition, PhysicalSize};
use wry::http::Request;
use wry::{PageLoadEvent, Rect, WebContext, WebViewAttributes, WebViewBuilder};

const BRIDGE_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub url: String,
    pub html: String,
    pub data_directory: String,
    pub transparent: bool,
    pub devtools: bool,
    pub user_agent: String,
    pub zoom_hotkeys: bool,
    pub clipboard: bool,
    pub incognito: bool,
    pub focused: bool,
    pub autoplay: bool,
    pub forward_input_events: bool,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
pub struct BridgeBounds {
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub full_window: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BridgeMessage {
    Hello { version: u32 },
    Create {
        id: u64,
        config: BridgeConfig,
        bounds: BridgeBounds,
    },
    Bounds { id: u64, bounds: BridgeBounds },
    SetVisible { id: u64, visible: bool },
    LoadUrl { id: u64, url: String },
    LoadHtml { id: u64, html: String },
    Eval { id: u64, script: String },
    Reload { id: u64 },
    Zoom { id: u64, scale: f64 },
    Focus { id: u64 },
    FocusParent { id: u64 },
    ClearBrowsingData { id: u64 },
    OpenDevtools { id: u64 },
    CloseDevtools { id: u64 },
    Print { id: u64 },
    Destroy { id: u64 },
    Event {
        id: u64,
        event: String,
        payload: String,
    },
    Error {
        id: u64,
        message: String,
    },
}

fn editor_pid_for_socket() -> u32 {
    if Engine::singleton().is_editor_hint() {
        return std::process::id();
    }

    let args: Vec<String> = std::env::args().collect();
    for pair in args.windows(2) {
        if pair[0] == "--editor-pid" {
            if let Ok(pid) = pair[1].parse::<u32>() {
                return pid;
            }
        }
    }
    0
}

fn project_socket_path() -> PathBuf {
    let project_root = ProjectSettings::singleton()
        .globalize_path("res://")
        .to_string();
    let mut hasher = DefaultHasher::new();
    project_root.hash(&mut hasher);
    std::env::temp_dir().join(format!(
        "godot_wry_4_7_{:016x}_{}.sock",
        hasher.finish(),
        editor_pid_for_socket()
    ))
}

fn write_message(stream: &mut UnixStream, message: &BridgeMessage) -> io::Result<()> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    encoded.push(b'\n');
    stream.write_all(&encoded)
}

pub struct EditorBridgeClient {
    reader: RefCell<BufReader<UnixStream>>,
    writer: Mutex<UnixStream>,
}

impl EditorBridgeClient {
    pub fn connect() -> io::Result<Self> {
        let stream = UnixStream::connect(project_socket_path())?;
        stream.set_nonblocking(true)?;
        let writer = stream.try_clone()?;
        writer.set_nonblocking(false)?;
        let client = Self {
            reader: RefCell::new(BufReader::new(stream)),
            writer: Mutex::new(writer),
        };
        client.send(&BridgeMessage::Hello {
            version: BRIDGE_PROTOCOL_VERSION,
        })?;
        Ok(client)
    }

    pub fn send(&self, message: &BridgeMessage) -> io::Result<()> {
        let mut writer = self.writer.lock().map_err(|_| {
            io::Error::new(io::ErrorKind::BrokenPipe, "editor bridge writer poisoned")
        })?;
        write_message(&mut writer, message)
    }

    pub fn poll(&self) -> io::Result<Vec<BridgeMessage>> {
        let mut messages = Vec::new();
        let mut reader = self.reader.borrow_mut();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let message = serde_json::from_str::<BridgeMessage>(line.trim())
                        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
                    messages.push(message);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(messages)
    }
}

struct BridgeConnection {
    reader: BufReader<UnixStream>,
    writer: Arc<Mutex<UnixStream>>,
    hosts: HashMap<u64, HostedWebView>,
    pending_hosts: HashMap<u64, (BridgeConfig, BridgeBounds)>,
    protocol_ok: bool,
}

impl BridgeConnection {
    fn new(stream: UnixStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        let writer_stream = stream.try_clone()?;
        writer_stream.set_nonblocking(false)?;
        Ok(Self {
            reader: BufReader::new(stream),
            writer: Arc::new(Mutex::new(writer_stream)),
            hosts: HashMap::new(),
            pending_hosts: HashMap::new(),
            protocol_ok: false,
        })
    }

    fn send(&self, message: &BridgeMessage) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = write_message(&mut writer, message);
        }
    }

    fn read_available(&mut self) -> io::Result<(Vec<BridgeMessage>, bool)> {
        let mut messages = Vec::new();
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return Ok((messages, true)),
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<BridgeMessage>(line.trim()) {
                        Ok(message) => messages.push(message),
                        Err(error) => {
                            godot_warn!("[Godot WRY] Ignoring invalid editor bridge message: {error}");
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    return Ok((messages, false));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn handle(&mut self, message: BridgeMessage) {
        match message {
            BridgeMessage::Hello { version } => {
                self.protocol_ok = version == BRIDGE_PROTOCOL_VERSION;
                godot_print!(
                    "[Godot WRY] Editor bridge handshake: version={version}, accepted={}",
                    self.protocol_ok
                );
                if !self.protocol_ok {
                    self.send(&BridgeMessage::Error {
                        id: 0,
                        message: format!(
                            "Editor bridge protocol mismatch: game={version}, editor={BRIDGE_PROTOCOL_VERSION}"
                        ),
                    });
                }
            }
            BridgeMessage::Create { id, config, bounds } if self.protocol_ok => {
                godot_print!("[Godot WRY] Editor bridge queued WebView id={id}");
                self.pending_hosts.insert(id, (config, bounds));
                self.try_create_pending();
            }
            BridgeMessage::Bounds { id, bounds } => {
                if let Some(host) = self.hosts.get_mut(&id) {
                    host.bounds = bounds;
                    host.requested_visible = bounds.visible;
                } else if let Some((_config, pending_bounds)) = self.pending_hosts.get_mut(&id) {
                    *pending_bounds = bounds;
                }
            }
            BridgeMessage::SetVisible { id, visible } => {
                if let Some(host) = self.hosts.get_mut(&id) {
                    host.requested_visible = visible;
                    host.refresh_bounds();
                } else if let Some((_config, bounds)) = self.pending_hosts.get_mut(&id) {
                    bounds.visible = visible;
                }
            }
            BridgeMessage::LoadUrl { id, url } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.load_url(&url);
                } else if let Some((config, _bounds)) = self.pending_hosts.get_mut(&id) {
                    config.url = url;
                    config.html.clear();
                }
            }
            BridgeMessage::LoadHtml { id, html } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.load_html(&html);
                } else if let Some((config, _bounds)) = self.pending_hosts.get_mut(&id) {
                    config.html = html;
                    config.url.clear();
                }
            }
            BridgeMessage::Eval { id, script } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.evaluate_script(&script);
                }
            }
            BridgeMessage::Reload { id } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.reload();
                }
            }
            BridgeMessage::Zoom { id, scale } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.zoom(scale);
                }
            }
            BridgeMessage::Focus { id } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.focus();
                }
            }
            BridgeMessage::FocusParent { id } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.focus_parent();
                }
            }
            BridgeMessage::ClearBrowsingData { id } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.clear_all_browsing_data();
                }
            }
            BridgeMessage::OpenDevtools { id } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.open_devtools();
                }
            }
            BridgeMessage::CloseDevtools { id } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.close_devtools();
                }
            }
            BridgeMessage::Print { id } => {
                if let Some(host) = self.hosts.get(&id) {
                    let _ = host.webview.print();
                }
            }
            BridgeMessage::Destroy { id } => {
                self.hosts.remove(&id);
                self.pending_hosts.remove(&id);
            }
            BridgeMessage::Event { .. } | BridgeMessage::Error { .. } | BridgeMessage::Create { .. } => {}
        }
    }

    fn try_create_pending(&mut self) {
        if find_game_panel().is_none() {
            return;
        }

        let pending_ids: Vec<u64> = self.pending_hosts.keys().copied().collect();
        for id in pending_ids {
            let Some((config, bounds)) = self.pending_hosts.remove(&id) else {
                continue;
            };
            match HostedWebView::new(id, config, bounds, Arc::clone(&self.writer)) {
                Ok(host) => {
                    self.hosts.insert(id, host);
                }
                Err(error) => {
                    self.send(&BridgeMessage::Error { id, message: error });
                }
            }
        }
    }
}

struct HostedWebView {
    webview: wry::WebView,
    _context: WebContext,
    bounds: BridgeBounds,
    requested_visible: bool,
    parent_window_id: i32,
    input_transform: Arc<Mutex<(f64, f64)>>,
}

impl HostedWebView {
    fn new(
        id: u64,
        config: BridgeConfig,
        bounds: BridgeBounds,
        writer: Arc<Mutex<UnixStream>>,
    ) -> Result<Self, String> {
        let panel = find_game_panel().ok_or_else(|| {
            "Godot 4.7 GamePanel was not found in the editor UI".to_string()
        })?;
        let window = panel
            .get_window()
            .ok_or_else(|| "GamePanel has no parent Window".to_string())?;
        let parent_window_id = window.get_window_id();
        let parent = GodotWindow::new(parent_window_id);

        let resolved_data_directory = resolve_data_directory(&config.data_directory);
        let mut context = WebContext::new(resolved_data_directory);

        let ipc_writer = Arc::clone(&writer);
        let load_writer = Arc::clone(&writer);
        let input_transform = Arc::new(Mutex::new((1.0f64, 1.0f64)));
        let ipc_transform = Arc::clone(&input_transform);
        let mut builder = WebViewBuilder::with_attributes(WebViewAttributes {
            context: Some(&mut context),
            url: if config.html.is_empty() {
                Some(config.url.clone())
            } else {
                None
            },
            html: if config.url.is_empty() {
                Some(config.html.clone())
            } else {
                None
            },
            transparent: config.transparent,
            devtools: config.devtools,
            user_agent: Some(config.user_agent.clone()),
            zoom_hotkeys_enabled: config.zoom_hotkeys,
            clipboard: config.clipboard,
            incognito: config.incognito,
            focused: config.focused,
            autoplay: config.autoplay,
            accept_first_mouse: true,
            ..Default::default()
        })
        .with_ipc_handler(move |request: Request<String>| {
            let mut payload = request.body().clone();
            if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&payload) {
                let forwarded = value
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|kind| kind.starts_with('_'))
                    .unwrap_or(false);
                if forwarded {
                    let (sx, sy) = ipc_transform.lock().map(|v| *v).unwrap_or((1.0, 1.0));
                    for key in ["x", "movementX"] {
                        if let Some(number) = value.get(key).and_then(|v| v.as_f64()) {
                            value[key] = serde_json::json!(number * sx);
                        }
                    }
                    for key in ["y", "movementY"] {
                        if let Some(number) = value.get(key).and_then(|v| v.as_f64()) {
                            value[key] = serde_json::json!(number * sy);
                        }
                    }
                    if let Ok(encoded) = serde_json::to_string(&value) {
                        payload = encoded;
                    }
                }
            }
            let message = BridgeMessage::Event {
                id,
                event: "ipc".to_string(),
                payload,
            };
            if let Ok(mut stream) = ipc_writer.lock() {
                if !payload.starts_with("{\"type\":\"_") {
                    godot_print!("[Godot WRY] Editor bridge IPC id={id}: {payload}");
                }
                let _ = write_message(&mut stream, &message);
            }
        })
        .with_on_page_load_handler(move |event: PageLoadEvent, url: String| {
            let event = match event {
                PageLoadEvent::Started => "page_load_started",
                PageLoadEvent::Finished => "page_load_finished",
            };
            let message = BridgeMessage::Event {
                id,
                event: event.to_string(),
                payload: url,
            };
            if let Ok(mut stream) = load_writer.lock() {
                let _ = write_message(&mut stream, &message);
            }
        })
        .with_custom_protocol("res".into(), move |_webview_id, request| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::protocols::get_res_response(request)
            }))
            .unwrap_or_else(|_| {
                http::Response::builder()
                    .header("Content-Type", "text/plain")
                    .status(500)
                    .body(std::borrow::Cow::from(
                        b"res:// handler error" as &[u8]
                    ))
                    .unwrap()
            })
        });

        if config.forward_input_events {
            builder = builder.with_initialization_script(FORWARD_INPUT_SCRIPT);
        }

        godot_print!(
            "[Godot WRY] Creating editor-hosted WKWebView id={id} in editor window {parent_window_id}"
        );
        let webview = builder
            .build_as_child(&parent)
            .map_err(|error| format!("Failed to create editor-hosted WKWebView: {error}"))?;
        godot_print!("[Godot WRY] Editor-hosted WKWebView id={id} created");

        let mut host = Self {
            webview,
            _context: context,
            bounds,
            requested_visible: bounds.visible,
            parent_window_id,
            input_transform,
        };
        host.refresh_bounds();
        Ok(host)
    }

    fn refresh_bounds(&mut self) {
        let Some(panel) = find_game_panel() else {
            let _ = self.webview.set_visible(false);
            return;
        };
        let Some(window) = panel.get_window() else {
            let _ = self.webview.set_visible(false);
            return;
        };

        if window.get_window_id() != self.parent_window_id {
            let new_window_id = window.get_window_id();
            let native_window = DisplayServer::singleton()
                .window_get_native_handle_ex(HandleType::WINDOW_HANDLE)
                .window_id(new_window_id)
                .done();
            if native_window == 0
                || self
                    .webview
                    .reparent(std::ptr::with_exposed_provenance_mut(native_window as usize))
                    .is_err()
            {
                let _ = self.webview.set_visible(false);
                return;
            }
            self.parent_window_id = new_window_id;
        }

        let rect = map_game_rect_to_editor(&panel, self.bounds);
        let source_width = if self.bounds.full_window {
            self.bounds.viewport_width.max(1) as f64
        } else {
            self.bounds.width.max(1.0) as f64
        };
        let source_height = if self.bounds.full_window {
            self.bounds.viewport_height.max(1) as f64
        } else {
            self.bounds.height.max(1.0) as f64
        };
        if let Ok(mut transform) = self.input_transform.lock() {
            transform.0 = source_width / rect.size.x.max(1.0) as f64;
            transform.1 = source_height / rect.size.y.max(1.0) as f64;
        }

        let scale = editor_content_scale(&panel);
        let physical = Rect {
            position: PhysicalPosition::new(
                (rect.position.x * scale.0).round(),
                (rect.position.y * scale.1).round(),
            )
            .into(),
            size: PhysicalSize::new(
                (rect.size.x * scale.0).round(),
                (rect.size.y * scale.1).round(),
            )
            .into(),
        };
        let _ = self.webview.set_bounds(physical);
        let _ = self
            .webview
            .set_visible(self.requested_visible && panel.is_visible_in_tree());
    }
}

fn resolve_data_directory(value: &str) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }

    let path = if let Some(relative) = value.strip_prefix("user://") {
        let mut path = PathBuf::from(
            ProjectSettings::singleton()
                .globalize_path("user://")
                .to_string(),
        );
        path.push(relative);
        path
    } else {
        PathBuf::from(value)
    };
    let _ = fs::create_dir_all(&path);
    Some(path)
}

fn editor_content_scale(control: &Gd<Control>) -> (f32, f32) {
    if let Some(window) = control.get_window() {
        let window_size = window.get_size();
        if let Some(viewport) = control.get_viewport() {
            let viewport_size = viewport.get_visible_rect().size;
            if viewport_size.x > 0.0 && viewport_size.y > 0.0 {
                return (
                    window_size.x as f32 / viewport_size.x,
                    window_size.y as f32 / viewport_size.y,
                );
            }
        }
    }
    (1.0, 1.0)
}

fn embed_size_mode() -> i64 {
    let Some(mut settings) = EditorInterface::singleton().get_editor_settings() else {
        return 0;
    };
    let result = settings.call(
        "get_project_metadata",
        &[
            "game_view".to_variant(),
            "embed_size_mode".to_variant(),
            0i64.to_variant(),
        ],
    );
    result.try_to::<i64>().unwrap_or(0)
}

fn map_game_rect_to_editor(panel: &Gd<Control>, bounds: BridgeBounds) -> Rect2 {
    let panel_rect = panel.get_global_rect();
    let viewport_w = bounds.viewport_width.max(1) as f32;
    let viewport_h = bounds.viewport_height.max(1) as f32;

    let mut game_rect = panel_rect;
    let mode = embed_size_mode();
    if mode != 2 {
        let mut ratio = (panel_rect.size.x / viewport_w)
            .min(panel_rect.size.y / viewport_h);
        // Fixed-size mode does not scale a smaller game up; keep-aspect does.
        if mode == 0 {
            ratio = ratio.min(1.0);
        }
        let size = Vector2::new(viewport_w * ratio, viewport_h * ratio);
        game_rect = Rect2::new(
            panel_rect.position + (panel_rect.size - size) * 0.5,
            size,
        );
    }

    if bounds.full_window {
        return game_rect;
    }

    let sx = game_rect.size.x / viewport_w;
    let sy = game_rect.size.y / viewport_h;
    Rect2::new(
        game_rect.position + Vector2::new(bounds.x * sx, bounds.y * sy),
        Vector2::new(bounds.width * sx, bounds.height * sy),
    )
}

fn find_game_panel() -> Option<Gd<Control>> {
    let root = EditorInterface::singleton().get_base_control()?;
    find_game_panel_recursive(root.upcast())
}

fn find_game_panel_recursive(node: Gd<godot::classes::Node>) -> Option<Gd<Control>> {
    for child in node.get_children().iter_shared() {
        if let Ok(control) = child.clone().try_cast::<Control>() {
            if control.get_theme_type_variation() == StringName::from("GamePanel")
                && control.is_visible_in_tree()
            {
                return Some(control);
            }
        }
        if let Some(found) = find_game_panel_recursive(child) {
            return Some(found);
        }
    }
    None
}

struct EditorBridgeServer {
    listener: UnixListener,
    connections: Vec<BridgeConnection>,
    socket_path: PathBuf,
}

impl EditorBridgeServer {
    fn start() -> io::Result<Self> {
        let socket_path = project_socket_path();
        if socket_path.exists() {
            let _ = fs::remove_file(&socket_path);
        }
        let listener = UnixListener::bind(&socket_path)?;
        listener.set_nonblocking(true)?;
        godot_print!(
            "[Godot WRY] Godot 4.7 editor bridge listening at {}",
            socket_path.display()
        );
        Ok(Self {
            listener,
            connections: Vec::new(),
            socket_path,
        })
    }

    fn poll(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => match BridgeConnection::new(stream) {
                    Ok(connection) => self.connections.push(connection),
                    Err(error) => godot_warn!(
                        "[Godot WRY] Failed to accept editor bridge connection: {error}"
                    ),
                },
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    godot_warn!("[Godot WRY] Editor bridge accept failed: {error}");
                    break;
                }
            }
        }

        let mut dead = Vec::new();
        for (index, connection) in self.connections.iter_mut().enumerate() {
            match connection.read_available() {
                Ok((messages, eof)) => {
                    for message in messages {
                        connection.handle(message);
                    }
                    connection.try_create_pending();
                    for host in connection.hosts.values_mut() {
                        host.refresh_bounds();
                    }
                    if eof {
                        dead.push(index);
                    }
                }
                Err(error) => {
                    godot_warn!("[Godot WRY] Editor bridge client disconnected: {error}");
                    dead.push(index);
                }
            }
        }
        for index in dead.into_iter().rev() {
            self.connections.remove(index);
        }
    }
}

impl Drop for EditorBridgeServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

thread_local! {
    static EDITOR_SERVER: RefCell<Option<EditorBridgeServer>> = const { RefCell::new(None) };
}

pub fn start_editor_server() {
    EDITOR_SERVER.with(|server| {
        if server.borrow().is_some() {
            return;
        }
        match EditorBridgeServer::start() {
            Ok(instance) => *server.borrow_mut() = Some(instance),
            Err(error) => godot_warn!("[Godot WRY] Could not start editor bridge: {error}"),
        }
    });
}

pub fn poll_editor_server() {
    EDITOR_SERVER.with(|server| {
        if let Some(server) = server.borrow_mut().as_mut() {
            server.poll();
        }
    });
}

pub fn stop_editor_server() {
    EDITOR_SERVER.with(|server| {
        server.borrow_mut().take();
    });
}

pub const FORWARD_INPUT_SCRIPT: &str = r#"
document.addEventListener('mousemove', (e) => {
    if (!document.hasFocus()) return;
    window.ipc.postMessage(JSON.stringify({
        type: '_mouse_move', x: e.clientX, y: e.clientY,
        movementX: e.movementX, movementY: e.movementY, button: e.button
    }));
});
document.addEventListener('mousedown', (e) => {
    if (!document.hasFocus()) return;
    window.ipc.postMessage(JSON.stringify({ type: '_mouse_down', x: e.clientX, y: e.clientY, button: e.button }));
});
document.addEventListener('mouseup', (e) => {
    if (!document.hasFocus()) return;
    window.ipc.postMessage(JSON.stringify({ type: '_mouse_up', x: e.clientX, y: e.clientY, button: e.button }));
});
document.addEventListener('wheel', (e) => {
    if (!document.hasFocus()) return;
    window.ipc.postMessage(JSON.stringify({
        type: '_mouse_wheel', x: e.clientX, y: e.clientY,
        deltaX: e.deltaX, deltaY: e.deltaY,
        shift: e.shiftKey, ctrl: e.ctrlKey, alt: e.altKey, meta: e.metaKey
    }));
});
document.addEventListener('keydown', (e) => {
    if (!document.hasFocus()) return;
    const isModifier = ["Alt", "Shift", "Control", "Meta"].includes(e.key);
    window.ipc.postMessage(JSON.stringify({
        type: '_key_down', key: e.key, code: e.code, keyCode: e.keyCode,
        shift: isModifier ? false : e.shiftKey,
        ctrl: isModifier ? false : e.ctrlKey,
        alt: isModifier ? false : e.altKey,
        meta: isModifier ? false : e.metaKey
    }));
});
document.addEventListener('keyup', (e) => {
    if (!document.hasFocus()) return;
    const isModifier = ["Alt", "Shift", "Control", "Meta"].includes(e.key);
    window.ipc.postMessage(JSON.stringify({
        type: '_key_up', key: e.key, code: e.code, keyCode: e.keyCode,
        shift: isModifier ? false : e.shiftKey,
        ctrl: isModifier ? false : e.ctrlKey,
        alt: isModifier ? false : e.altKey,
        meta: isModifier ? false : e.metaKey
    }));
});
"#;

static NEXT_BRIDGE_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_bridge_id() -> u64 {
    NEXT_BRIDGE_ID.fetch_add(1, Ordering::Relaxed)
}
