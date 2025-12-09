use std::{iter, path::PathBuf};

use tokio::sync::mpsc;
use tui_realm_treeview::{Node, NodeValue, TREE_CMD_CLOSE, TREE_CMD_OPEN, TREE_INITIAL_NODE, Tree, TreeView};
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, State, StateValue,
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent, KeyModifiers},
    props::{Alignment, BorderType, Borders, Color, Style},
};

use crate::{
    app::{
        messages::{DownloadPopupMsg, Msg},
        user_events::{FileEvent, UserEvent},
    },
    cscs::{api_client::PathType, ports::BackgroundTask},
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

#[derive(MockComponent)]
pub struct FileTree {
    component: TreeView<FileNode>,
    file_tree_tx: mpsc::Sender<BackgroundTask>,
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
        }
    }
}
impl Component<Msg, UserEvent> for FileTree {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Left,
                modifiers: KeyModifiers::NONE,
            }) => {
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
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
