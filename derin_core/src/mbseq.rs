// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::iter::{ExactSizeIterator, FromIterator};

use derin_common_types::buttons::{MouseButton, MOUSE_INT_MASK, MOUSE_INT_MASK_LEN, NUM_MOUSE_BUTTONS};
use arrayvec::{ArrayVec, IntoIter};
use crate::{
    cgmath::Point2,
    event::MouseDown,
    widget::WidgetId,
};

type PointArray = [(Point2<i32>, WidgetId); MOUSE_INT_MASK_LEN as usize];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MouseButtonSequenceTrackPos {
    seq: MouseButtonSequence,
    down_positions: ArrayVec<PointArray>
}

pub(crate) struct MouseButtonSeqTrackPosIter {
    seq_iter: MouseButtonSeqIter,
    pos_iter: IntoIter<PointArray>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButtonSequence {
    buttons: u16,
    len: u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButtonSeqIter {
    buttons: u16,
    len: u8
}

pub(crate) struct MouseDownWithId {
    pub mouse_down: MouseDown,
    pub widget_id: WidgetId,
}

impl MouseButtonSequenceTrackPos {
    pub fn new() -> MouseButtonSequenceTrackPos {
        MouseButtonSequenceTrackPos {
            seq: MouseButtonSequence::new(),
            down_positions: ArrayVec::new()
        }
    }

    pub fn push_button(&mut self, button: MouseButton, pos: Point2<i32>, widget_id: WidgetId) -> &mut MouseButtonSequenceTrackPos {
        self.release_button(button);
        self.seq.push_button(button);
        self.down_positions.push((pos, widget_id));
        self
    }

    pub fn release_button(&mut self, button: MouseButton) -> &mut MouseButtonSequenceTrackPos {
        for (i, b) in self.seq.into_iter().enumerate() {
            if b == button {
                self.down_positions.remove(i);
                break;
            }
        }
        self.seq.release_button(button);
        self
    }

    pub fn contains(&self, button: MouseButton) -> Option<MouseDownWithId> {
        self.seq.into_iter().enumerate().find(|&(_, b)| b == button)
            .map(|(i, b)| MouseDownWithId {
                mouse_down: MouseDown {
                    button: b,
                    down_pos: self.down_positions[i].0
                },
                widget_id: self.down_positions[i].1,
            })
    }

    #[inline]
    pub fn len(&self) -> u8 {
        self.seq.len()
    }
}

impl MouseButtonSequence {
    #[inline]
    pub fn new() -> MouseButtonSequence {
        assert!(MOUSE_INT_MASK_LEN as u32 * NUM_MOUSE_BUTTONS as u32 <= 16);

        MouseButtonSequence {
            buttons: 0,
            len: 0
        }
    }

    pub fn push_button(&mut self, button: MouseButton) -> &mut MouseButtonSequence {
        self.release_button(button);

        self.buttons |= (u8::from(button) as u16) << (self.len as u16 * MOUSE_INT_MASK_LEN);
        self.len += 1;
        self
    }

    pub fn release_button(&mut self, button: MouseButton) -> &mut MouseButtonSequence {
        for (i, b) in self.into_iter().enumerate() {
            if b == button {
                let postfix_bitmask = (!0) << ((i as u16 + 1) * MOUSE_INT_MASK_LEN);
                let postfix_bits = (self.buttons & postfix_bitmask) >> MOUSE_INT_MASK_LEN;

                self.buttons &= !((!0) << ((i as u16) * MOUSE_INT_MASK_LEN));
                self.buttons |= postfix_bits;
                self.len -= 1;
                break;
            }
        }
        self
    }

    // pub fn contains(&self, button: MouseButton) -> bool {
    //     self.into_iter().find(|b| *b == button).is_some()
    // }

    // #[inline]
    // pub fn get(&mut self, index: u8) -> Option<MouseButton> {
    //     if self.len < index {
    //         let val = (self.buttons >> (index as u16 * MOUSE_INT_MASK_LEN)) & MOUSE_INT_MASK;
    //         Some(MouseButton::from_u8(val as u8).unwrap())
    //     } else {
    //         None
    //     }
    // }

    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }
}

impl IntoIterator for MouseButtonSequence {
    type Item = MouseButton;
    type IntoIter = MouseButtonSeqIter;

    #[inline]
    fn into_iter(self) -> MouseButtonSeqIter {
        MouseButtonSeqIter {
            buttons: self.buttons,
            len: self.len
        }
    }
}

impl FromIterator<MouseButton> for MouseButtonSequence {
    #[inline]
    fn from_iter<T>(iter: T) -> MouseButtonSequence
        where T: IntoIterator<Item=MouseButton>
    {
        let mut seq = MouseButtonSequence::new();
        for button in iter {
            seq.push_button(button);
        }
        seq
    }
}

impl IntoIterator for MouseButtonSequenceTrackPos {
    type Item = MouseDownWithId;
    type IntoIter = MouseButtonSeqTrackPosIter;

    #[inline]
    fn into_iter(self) -> MouseButtonSeqTrackPosIter {
        MouseButtonSeqTrackPosIter {
            seq_iter: self.seq.into_iter(),
            pos_iter: self.down_positions.into_iter()
        }
    }
}

impl Iterator for MouseButtonSeqIter {
    type Item = MouseButton;

    #[inline]
    fn next(&mut self) -> Option<MouseButton> {
        match self.len {
            0 => None,
            _ => {
                let val = self.buttons & MOUSE_INT_MASK;
                self.len -= 1;
                self.buttons >>= MOUSE_INT_MASK_LEN;
                Some(MouseButton::from_u8(val as u8).unwrap())
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len as usize, Some(self.len as usize))
    }
}

impl ExactSizeIterator for MouseButtonSeqIter {}

impl Iterator for MouseButtonSeqTrackPosIter {
    type Item = MouseDownWithId;

    #[inline]
    fn next(&mut self) -> Option<MouseDownWithId> {
        let (down_pos, widget_id) = self.pos_iter.next()?;
        Some(MouseDownWithId {
            mouse_down: MouseDown {
                button: self.seq_iter.next()?,
                down_pos,
            },
            widget_id,
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.seq_iter.size_hint()
    }
}

impl ExactSizeIterator for MouseButtonSeqTrackPosIter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_buttons() {
        use self::MouseButton::*;
        let mut seq = MouseButtonSequence::new();
        seq.push_button(Left);
        seq.push_button(Middle);
        seq.push_button(Right);
        seq.push_button(Middle);

        assert_eq!(3, seq.len());
        assert_eq!(&[Left, Right, Middle], &*seq.into_iter().collect::<Vec<_>>());
    }

    #[test]
    fn release_buttons() {
        use self::MouseButton::*;
        let mut seq = MouseButtonSequence::new();
        seq.push_button(Left);
        seq.push_button(X1);
        seq.push_button(X2);
        seq.push_button(Middle);
        seq.push_button(Right);

        seq.release_button(X2);

        assert_eq!(4, seq.len());
        assert_eq!(&[Left, X1, Middle, Right], &*seq.into_iter().collect::<Vec<_>>());
    }
}
