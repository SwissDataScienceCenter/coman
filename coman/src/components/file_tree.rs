use std::{collections::VecDeque, iter, path::PathBuf};

use tokio::sync::mpsc;
use tui_realm_treeview::{Node, NodeValue, TREE_CMD_CLOSE, TREE_CMD_OPEN, TREE_INITIAL_NODE, Tree, TreeView};
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    props::{Alignment, BorderType, Borders, Color, Style},
    ratatui::layout::{Position as RectPosition, Rect},
};

use crate::{
    app::{
        messages::{DownloadPopupMsg, MenuMsg, Msg},
        user_events::{FileEvent, UserEvent},
    },
    cscs::{api_client::types::PathType, ports::BackgroundTask},
    trace_dbg,
};

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
pub struct FileNode {
    pub name: String,
    pub path_type: PathType,
}

impl FileNode {
    pub fn new(name: String, path_type: PathType) -> Self {
        Self { name, path_type }
    }
}

impl Default for FileNode {
    fn default() -> Self {
        Self {
            name: "/".to_owned(),
            path_type: PathType::Directory,
        }
    }
}

impl NodeValue for FileNode {
    fn render_parts_iter(&self) -> impl Iterator<Item = (&str, Option<Style>)> {
        iter::once((self.name.as_str(), None))
    }
}

pub struct FileTree {
    component: TreeView<FileNode>,
    file_tree_tx: mpsc::Sender<BackgroundTask>,
    current_rect: Rect,
}
impl FileTree {
    pub fn new(file_tree_tx: mpsc::Sender<BackgroundTask>) -> Self {
        let root_node: Node<FileNode> = Node::new("/".to_owned(), FileNode::default());
        let tree = Tree::new(root_node.clone());

        // Load root node
        let tree_tx = file_tree_tx.clone();
        tokio::spawn(async move {
            tree_tx
                .send(BackgroundTask::ListPaths(PathBuf::from("/")))
                .await
                .unwrap()
        });

        Self {
            component: TreeView::default()
                .foreground(Color::Reset)
                .borders(
                    Borders::default()
                        .color(Color::LightYellow)
                        .modifiers(BorderType::Rounded),
                )
                .inactive(Style::default().fg(Color::Gray))
                .indent_size(3)
                .scroll_step(6)
                .title(tree.root().id(), Alignment::Left)
                .highlighted_color(Color::LightYellow)
                .highlight_symbol("❯")
                .with_tree(tree)
                .initial_node(root_node.id()),
            file_tree_tx,
            current_rect: Rect::ZERO,
        }
    }
    fn node_list(&self) -> Vec<&String> {
        let root = self.component.tree().root();
        let mut ids = vec![];
        let mut stack = VecDeque::new();
        stack.push_back(root.id());

        while let Some(current_id) = stack.pop_front() {
            ids.push(current_id);
            let node = root.query(current_id).unwrap();
            if !node.is_leaf() && self.component.tree_state().is_open(node) {
                for child in node.children().iter().rev() {
                    stack.push_front(child.id());
                }
            }
        }

        ids
    }

    fn open_current_node(&mut self) -> CmdResult {
        let current_id = self.state().unwrap_one().unwrap_string();
        let node = self.component.tree().root().query(&current_id).unwrap();
        match node.value().path_type {
            PathType::Directory => {
                if node.children().is_empty() {
                    // try loading children if there are none
                    let tree_tx = self.file_tree_tx.clone();
                    tokio::spawn(async move {
                        tree_tx
                            .send(BackgroundTask::ListPaths(PathBuf::from(current_id)))
                            .await
                            .unwrap();
                    });
                    CmdResult::None
                } else {
                    self.perform(Cmd::Custom(TREE_CMD_OPEN))
                }
            }
            PathType::File => CmdResult::None,
            PathType::Link => CmdResult::None,
        }
    }

    fn close_current_node(&mut self) -> CmdResult {
        let current_id = self.state().unwrap_one().unwrap_string();
        let node = self.component.tree().root().query(&current_id).unwrap();
        if self.component.tree_state().is_closed(node) {
            // current node is already closed, so we select and close the parent
            if let Some(parent) = self.component.tree().root().parent(node.id()) {
                self.attr(
                    Attribute::Custom(TREE_INITIAL_NODE),
                    AttrValue::String(parent.id().clone()),
                );
            }
        }
        self.perform(Cmd::Custom(TREE_CMD_CLOSE))
    }

