use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SHELL_COUNTER: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// ShellStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShellStatus {
    Initializing,
    Running,
    Idle,
    Error(String),
    Destroyed,
}

impl fmt::Display for ShellStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellStatus::Initializing => write!(f, "Initializing"),
            ShellStatus::Running => write!(f, "Running"),
            ShellStatus::Idle => write!(f, "Idle"),
            ShellStatus::Error(e) => write!(f, "Error({})", e),
            ShellStatus::Destroyed => write!(f, "Destroyed"),
        }
    }
}

// ---------------------------------------------------------------------------
// ShellKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShellKind {
    Hermes,
    ZeroClaw,
    CUDAClaw,
    Ensign,
    Custom(String),
}

impl ShellKind {
    pub fn is_sandboxed(&self) -> bool {
        matches!(self, ShellKind::ZeroClaw | ShellKind::CUDAClaw)
    }

    pub fn label(&self) -> String {
        match self {
            ShellKind::Hermes => "Hermes".into(),
            ShellKind::ZeroClaw => "ZeroClaw".into(),
            ShellKind::CUDAClaw => "CUDAClaw".into(),
            ShellKind::Ensign => "Ensign".into(),
            ShellKind::Custom(name) => format!("Custom({})", name),
        }
    }

    pub fn default_apis(&self) -> Vec<String> {
        match self {
            ShellKind::Hermes => vec!["read".into(), "write".into(), "network".into()],
            ShellKind::ZeroClaw => vec!["read".into()],
            ShellKind::CUDAClaw => vec!["read".into(), "compute".into(), "cuda".into()],
            ShellKind::Ensign => vec!["read".into(), "write".into()],
            ShellKind::Custom(_) => vec!["read".into()],
        }
    }
}

impl fmt::Display for ShellKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ---------------------------------------------------------------------------
// PortConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortConfig {
    pub protocol: String,
    pub direction: String,
    pub target: String,
    pub permissions: Vec<String>,
    pub enabled: bool,
}

