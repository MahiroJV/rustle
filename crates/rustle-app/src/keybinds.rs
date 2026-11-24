//! Keyboard shortcuts and mouse buttons, all in one place.

use iced::keyboard::key::Named;
use iced::keyboard::{self, Key, Modifiers};
use iced::{Event, event, mouse, window};

use crate::message::Message;

/// Turns every raw window event into a message (or nothing).
/// Used with `iced::event::listen_with` in main.rs.
pub fn on_event(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            physical_key,
            modifiers,
            ..
        }) => {
            // A focused text field already used this key (e.g. Backspace
            // while typing a path), so it must not also navigate.
            if status == event::Status::Captured {
                return None;
            }
            // `to_latin` gives the Latin letter of the physical key, so
            // Ctrl+T works even with a Russian or Azerbaijani layout.
            shortcut(&key, key.to_latin(physical_key), modifiers)
        }
        Event::Mouse(mouse::Event::ButtonPressed(button)) => mouse_button(button),
        _ => None,
    }
}

/// The keyboard shortcut table. `latin` is the key as a Latin character
/// (lowercase is enough), if it is a character key.
pub fn shortcut(key: &Key, latin: Option<char>, mods: Modifiers) -> Option<Message> {
    let ctrl = mods.command(); // Ctrl on Linux/Windows, Cmd on macOS
    let shift = mods.shift();
    let alt = mods.alt();

    if let Key::Named(named) = key {
        return match (named, ctrl, shift, alt) {
            (Named::Tab, true, false, false) => Some(Message::NextTab),
            (Named::Tab, true, true, false) => Some(Message::PrevTab),
            (Named::ArrowLeft, false, false, true) => Some(Message::Back),
            (Named::ArrowRight, false, false, true) => Some(Message::Forward),
            (Named::ArrowUp, false, false, true) => Some(Message::Up),
            (Named::Backspace, false, false, false) => Some(Message::Up),
            (Named::F5, false, false, false) => Some(Message::Refresh),
            _ => None,
        };
    }

    if !ctrl || alt {
        return None;
    }
    match latin?.to_ascii_lowercase() {
        't' if !shift => Some(Message::NewTab(None)),
        'w' if !shift => Some(Message::CloseActiveTab),
        'l' if !shift => Some(Message::FocusLocation),
        'h' if !shift => Some(Message::ToggleHidden),
        ',' => Some(Message::OpenSettings),
        _ => None,
    }
}

/// The extra buttons on the side of many mice.
pub fn mouse_button(button: mouse::Button) -> Option<Message> {
    match button {
        mouse::Button::Back => Some(Message::Back),
        mouse::Button::Forward => Some(Message::Forward),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(c: &str) -> Key {
        Key::Character(c.into())
    }

    #[test]
    fn ctrl_letters() {
        assert_eq!(shortcut(&ch("t"), Some('t'), Modifiers::CTRL), Some(Message::NewTab(None)));
        assert_eq!(shortcut(&ch("w"), Some('w'), Modifiers::CTRL), Some(Message::CloseActiveTab));
        assert_eq!(shortcut(&ch("h"), Some('h'), Modifiers::CTRL), Some(Message::ToggleHidden));
        assert_eq!(shortcut(&ch(","), Some(','), Modifiers::CTRL), Some(Message::OpenSettings));
    }

    #[test]
    fn other_keyboard_layout_still_works() {
        // On a Russian layout the T key types "е", but its Latin letter is 't'.
        assert_eq!(shortcut(&ch("е"), Some('t'), Modifiers::CTRL), Some(Message::NewTab(None)));
    }

    #[test]
    fn letters_without_ctrl_do_nothing() {
        assert_eq!(shortcut(&ch("t"), Some('t'), Modifiers::empty()), None);
    }

    #[test]
    fn navigation_keys() {
        let named = |n| Key::Named(n);
        assert_eq!(shortcut(&named(Named::ArrowLeft), None, Modifiers::ALT), Some(Message::Back));
        assert_eq!(shortcut(&named(Named::ArrowRight), None, Modifiers::ALT), Some(Message::Forward));
        assert_eq!(shortcut(&named(Named::Backspace), None, Modifiers::empty()), Some(Message::Up));
        assert_eq!(shortcut(&named(Named::Tab), None, Modifiers::CTRL | Modifiers::SHIFT), Some(Message::PrevTab));
        // Plain arrow keys are left alone (they'll move the selection later).
        assert_eq!(shortcut(&named(Named::ArrowLeft), None, Modifiers::empty()), None);
    }

    #[test]
    fn mouse_side_buttons() {
        assert_eq!(mouse_button(mouse::Button::Back), Some(Message::Back));
        assert_eq!(mouse_button(mouse::Button::Forward), Some(Message::Forward));
        assert_eq!(mouse_button(mouse::Button::Left), None);
    }
}
