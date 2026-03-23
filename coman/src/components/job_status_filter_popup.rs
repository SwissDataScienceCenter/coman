use strum::{VariantArray, VariantNames};
use tui_realm_stdlib::Checkbox;
use tuirealm::{
    Attribute, Component, Event, MockComponent, State,
    command::{Cmd, CmdResult, Direction},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, PropValue},
};

use crate::{
    app::{
        messages::{JobFilterPopupMsg, Msg},
        user_events::UserEvent,
    },
    cscs::api_client::types::JobStatus,
};

#[derive(MockComponent)]
pub struct JobStatusFilterPopup {
    component: Checkbox,
    values: Vec<usize>,
    max_idx: usize,
}

impl JobStatusFilterPopup {
    pub fn new() -> Self {
        let mut variants: Vec<_> = <JobStatus as VariantNames>::VARIANTS
            .iter()
            .map(|v| v.to_owned())
            .collect();
        variants.insert(0, "All");
        let max_idx = variants.len();
        let values: Vec<_> = (0..max_idx).collect();
        Self {
            component: Checkbox::default()
                .borders(Borders::default().modifiers(BorderType::Thick).color(Color::Green))
                .title("Select Status to show", Alignment::Left)
                .rewind(true)
                .choices(variants)
                .values(&values),
            values,
            max_idx,
        }
    }
}

impl Component<Msg, UserEvent> for JobStatusFilterPopup {
    fn on(&mut self, ev: Event<UserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent { code: Key::Right, .. }) => self.perform(Cmd::Move(Direction::Right)),
            Event::Keyboard(KeyEvent { code: Key::Left, .. }) => self.perform(Cmd::Move(Direction::Left)),
            Event::Keyboard(KeyEvent {
                code: Key::Char(' '), ..
            }) => {
                if let CmdResult::Changed(state) = self.perform(Cmd::Toggle)
                    && let State::Vec(state) = state
                {
                    let new_state: Vec<_> = state.into_iter().map(|s| s.unwrap_usize()).collect();
                    if new_state.contains(&0) && !self.values.contains(&0) {
                        // 'All' selected, activate all
                        self.values = (0..self.max_idx).collect();
                    } else if !new_state.contains(&0) && self.values.contains(&0) {
                        // 'All' deselected, deactivate all
                        self.values = vec![];
                    } else {
                        // check if all values except 'All' are selected
                        self.values = new_state;
                        if (1..self.max_idx).all(|s| self.values.contains(&s)) {
                            self.values.push(0);
                        } else if self.values.contains(&0) {
                            self.values.retain(|v| *v != 0);
                        }
                    }
                    self.attr(
                        Attribute::Value,
                        tuirealm::AttrValue::Payload(tuirealm::props::PropPayload::Vec(
                            self.values.iter().map(|v| PropValue::Usize(*v)).collect(),
                        )),
                    );
                }
                CmdResult::None
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::JobFilterPopup(JobFilterPopupMsg::Closed));
            }
            Event::Keyboard(KeyEvent { code: Key::Enter, .. }) => {
                let selected_statuses = self
                    .values
                    .iter()
                    .filter(|v| **v > 0)
                    .map(|v| <JobStatus as VariantArray>::VARIANTS[*v - 1].clone())
                    .collect();

                return Some(Msg::JobFilterPopup(JobFilterPopupMsg::FilterSelected(
                    selected_statuses,
                )));
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
