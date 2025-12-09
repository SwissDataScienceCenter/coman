use tui_realm_stdlib::Phantom;
use tuirealm::{
    Component, Event, MockComponent,
    event::{Key, KeyEvent, KeyModifiers},
};

use crate::app::{
    messages::{JobMsg, MenuMsg, Msg, StatusMsg, SystemSelectMsg, View},
    user_events::{CscsEvent, FileEvent, UserEvent},
};

#[derive(Default, MockComponent)]
pub struct GlobalListener {
    component: Phantom,
    current_view: View,
}

impl Component<Msg, UserEvent> for GlobalListener {
    fn on(&mut self, ev: tuirealm::Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'), ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }) => Some(Msg::AppClose),
            Event::Keyboard(KeyEvent {
                code: Key::Char('x'), ..
            }) => Some(Msg::Menu(MenuMsg::Opened)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('f'), ..
            }) => {
                self.current_view = View::Files;
                Some(Msg::ChangeView(View::Files))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('w'), ..
            }) => {
                self.current_view = View::Workloads;
                Some(Msg::ChangeView(View::Workloads))
            }
            Event::User(UserEvent::Error(msg)) => Some(Msg::Error(msg)),
            Event::User(UserEvent::Info(msg)) => Some(Msg::Info(msg)),
            Event::User(UserEvent::Cscs(CscsEvent::LoggedIn)) => {
                Some(Msg::Status(StatusMsg::Info("Successfully logged in".to_string())))
            }
            Event::User(UserEvent::Cscs(CscsEvent::SelectSystemList(systems))) => {
                Some(Msg::SystemSelectPopup(SystemSelectMsg::Opened(systems)))
            }
            Event::User(UserEvent::Cscs(CscsEvent::GotJobDetails(details))) => Some(Msg::Job(JobMsg::Details(details))),
            Event::User(UserEvent::File(FileEvent::DownloadSuccessful)) => {
                Some(Msg::Status(StatusMsg::Info("File successfully downloaded".to_owned())))
            }
            _ => None,
        }
    }
}
