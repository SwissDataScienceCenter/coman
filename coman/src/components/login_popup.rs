use tui_realm_stdlib::components::Input;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    component::{AppComponent, Component},
    event::{Event, Key, KeyEvent},
    props::{AttrValue, Attribute, BorderType, Borders, Color, InputType, Layout, Props, QueryResult, Style, Title},
    ratatui::{
        Frame,
        layout::{Constraint, Direction as LayoutDirection, Rect},
        text::Line,
        widgets::Block,
    },
    state::State,
};

use crate::app::{
    messages::{LoginPopupMsg, Msg},
    user_events::UserEvent,
};
#[derive(Debug, PartialEq, Eq)]
enum ActiveInput {
    ClientId,
    ClientSecret,
}

pub struct LoginPopup {
    props: Props,
    client_id_input: Box<ClientIdInput>,
    client_secret_input: Box<ClientSecretInput>,
    active_input: ActiveInput,
}

impl LoginPopup {
    pub fn new() -> Self {
        let mut popup = Self {
            props: Props::default(),
            client_id_input: Box::new(ClientIdInput::default()),
            client_secret_input: Box::new(ClientSecretInput::default()),
            active_input: ActiveInput::ClientId,
        };
        popup.client_id_input.attr(Attribute::Focus, AttrValue::Flag(true));
        popup
            .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Green))
            .title("Login")
            .layout(
                Layout::default()
                    .constraints(&[Constraint::Length(3), Constraint::Length(3)])
                    .direction(LayoutDirection::Vertical)
                    .margin(1),
            )
    }

    #[allow(dead_code)]
    pub fn foreground(mut self, fg: Color) -> Self {
        self.attr(Attribute::Foreground, AttrValue::Color(fg));
        self
    }

    #[allow(dead_code)]
    pub fn background(mut self, bg: Color) -> Self {
        self.attr(Attribute::Background, AttrValue::Color(bg));
        self
    }

    pub fn borders(mut self, b: Borders) -> Self {
        self.attr(Attribute::Borders, AttrValue::Borders(b));
        self
    }

    pub fn title<S: Into<String>>(mut self, t: S) -> Self {
        self.attr(Attribute::Title, AttrValue::Title(Title::from(t.into())));
        self
    }

    pub fn layout(mut self, layout: Layout) -> Self {
        self.attr(Attribute::Layout, AttrValue::Layout(layout));
        self
    }

    pub fn focus_next(&mut self) {
        self.active_input = match self.active_input {
            ActiveInput::ClientId => {
                self.client_id_input.attr(Attribute::Focus, AttrValue::Flag(false));
                self.client_secret_input.attr(Attribute::Focus, AttrValue::Flag(true));
                ActiveInput::ClientSecret
            }
            ActiveInput::ClientSecret => {
                self.client_secret_input.attr(Attribute::Focus, AttrValue::Flag(false));
                self.client_id_input.attr(Attribute::Focus, AttrValue::Flag(true));
                ActiveInput::ClientId
            }
        };
    }
}

impl Component for LoginPopup {
    fn view(&mut self, render: &mut Frame, area: Rect) {
        // Make a Span
        if self
            .props
            .get(Attribute::Display)
            .unwrap_or(&AttrValue::Flag(true))
            .clone()
            .unwrap_flag()
        {
            // Make block
            let borders = self
                .props
                .get(Attribute::Borders)
                .unwrap_or(&AttrValue::Borders(Borders::default()))
                .clone()
                .unwrap_borders();
            let title = self
                .props
                .get(Attribute::Title)
                .and_then(|x| x.as_title())
                .map_or(Line::from(""), |v| v.content.clone());
            let div = Block::default()
                .borders(borders.sides)
                .border_style(borders.style())
                .border_type(borders.modifiers)
                .title(title);
            // Render block
            render.render_widget(div, area);
            // Render children
            if let Some(layout) = self.props.get(Attribute::Layout).map(|x| x.clone().unwrap_layout()) {
                // make chunks
                let chunks = layout.chunks(area);
                self.client_id_input.view(render, chunks[0]);
                self.client_secret_input.view(render, chunks[1]);
            }
        }
    }
    fn query(&self, attr: Attribute) -> Option<QueryResult<'_>> {
        self.props.get_for_query(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value.clone());
    }

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        // Send command to children and return batch
        match self.active_input {
            ActiveInput::ClientId => self.client_id_input.perform(cmd),
            ActiveInput::ClientSecret => self.client_secret_input.perform(cmd),
        }
    }
}

impl AppComponent<Msg, UserEvent> for LoginPopup {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent { code: Key::Left, .. }) => self.perform(Cmd::Move(Direction::Left)),
            Event::Keyboard(KeyEvent { code: Key::Right, .. }) => self.perform(Cmd::Move(Direction::Right)),
            Event::Keyboard(KeyEvent { code: Key::Home, .. }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => self.perform(Cmd::GoTo(Position::End)),
            Event::Keyboard(KeyEvent { code: Key::Delete, .. }) => self.perform(Cmd::Cancel),
            Event::Keyboard(KeyEvent {
                code: Key::Backspace, ..
            }) => self.perform(Cmd::Delete),
            Event::Keyboard(KeyEvent {
                code: Key::Char(ch), ..
            }) => self.perform(Cmd::Type(ch.to_owned())),
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => {
                self.focus_next();
                CmdResult::NoChange
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                //todo send data
                let client_id = self.client_id_input.state().unwrap_single().unwrap_string();
                let client_secret = self.client_secret_input.state().unwrap_single().unwrap_string();
                return Some(Msg::LoginPopup(LoginPopupMsg::LoginDone(client_id, client_secret)));
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::LoginPopup(LoginPopupMsg::Closed));
            }
            _ => CmdResult::NoChange,
        };
        Some(Msg::None)
    }
}

#[derive(Component)]
struct ClientIdInput {
    component: Input,
}

impl Default for ClientIdInput {
    fn default() -> Self {
        Self {
            component: Input::default()
                .borders(Borders::default().modifiers(BorderType::Rounded))
                .foreground(Color::LightCyan)
                .input_type(InputType::Text)
                .title("Client Id")
                .invalid_style(Style::default().fg(Color::Red)),
        }
    }
}

#[derive(Component)]
struct ClientSecretInput {
    component: Input,
}
impl Default for ClientSecretInput {
    fn default() -> Self {
        Self {
            component: Input::default()
                .borders(Borders::default().modifiers(BorderType::Rounded))
                .foreground(Color::LightCyan)
                .input_type(InputType::Password('*'))
                .title("Client Secret")
                .invalid_style(Style::default().fg(Color::Red)),
        }
    }
}