impl PortConfig {
    pub fn new(protocol: &str, direction: &str, target: &str) -> Self {
        Self {
            protocol: protocol.into(),
            direction: direction.into(),
            target: target.into(),
            permissions: vec!["read".into()],
            enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// ChildShell
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildShell {
    pub id: String,
    pub name: String,
    pub kind: ShellKind,
    pub universe_path: String,
    pub parent_path: String,
    pub ports: Vec<PortConfig>,
    pub apis: Vec<String>,
    pub rooms: Vec<String>,
    pub status: ShellStatus,
    pub created_at: u64,
    pub autonomy_level: u32,
    pub conservation_budget: f64,
    pub conservation_used: f64,
}

impl ChildShell {
    fn new(name: &str, kind: ShellKind, parent_path: &str) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let seq = SHELL_COUNTER.fetch_add(1, Ordering::Relaxed);
        let id = format!("shell-{}-{}", created_at, seq);
        let universe_path = format!("{}/{}", parent_path, name);
        Self {
            id,
            name: name.into(),
            kind,
            universe_path,
            parent_path: parent_path.into(),
            ports: Vec::new(),
            apis: Vec::new(),
            rooms: Vec::new(),
            status: ShellStatus::Initializing,
            created_at,
            autonomy_level: 0,
            conservation_budget: 0.0,
            conservation_used: 0.0,
        }
    }

    pub fn status(&self) -> ShellStatus {
        self.status.clone()
    }

    pub fn add_port(&mut self, port: PortConfig) {
        self.ports.push(port);
    }

    pub fn grant_api(&mut self, api: &str) {
        if !self.apis.contains(&api.to_string()) {
            self.apis.push(api.into());
        }
    }

    pub fn revoke_api(&mut self, api: &str) {
        self.apis.retain(|a| a != api);
    }

    pub fn grant_room(&mut self, room_id: &str) {
        if !self.rooms.contains(&room_id.to_string()) {
            self.rooms.push(room_id.into());
        }
    }

    pub fn set_budget(&mut self, budget: f64) {
        self.conservation_budget = budget;
    }

    pub fn is_within_budget(&self) -> bool {
        self.conservation_used <= self.conservation_budget
    }

    pub fn can_access_path(&self, path: &str) -> bool {
        if !self.kind.is_sandboxed() {
            return true;
        }
        path.starts_with(&self.universe_path)
    }

    pub fn describe(&self) -> String {
        format!(
            "[{}] {} ({}) — {} | ports: {} | apis: {} | rooms: {} | budget: {:.2}/{:.2} | sandboxed: {}",
            self.id,
            self.name,
            self.kind.label(),
            self.status,
            self.ports.len(),
            self.apis.join(","),
            self.rooms.len(),
            self.conservation_used,
            self.conservation_budget,
            self.kind.is_sandboxed(),
        )
    }
}

// ---------------------------------------------------------------------------
// ShellTemplate
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellTemplate {
    pub name: String,
    pub kind: ShellKind,
    pub default_ports: Vec<PortConfig>,
    pub default_apis: Vec<String>,
    pub default_rooms: Vec<String>,
    pub default_budget: f64,
    pub autonomy_level: u32,
    pub max_children: u32,
}

impl ShellTemplate {
    pub fn zeroclaw() -> Self {
        Self {
            name: "zeroclaw".into(),
            kind: ShellKind::ZeroClaw,
            default_ports: vec![PortConfig::new("local", "in", "localhost")],
            default_apis: vec!["read".into()],
            default_rooms: vec!["main".into()],
            default_budget: 100.0,
            autonomy_level: 1,
            max_children: 0,
        }
    }

    pub fn cudaclaw() -> Self {
        Self {
            name: "cudaclaw".into(),
            kind: ShellKind::CUDAClaw,
            default_ports: vec![PortConfig::new("cuda", "in", "gpu-bus")],
            default_apis: vec!["read".into(), "compute".into(), "cuda".into()],
            default_rooms: vec!["compute".into()],
            default_budget: 500.0,
            autonomy_level: 2,
            max_children: 0,
        }
    }

    pub fn telegram_bot() -> Self {
        Self {
            name: "telegram_bot".into(),
            kind: ShellKind::Custom("TelegramBot".into()),
            default_ports: vec![PortConfig::new("telegram", "inout", "api.telegram.org")],
            default_apis: vec!["read".into(), "write".into(), "telegram".into()],
            default_rooms: vec!["chat".into()],
            default_budget: 200.0,
            autonomy_level: 3,
            max_children: 0,
        }
    }

    pub fn underwater_vehicle() -> Self {
        Self {
            name: "underwater_vehicle".into(),
            kind: ShellKind::Custom("UnderwaterVehicle".into()),
            default_ports: vec![
                PortConfig::new("sensor", "in", "serial://imu"),
                PortConfig::new("navigation", "out", "serial://thrusters"),
            ],
            default_apis: vec!["read".into(), "navigate".into()],
            default_rooms: vec!["navigation".into()],
            default_budget: 1000.0,
            autonomy_level: 5,
            max_children: 2,
        }
    }

    pub fn media_center() -> Self {
        Self {
            name: "media_center".into(),
            kind: ShellKind::Custom("MediaCenter".into()),
            default_ports: vec![PortConfig::new("media", "inout", "media-bus")],
            default_apis: vec!["read".into(), "write".into(), "media".into()],
            default_rooms: vec!["social".into()],
            default_budget: 300.0,
            autonomy_level: 2,
            max_children: 4,
        }
    }

    pub fn custom(name: &str) -> Self {
        Self {
            name: name.into(),
            kind: ShellKind::Custom(name.into()),
            default_ports: Vec::new(),
            default_apis: vec!["read".into()],
            default_rooms: Vec::new(),
            default_budget: 100.0,
            autonomy_level: 0,
            max_children: 0,
        }
    }

    pub fn with_port(mut self, port: PortConfig) -> Self {
        self.default_ports.push(port);
        self
    }

    pub fn with_api(mut self, api: &str) -> Self {
        if !self.default_apis.contains(&api.to_string()) {
            self.default_apis.push(api.into());
        }
        self
    }

    pub fn with_room(mut self, room: &str) -> Self {
        if !self.default_rooms.contains(&room.to_string()) {
            self.default_rooms.push(room.into());
        }
        self
    }

    pub fn with_budget(mut self, budget: f64) -> Self {
        self.default_budget = budget;
        self
    }
}

// ---------------------------------------------------------------------------
// SpawnError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpawnError {
    NameExists(String),
    MaxChildrenReached(u32),
    PathConflict(String),
    TemplateNotFound(String),
    SandboxViolation(String),
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpawnError::NameExists(name) => write!(f, "child '{}' already exists", name),
            SpawnError::MaxChildrenReached(max) => write!(f, "max children ({}) reached", max),
            SpawnError::PathConflict(path) => write!(f, "path conflict: {}", path),
            SpawnError::TemplateNotFound(name) => write!(f, "template '{}' not found", name),
            SpawnError::SandboxViolation(path) => write!(f, "sandbox violation: {}", path),
        }
    }
}

