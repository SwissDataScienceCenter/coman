use tui_realm_stdlib::components::Phantom;
use tuirealm::{
    component::AppComponent,
    component::Component,
    event::Event,
    event::{Key, KeyEvent, KeyModifiers},
};

use crate::app::{
    messages::{JobMsg, MenuMsg, Msg, StatusMsg, SystemSelectMsg, View},
    user_events::{CscsEvent, FileEvent, UserEvent},
};

#[derive(Default, Component)]
pub struct GlobalListener {
    component: Phantom,
    current_view: View,
}

impl AppComponent<Msg, UserEvent> for GlobalListener {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
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
            Event::User(UserEvent::Error(msg)) => Some(Msg::Error(msg.to_owned())),
            Event::User(UserEvent::Info(msg)) => Some(Msg::Info(msg.to_owned())),
            Event::User(UserEvent::Cscs(CscsEvent::LoggedIn)) => {
                Some(Msg::Status(StatusMsg::Info("Successfully logged in".to_string())))
            }
            Event::User(UserEvent::Cscs(CscsEvent::SelectSystemList(systems))) => {
                Some(Msg::SystemSelectPopup(SystemSelectMsg::Opened(systems.to_owned())))
            }
            Event::User(UserEvent::Cscs(CscsEvent::GotJobDetails(details))) => {
                Some(Msg::Job(JobMsg::Details(details.to_owned())))
            }
            Event::User(UserEvent::File(FileEvent::DownloadSuccessful)) => {
                Some(Msg::Status(StatusMsg::Info("File successfully downloaded".to_owned())))
            }
            _ => None,
        }
    }
}