    fn mouse_select_row(&mut self, row: u16) -> CmdResult {
        let mut list_index = (row - self.current_rect.y) as usize;
        list_index = list_index.saturating_sub(1);
        let render_area_h = self.current_rect.height as usize - 2;
        // adjust for border
        if list_index >= render_area_h {
            list_index = render_area_h - 1;
        }

        // the tree view auto-scrolls when selecting a node, we need to compensate for that in our
        // selection. See `calc_rows_to_skip` in `TreeWidget` for where this comes from.
        let nodes = self.node_list();
        let offset_max = nodes.len().saturating_sub(render_area_h);
        let num_lines_to_show_at_top = render_area_h / 2;
        let root = self.component.tree().root().clone();
        let prev = self.component.tree_state().selected().unwrap();
        let prev_index = nodes.iter().position(|n| n == &&prev.to_string()).unwrap() + 1;
        let current_offset = prev_index.saturating_sub(num_lines_to_show_at_top).min(offset_max);
        list_index += current_offset;
        // current offset is how far the view is currently scrolled

        let selected = root.query(nodes[list_index]).unwrap();
        if prev != selected.id() {
            self.attr(
                Attribute::Custom(TREE_INITIAL_NODE),
                AttrValue::String(selected.id().to_string()),
            );
        }
        if self.component.tree_state().is_open(selected) {
            self.perform(Cmd::Custom(TREE_CMD_CLOSE))
        } else {
            self.open_current_node()
        }
    }
}
impl MockComponent for FileTree {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.current_rect = area;
        self.component.view(frame, area);
    }
    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.component.query(attr)
    }
    fn attr(&mut self, query: Attribute, attr: AttrValue) {
        self.component.attr(query, attr)
    }
    fn state(&self) -> State {
        self.component.state()
    }
    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        self.component.perform(cmd)
    }
}
impl Component<Msg, UserEvent> for FileTree {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Left,
                modifiers: KeyModifiers::NONE,
            }) => self.close_current_node(),
            Event::Keyboard(KeyEvent {
                code: Key::Right,
                modifiers: KeyModifiers::NONE,
            }) => {
                let current_id = self.state().unwrap_one().unwrap_string();
                let node = self.component.tree().root().query(&current_id).unwrap();
                match node.value().path_type {
                    PathType::Directory => {
                        if node.children().is_empty() {
                            // try loading children if there are none
                            let tree_tx = self.file_tree_tx.clone();
                            tokio::spawn(async move {
                                tree_tx
                                    .send(BackgroundTask::ListPaths(PathBuf::from(current_id)))
                                    .await
                                    .unwrap();
                            });
                            CmdResult::None
                        } else {
                            self.perform(Cmd::Custom(TREE_CMD_OPEN))
                        }
                    }
                    PathType::File => CmdResult::None,
                    PathType::Link => CmdResult::None,
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home,
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent {
                code: Key::End,
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::GoTo(Position::End)),

            Event::Mouse(MouseEvent {
                kind, column: col, row, ..
            }) => {
                if !self.current_rect.contains(RectPosition { x: col, y: row }) {
                    CmdResult::None
                } else {
                    match kind {
                        MouseEventKind::Down(MouseButton::Left) => self.mouse_select_row(row),
                        MouseEventKind::Down(MouseButton::Right) => {
                            self.mouse_select_row(row);
                            return Some(Msg::Menu(MenuMsg::Opened));
                        }
                        MouseEventKind::ScrollDown => self.perform(Cmd::Scroll(Direction::Down)),
                        MouseEventKind::ScrollUp => self.perform(Cmd::Scroll(Direction::Up)),
                        _ => CmdResult::None,
                    }
                }
            }
            Event::User(UserEvent::File(FileEvent::List(id, subpaths))) => {
                let tree = self.component.tree_mut();
                let parent = tree.root_mut().query_mut(&id).unwrap();
                parent.clear();
                for entry in subpaths {
                    let id = if entry.name.starts_with("/") {
                        entry.name.clone()
                    } else {
                        format!("{}/{}", id, entry.name.clone())
                    };
                    parent.add_child(Node::new(id, FileNode::new(entry.name, entry.path_type)));
                }
                self.perform(Cmd::Custom(TREE_CMD_OPEN));
                CmdResult::None
            }
            Event::User(UserEvent::File(FileEvent::DownloadCurrentFile)) => {
                if let State::One(StateValue::String(id)) = self.state() {
                    let path = PathBuf::from(id);
                    return Some(Msg::DownloadPopup(DownloadPopupMsg::Opened(path)));
                }
                CmdResult::None
            }
            Event::User(UserEvent::File(FileEvent::DeleteCurrentFile)) => {
                if let State::One(StateValue::String(id)) = self.state() {
                    let tree_tx = self.file_tree_tx.clone();
                    let id = trace_dbg!(id);
                    tokio::spawn(async move {
                        tree_tx.send(BackgroundTask::DeleteFile(id)).await.unwrap();
                    });
                }
                CmdResult::None
            }
            Event::User(UserEvent::File(FileEvent::DeleteSuccessful(id))) => {
                let mut selected_id = id.clone();
                let tree = self.component.tree_mut();
                let parent = tree.root_mut().parent_mut(&id);
                if let Some(parent) = parent {
                    parent.remove_child(&id);
                    selected_id = parent.id().clone();
                }
                self.attr(Attribute::Custom(TREE_INITIAL_NODE), AttrValue::String(selected_id));
                CmdResult::Changed(self.component.state())
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