impl std::error::Error for SpawnError {}

// ---------------------------------------------------------------------------
// ShellSpawn
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSpawn {
    pub parent_path: String,
    pub children: HashMap<String, ChildShell>,
    pub templates: HashMap<String, ShellTemplate>,
}

impl ShellSpawn {
    pub fn new(parent_path: &str) -> Self {
        Self {
            parent_path: parent_path.into(),
            children: HashMap::new(),
            templates: HashMap::new(),
        }
    }

    pub fn register_template(&mut self, template: ShellTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn spawn(&mut self, kind: ShellKind, name: &str) -> Result<ChildShell, SpawnError> {
        if self.children.contains_key(name) {
            return Err(SpawnError::NameExists(name.into()));
        }
        let mut child = ChildShell::new(name, kind, &self.parent_path);
        for api in child.kind.default_apis() {
            child.grant_api(&api);
        }
        child.status = ShellStatus::Running;
        self.children.insert(name.into(), child.clone());
        Ok(child)
    }

    pub fn spawn_from_template(
        &mut self,
        template_name: &str,
        name: &str,
    ) -> Result<ChildShell, SpawnError> {
        let template = self
            .templates
            .get(template_name)
            .ok_or_else(|| SpawnError::TemplateNotFound(template_name.into()))?
            .clone();

        if self.children.contains_key(name) {
            return Err(SpawnError::NameExists(name.into()));
        }

        let mut child = ChildShell::new(name, template.kind, &self.parent_path);
        child.ports = template.default_ports.clone();
        child.apis = template.default_apis.clone();
        child.rooms = template.default_rooms.clone();
        child.conservation_budget = template.default_budget;
        child.autonomy_level = template.autonomy_level;
        child.status = ShellStatus::Running;
        self.children.insert(name.into(), child.clone());
        Ok(child)
    }

    pub fn get_child(&self, name: &str) -> Option<&ChildShell> {
        self.children.get(name)
    }

    pub fn get_child_mut(&mut self, name: &str) -> Option<&mut ChildShell> {
        self.children.get_mut(name)
    }

    pub fn list_children(&self) -> Vec<&ChildShell> {
        self.children.values().collect()
    }

    pub fn destroy(&mut self, name: &str) -> Result<(), SpawnError> {
        let child = self
            .children
            .get_mut(name)
            .ok_or_else(|| SpawnError::NameExists(format!("{}: not found", name)))?;
        child.status = ShellStatus::Destroyed;
        self.children.remove(name);
        Ok(())
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_instance() -> ShellSpawn {
        ShellSpawn::new("/shells/hermes")
    }

    // -- ShellKind tests --

    #[test]
    fn shellkind_is_sandboxed() {
        assert!(ShellKind::ZeroClaw.is_sandboxed());
        assert!(ShellKind::CUDAClaw.is_sandboxed());
        assert!(!ShellKind::Hermes.is_sandboxed());
        assert!(!ShellKind::Ensign.is_sandboxed());
        assert!(!ShellKind::Custom("X".into()).is_sandboxed());
    }

    #[test]
    fn shellkind_label() {
        assert_eq!(ShellKind::Hermes.label(), "Hermes");
        assert_eq!(ShellKind::ZeroClaw.label(), "ZeroClaw");
        assert_eq!(ShellKind::CUDAClaw.label(), "CUDAClaw");
        assert_eq!(ShellKind::Ensign.label(), "Ensign");
        assert_eq!(ShellKind::Custom("Foo".into()).label(), "Custom(Foo)");
    }

    #[test]
    fn shellkind_default_apis() {
        assert_eq!(ShellKind::Hermes.default_apis(), vec!["read", "write", "network"]);
        assert_eq!(ShellKind::ZeroClaw.default_apis(), vec!["read"]);
        assert!(ShellKind::CUDAClaw.default_apis().contains(&"cuda".into()));
    }

    #[test]
    fn shellkind_display() {
        assert_eq!(format!("{}", ShellKind::Hermes), "Hermes");
    }

    // -- ShellStatus tests --

    #[test]
    fn shell_status_display() {
        assert_eq!(format!("{}", ShellStatus::Running), "Running");
        assert_eq!(format!("{}", ShellStatus::Error("oops".into())), "Error(oops)");
    }

    #[test]
    fn shell_status_equality() {
        assert_eq!(ShellStatus::Idle, ShellStatus::Idle);
        assert_ne!(ShellStatus::Initializing, ShellStatus::Running);
    }

    // -- PortConfig tests --

    #[test]
    fn port_config_new() {
        let p = PortConfig::new("http", "in", "0.0.0.0:8080");
        assert_eq!(p.protocol, "http");
        assert_eq!(p.direction, "in");
        assert_eq!(p.target, "0.0.0.0:8080");
        assert!(p.enabled);
        assert_eq!(p.permissions, vec!["read"]);
    }

    #[test]
    fn port_config_equality() {
        let a = PortConfig::new("tcp", "in", "x");
        let b = PortConfig::new("tcp", "in", "x");
        assert_eq!(a, b);
    }

    // -- ChildShell tests --

    #[test]
    fn child_shell_new() {
        let c = ChildShell::new("test-child", ShellKind::ZeroClaw, "/parent");
        assert_eq!(c.name, "test-child");
        assert_eq!(c.kind, ShellKind::ZeroClaw);
        assert!(c.id.starts_with("shell-"));
        assert_eq!(c.universe_path, "/parent/test-child");
        assert_eq!(c.parent_path, "/parent");
        assert_eq!(c.status, ShellStatus::Initializing);
    }

    #[test]
    fn child_shell_add_port() {
        let mut c = ChildShell::new("c", ShellKind::Hermes, "/p");
        c.add_port(PortConfig::new("ws", "in", "0.0.0.0:9000"));
        assert_eq!(c.ports.len(), 1);
        assert_eq!(c.ports[0].protocol, "ws");
    }

    #[test]
    fn child_shell_grant_api() {
        let mut c = ChildShell::new("c", ShellKind::Hermes, "/p");
        c.grant_api("compute");
        assert!(c.apis.contains(&"compute".into()));
    }

    #[test]
    fn child_shell_grant_api_no_dup() {
        let mut c = ChildShell::new("c", ShellKind::Hermes, "/p");
        c.grant_api("read");
        c.grant_api("read");
        assert_eq!(c.apis.iter().filter(|a| **a == "read").count(), 1);
    }

    #[test]
    fn child_shell_revoke_api() {
        let mut c = ChildShell::new("c", ShellKind::Hermes, "/p");
        c.grant_api("compute");
        c.revoke_api("compute");
        assert!(!c.apis.contains(&"compute".into()));
    }

    #[test]
    fn child_shell_grant_room() {
        let mut c = ChildShell::new("c", ShellKind::Hermes, "/p");
        c.grant_room("lab");
        assert!(c.rooms.contains(&"lab".into()));
    }

    #[test]
    fn child_shell_grant_room_no_dup() {
        let mut c = ChildShell::new("c", ShellKind::Hermes, "/p");
        c.grant_room("lab");
        c.grant_room("lab");
        assert_eq!(c.rooms.len(), 1);
    }

    #[test]
    fn child_shell_budget() {
        let mut c = ChildShell::new("c", ShellKind::Hermes, "/p");
        c.set_budget(100.0);
        assert_eq!(c.conservation_budget, 100.0);
        assert!(c.is_within_budget());
        c.conservation_used = 150.0;
        assert!(!c.is_within_budget());
    }

    #[test]
    fn child_shell_can_access_path_sandboxed() {
        let c = ChildShell::new("zc", ShellKind::ZeroClaw, "/shells/hermes");
        assert!(c.can_access_path("/shells/hermes/zc/data"));
        assert!(!c.can_access_path("/shells/hermes/other/data"));
        assert!(!c.can_access_path("/etc/passwd"));
    }

    #[test]
    fn child_shell_can_access_path_unsandboxed() {
        let c = ChildShell::new("h", ShellKind::Hermes, "/shells/hermes");
        assert!(c.can_access_path("/any/path"));
        assert!(c.can_access_path("/etc/passwd"));
    }

    #[test]
    fn child_shell_describe() {
        let c = ChildShell::new("bot", ShellKind::ZeroClaw, "/p");
        let desc = c.describe();
        assert!(desc.contains("bot"));
        assert!(desc.contains("ZeroClaw"));
        assert!(desc.contains("sandboxed: true"));
    }

    #[test]
    fn child_shell_status() {
        let c = ChildShell::new("c", ShellKind::Hermes, "/p");
        assert_eq!(c.status(), ShellStatus::Initializing);
    }

    // -- ShellTemplate tests --

    #[test]
    fn template_zeroclaw() {
        let t = ShellTemplate::zeroclaw();
        assert_eq!(t.name, "zeroclaw");
        assert_eq!(t.kind, ShellKind::ZeroClaw);
        assert_eq!(t.default_rooms, vec!["main"]);
        assert_eq!(t.max_children, 0);
    }

    #[test]
    fn template_cudaclaw() {
        let t = ShellTemplate::cudaclaw();
        assert_eq!(t.kind, ShellKind::CUDAClaw);
        assert!(t.default_apis.contains(&"cuda".into()));
        assert_eq!(t.default_budget, 500.0);
    }

    #[test]
    fn template_telegram_bot() {
        let t = ShellTemplate::telegram_bot();
        assert!(t.default_ports[0].protocol == "telegram");
    }

    #[test]
    fn template_underwater_vehicle() {
        let t = ShellTemplate::underwater_vehicle();
        assert_eq!(t.default_ports.len(), 2);
        assert_eq!(t.max_children, 2);
    }

    #[test]
    fn template_media_center() {
        let t = ShellTemplate::media_center();
        assert_eq!(t.default_budget, 300.0);
    }

    #[test]
    fn template_custom() {
        let t = ShellTemplate::custom("mybot");
        assert_eq!(t.name, "mybot");
        assert_eq!(t.kind, ShellKind::Custom("mybot".into()));
    }

    #[test]
    fn template_builder_chain() {
        let t = ShellTemplate::custom("x")
            .with_port(PortConfig::new("tcp", "out", "10.0.0.1"))
            .with_api("fly")
            .with_room("cockpit")
            .with_budget(999.0);
        assert_eq!(t.default_ports.len(), 1);
        assert!(t.default_apis.contains(&"fly".into()));
        assert!(t.default_rooms.contains(&"cockpit".into()));
        assert_eq!(t.default_budget, 999.0);
    }

    #[test]
    fn template_with_api_no_dup() {
        let t = ShellTemplate::custom("x").with_api("read").with_api("read");
        assert_eq!(t.default_apis.iter().filter(|a| **a == "read").count(), 1);
    }

    #[test]
    fn template_with_room_no_dup() {
        let t = ShellTemplate::custom("x").with_room("r1").with_room("r1");
        assert_eq!(t.default_rooms.len(), 1);
    }

    // -- ShellSpawn tests --

    #[test]
    fn spawn_new() {
        let s = spawn_instance();
        assert_eq!(s.parent_path, "/shells/hermes");
        assert_eq!(s.child_count(), 0);
    }

    #[test]
    fn spawn_child() {
        let mut s = spawn_instance();
        let child = s.spawn(ShellKind::ZeroClaw, "zc-1").unwrap();
        assert_eq!(child.name, "zc-1");
        assert_eq!(child.kind, ShellKind::ZeroClaw);
        assert_eq!(child.status, ShellStatus::Running);
        assert_eq!(s.child_count(), 1);
    }

    #[test]
    fn spawn_child_default_apis() {
        let mut s = spawn_instance();
        let child = s.spawn(ShellKind::Hermes, "h1").unwrap();
        assert!(child.apis.contains(&"read".into()));
        assert!(child.apis.contains(&"write".into()));
        assert!(child.apis.contains(&"network".into()));
    }

    #[test]
    fn spawn_duplicate_name() {
        let mut s = spawn_instance();
        s.spawn(ShellKind::ZeroClaw, "dup").unwrap();
        let err = s.spawn(ShellKind::ZeroClaw, "dup").unwrap_err();
        assert_eq!(err, SpawnError::NameExists("dup".into()));
    }

    #[test]
    fn spawn_from_template() {
        let mut s = spawn_instance();
        s.register_template(ShellTemplate::zeroclaw());
        let child = s.spawn_from_template("zeroclaw", "zc-tpl").unwrap();
        assert_eq!(child.kind, ShellKind::ZeroClaw);
        assert_eq!(child.apis, vec!["read"]);
        assert_eq!(child.rooms, vec!["main"]);
        assert_eq!(child.conservation_budget, 100.0);
        assert_eq!(child.autonomy_level, 1);
    }

    #[test]
    fn spawn_from_template_not_found() {
        let mut s = spawn_instance();
        let err = s.spawn_from_template("nonexistent", "x").unwrap_err();
        assert_eq!(err, SpawnError::TemplateNotFound("nonexistent".into()));
    }

    #[test]
    fn spawn_from_template_duplicate_name() {
        let mut s = spawn_instance();
        s.register_template(ShellTemplate::zeroclaw());
        s.spawn_from_template("zeroclaw", "dup").unwrap();
        let err = s.spawn_from_template("zeroclaw", "dup").unwrap_err();
        assert_eq!(err, SpawnError::NameExists("dup".into()));
    }

    #[test]
    fn get_child() {
        let mut s = spawn_instance();
        s.spawn(ShellKind::Hermes, "h1").unwrap();
        assert!(s.get_child("h1").is_some());
        assert!(s.get_child("nope").is_none());
    }

    #[test]
    fn get_child_mut() {
        let mut s = spawn_instance();
        s.spawn(ShellKind::Hermes, "h1").unwrap();
        let child = s.get_child_mut("h1").unwrap();
        child.grant_room("lab");
        assert!(s.get_child("h1").unwrap().rooms.contains(&"lab".into()));
    }

    #[test]
    fn list_children() {
        let mut s = spawn_instance();
        s.spawn(ShellKind::ZeroClaw, "a").unwrap();
        s.spawn(ShellKind::CUDAClaw, "b").unwrap();
        s.spawn(ShellKind::Hermes, "c").unwrap();
        assert_eq!(s.list_children().len(), 3);
    }

    #[test]
    fn destroy_child() {
        let mut s = spawn_instance();
        s.spawn(ShellKind::ZeroClaw, "gone").unwrap();
        assert_eq!(s.child_count(), 1);
        s.destroy("gone").unwrap();
        assert_eq!(s.child_count(), 0);
        assert!(s.get_child("gone").is_none());
    }

    #[test]
    fn destroy_nonexistent() {
        let mut s = spawn_instance();
        let err = s.destroy("ghost").unwrap_err();
        assert!(matches!(err, SpawnError::NameExists(_)));
    }

    #[test]
    fn child_count() {
        let mut s = spawn_instance();
        assert_eq!(s.child_count(), 0);
        s.spawn(ShellKind::Ensign, "e1").unwrap();
        assert_eq!(s.child_count(), 1);
        s.spawn(ShellKind::Ensign, "e2").unwrap();
        assert_eq!(s.child_count(), 2);
        s.destroy("e1").unwrap();
        assert_eq!(s.child_count(), 1);
    }

    // -- Serde tests --

    #[test]
    fn serde_shell_status() {
        let status = ShellStatus::Error("boom".into());
        let json = serde_json::to_string(&status).unwrap();
        let back: ShellStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }

    #[test]
    fn serde_shell_kind() {
        let kind = ShellKind::Custom("Foo".into());
        let json = serde_json::to_string(&kind).unwrap();
        let back: ShellKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, back);
    }

    #[test]
    fn serde_port_config() {
        let p = PortConfig::new("http", "in", "0.0.0.0:80");
        let json = serde_json::to_string(&p).unwrap();
        let back: PortConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn serde_child_shell() {
        let mut c = ChildShell::new("s1", ShellKind::CUDAClaw, "/p");
        c.grant_api("cuda");
        c.add_port(PortConfig::new("gpu", "in", "pci"));
        let json = serde_json::to_string(&c).unwrap();
        let back: ChildShell = serde_json::from_str(&json).unwrap();
        assert_eq!(c.name, back.name);
        assert_eq!(c.kind, back.kind);
        assert_eq!(c.apis, back.apis);
    }

    #[test]
    fn serde_shell_spawn() {
        let mut s = spawn_instance();
        s.register_template(ShellTemplate::cudaclaw());
        s.spawn_from_template("cudaclaw", "gpu1").unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let back: ShellSpawn = serde_json::from_str(&json).unwrap();
        assert_eq!(back.child_count(), 1);
        assert!(back.templates.contains_key("cudaclaw"));
    }

    #[test]
    fn serde_spawn_error() {
        let err = SpawnError::SandboxViolation("/etc".into());
        let json = serde_json::to_string(&err).unwrap();
        let back: SpawnError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, back);
    }

    // -- SpawnError display tests --

    #[test]
    fn spawn_error_display() {
        assert!(SpawnError::NameExists("x".into()).to_string().contains("x"));
        assert!(SpawnError::MaxChildrenReached(5).to_string().contains("5"));
        assert!(SpawnError::PathConflict("/bad".into()).to_string().contains("/bad"));
        assert!(SpawnError::TemplateNotFound("t".into()).to_string().contains("t"));
        assert!(SpawnError::SandboxViolation("/s".into()).to_string().contains("/s"));
    }

    // -- Integration tests --

    #[test]
    fn full_lifecycle() {
        let mut spawner = ShellSpawn::new("/shells/hermes");

        // Register templates
        spawner.register_template(ShellTemplate::zeroclaw());
        spawner.register_template(ShellTemplate::cudaclaw());
        spawner.register_template(
            ShellTemplate::custom("sensor")
                .with_port(PortConfig::new("i2c", "in", "/dev/i2c-1"))
                .with_api("sensor_read")
                .with_room("sensor-lab")
                .with_budget(50.0),
        );

        // Spawn from templates
        let zc = spawner.spawn_from_template("zeroclaw", "zc-1").unwrap();
        assert_eq!(zc.kind, ShellKind::ZeroClaw);
        assert_eq!(zc.rooms, vec!["main"]);

        let cc = spawner.spawn_from_template("cudaclaw", "cc-1").unwrap();
        assert!(cc.apis.contains(&"cuda".into()));

        let sensor = spawner.spawn_from_template("sensor", "s-1").unwrap();
        assert!(sensor.ports.iter().any(|p| p.protocol == "i2c"));

        // Direct spawn
        let ensign = spawner.spawn(ShellKind::Ensign, "ensign-1").unwrap();
        assert_eq!(ensign.kind, ShellKind::Ensign);

        // Mutate a child
        spawner.get_child_mut("zc-1").unwrap().grant_room("extra");
        assert!(spawner.get_child("zc-1").unwrap().rooms.contains(&"extra".into()));

        // Budget tracking
        spawner.get_child_mut("cc-1").unwrap().conservation_used = 400.0;
        assert!(spawner.get_child("cc-1").unwrap().is_within_budget());
        spawner.get_child_mut("cc-1").unwrap().conservation_used = 600.0;
        assert!(!spawner.get_child("cc-1").unwrap().is_within_budget());

        // Sandboxing
        assert!(!spawner.get_child("zc-1").unwrap().can_access_path("/etc/passwd"));
        assert!(spawner.get_child("ensign-1").unwrap().can_access_path("/etc/passwd"));

        // List & count
        assert_eq!(spawner.child_count(), 4);
        assert_eq!(spawner.list_children().len(), 4);

        // Destroy
        spawner.destroy("s-1").unwrap();
        assert_eq!(spawner.child_count(), 3);
        assert!(spawner.get_child("s-1").is_none());

        // Serde round-trip
        let json = serde_json::to_string(&spawner).unwrap();
        let back: ShellSpawn = serde_json::from_str(&json).unwrap();
        assert_eq!(back.child_count(), 3);
        assert!(back.templates.contains_key("sensor"));
    }

    #[test]
    fn multiple_templates_same_kind() {
        let mut s = spawn_instance();
        s.register_template(ShellTemplate::custom("bot-a"));
        s.register_template(ShellTemplate::custom("bot-b"));
        let a = s.spawn_from_template("bot-a", "a").unwrap();
        let b = s.spawn_from_template("bot-b", "b").unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(s.child_count(), 2);
    }

    #[test]
    fn sandboxed_cannot_escape() {
        let mut s = spawn_instance();
        s.register_template(ShellTemplate::zeroclaw());
        let child = s.spawn_from_template("zeroclaw", "sandboxed").unwrap();
        let path = &child.universe_path;
        assert!(child.can_access_path(path));
        assert!(child.can_access_path(&format!("{}/deep/file", path)));
        assert!(!child.can_access_path("/shells/hermes"));
    }
}
