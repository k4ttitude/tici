#[derive(Debug)]
pub struct Pane {
    pub index: u32,
    pub title: String,
    pub current_path: String,
    pub active: bool,
    pub current_command: String,
    pub pid: u32,
}

#[derive(Debug)]
pub struct Window {
    pub session_name: String,
    pub index: u32,
    pub name: String,
    pub active: bool,
    pub layout: String,
    pub panes: Vec<Pane>,
}
