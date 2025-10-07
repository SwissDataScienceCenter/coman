use std::collections::HashSet;

use crate::action::Action;
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Default, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Focus {
    #[default]
    Inactive,
    Active,
    Permanent,
    PermanentInactive,
    Exclusive,
}

#[derive(Debug, Clone)]
pub struct FocusManager {
    current_focus: String,
    previous_focus: Vec<(String, bool)>,
    exclusive: bool,
}

impl FocusManager {
    pub fn new(initial_focus: String) -> Self {
        Self {
            current_focus: initial_focus.clone(),
            previous_focus: vec![(initial_focus, false)],
            exclusive: false,
        }
    }

    pub fn request_focus(&mut self, component_id: String, focus: Focus) -> Option<Action> {
        match (focus, self.exclusive) {
            (Focus::Inactive, _) => {
                if self.current_focus == component_id {
                    self.release_focus(component_id)
                } else {
                    Some(Action::FocusChanged(component_id, Focus::Inactive))
                }
            }
            (Focus::Active, true) => None,
            (Focus::Active, false) => {
                if self.current_focus != component_id {
                    self.previous_focus
                        .push((self.current_focus.clone(), self.exclusive));
                    self.current_focus = component_id;
                    Some(Action::FocusChanged(
                        self.current_focus.clone(),
                        Focus::Active,
                    ))
                } else {
                    None
                }
            }
            (Focus::Permanent, true) | (Focus::PermanentInactive, _) => {
                Some(Action::FocusChanged(component_id, Focus::PermanentInactive))
            }
            (Focus::Permanent, false) => Some(Action::FocusChanged(component_id, Focus::Permanent)),
            (Focus::Exclusive, _) => {
                if self.current_focus != component_id {
                    self.previous_focus
                        .push((self.current_focus.clone(), self.exclusive));
                    self.current_focus = component_id;
                }
                self.exclusive = true;
                Some(Action::FocusChanged(
                    self.current_focus.clone(),
                    Focus::Exclusive,
                ))
            }
        }
    }
    pub fn release_focus(&mut self, component_id: String) -> Option<Action> {
        if self.current_focus != component_id {
            self.previous_focus.pop_if(|(id, _)| *id == component_id);
            return Some(Action::FocusChanged(component_id, Focus::Inactive));
        }
        (self.current_focus, self.exclusive) = self
            .previous_focus
            .pop()
            .unwrap_or((self.current_focus.clone(), self.exclusive));
        if self.exclusive {
            Some(Action::FocusChanged(
                self.current_focus.clone(),
                Focus::Exclusive,
            ))
        } else {
            Some(Action::FocusChanged(
                self.current_focus.clone(),
                Focus::Active,
            ))
        }
    }

    pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let action = match action {
            Action::RequestFocus(component_id, focus) => self.request_focus(component_id, focus),
            Action::ReleaseFocus(component_id) => self.release_focus(component_id),
            _ => None,
        };

        Ok(action)
    }

    pub fn should_receive_event(&self, component_id: String) -> bool {
        if self.exclusive {
            self.current_focus == component_id
        } else {
            true
        }
    }
}
