// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cgmath::Point2;
use cgmath_geometry::BoundBox;

use tree::{WidgetID, Widget, WidgetIdent};
use render::RenderFrame;

use std::collections::HashMap;

id!(pub PopupID);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupAttributes {
    pub rect: BoundBox<Point2<i32>>,
    pub title: String,
    pub decorations: bool,
    // pub tool_window: bool,
    // pub focusable: bool,
    pub ident: WidgetIdent
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupSummary {
    pub id: PopupID,
    pub attributes: PopupAttributes
}

impl Default for PopupAttributes {
    #[inline]
    fn default() -> PopupAttributes {
        PopupAttributes {
            rect: BoundBox::new2(0, 0, 0, 0),
            title: String::new(),
            decorations: true,
            // tool_window: false,
            // focusable: true,
            ident: WidgetIdent::Num(0)
        }
    }
}

enum Removed {
    Popup(PopupID),
    Owner(WidgetID)
}

pub(crate) struct PopupMap<A, F: RenderFrame> {
    popups: HashMap<PopupID, PopupWidget<A, F>>,
    owners: HashMap<WidgetID, HashMap<WidgetIdent, PopupID>>,
    removed: Vec<Removed>
}

pub(crate) struct PopupWidget<A, F: RenderFrame> {
    pub widget: Box<Widget<A, F>>,
    pub mouse_pos: Point2<i32>,
    pub needs_redraw: bool,
    pub owner_id: WidgetID,
    pub ident: WidgetIdent
}

pub struct ChildPopupsMut<'a, A: 'a, F: 'a + RenderFrame> {
    valid_popups: &'a mut HashMap<WidgetIdent, PopupID>,
    popup_map: &'a mut HashMap<PopupID, PopupWidget<A, F>>,
    removed: &'a mut Vec<Removed>
}

impl<A, F: RenderFrame> PopupMap<A, F> {
    pub fn new() -> PopupMap<A, F> {
        PopupMap {
            popups: HashMap::new(),
            owners: HashMap::new(),
            removed: Vec::new()
        }
    }

    pub fn insert(&mut self, owner_id: WidgetID, ident: WidgetIdent, widget: Box<Widget<A, F>>) -> PopupID {
        let ident_map = self.owners.entry(owner_id).or_insert(HashMap::new());
        let popup_id = *ident_map.entry(ident.clone()).or_insert(PopupID::new());
        match self.popups.get_mut(&popup_id) {
            Some(popup) => popup.widget = widget,
            None => {
                self.popups.insert(popup_id, PopupWidget {
                    widget,
                    mouse_pos: Point2::new(0, 0),
                    needs_redraw: true,
                    owner_id,
                    ident
                });
            }
        }

        popup_id
    }

    // pub fn get(&self, popup_id: PopupID) -> Option<&PopupWidget<A, F>> {
    //     self.popups.get(&popup_id)
    // }

    // pub fn get_mut(&mut self, popup_id: PopupID) -> Option<&mut PopupWidget<A, F>> {
    //     self.popups.get_mut(&popup_id)
    // }

    pub fn popups_owned_by_mut(&mut self, owner_id: WidgetID) -> Option<ChildPopupsMut<A, F>> {
        let PopupMap {
            ref mut popups,
            ref mut owners,
            ref mut removed
        } = *self;

        owners.get_mut(&owner_id).map(move |valid_popups| ChildPopupsMut {
            valid_popups, removed,
            popup_map: popups
        })
    }

    pub fn remove(&mut self, popup_id: PopupID) -> Option<PopupWidget<A, F>> {
        let popup = self.popups.remove(&popup_id)?;
        let owner_popups = self.owners.get_mut(&popup.owner_id).unwrap();
        owner_popups.remove(&popup.ident);
        if owner_popups.len() == 0 {
            self.owners.remove(&popup.owner_id);
        }

        Some(popup)
    }

    pub fn take(&mut self, popup_id: PopupID) -> Option<PopupWidget<A, F>> {
        let popup = self.popups.remove(&popup_id)?;
        self.owners.get_mut(&popup.owner_id).unwrap().remove(&popup.ident);

        Some(popup)
    }

    pub fn replace(&mut self, popup_id: PopupID, popup: PopupWidget<A, F>) {
        self.owners.get_mut(&popup.owner_id).unwrap().insert(popup.ident.clone(), popup_id);
        self.popups.insert(popup_id, popup);
    }

    pub fn popups_removed_by_children<'a>(&'a mut self) -> impl 'a + Iterator<Item=PopupID> {
        let PopupMap {
            ref mut owners,
            ref mut removed,
            ..
        } = *self;

        removed.drain(..).filter_map(move |remove| match remove {
            Removed::Popup(popup_id) => Some(popup_id),
            Removed::Owner(owner_id) => {
                owners.remove(&owner_id);
                None
            }
        })
    }

    pub(crate) fn popups_mut<'a>(&'a mut self) -> impl 'a + Iterator<Item=(PopupID, &'a mut PopupWidget<A, F>)> {
        self.popups.iter_mut().map(|(i, p)| (*i, p))
    }
}

impl<'a, A, F: RenderFrame> ChildPopupsMut<'a, A, F> {
    pub fn idents<'b>(&'b self) -> impl 'b + Iterator<Item=WidgetIdent> {
        self.valid_popups.keys().cloned()
    }

    pub fn get(&self, ident: WidgetIdent) -> Option<&Widget<A, F>> {
        match self.valid_popups.get(&ident) {
            Some(popup_id) => self.popup_map.get(popup_id).map(|p| &*p.widget),
            None => None
        }
    }

    pub fn get_mut(&mut self, ident: WidgetIdent) -> Option<&mut Widget<A, F>> {
        match self.valid_popups.get(&ident) {
            Some(popup_id) => Some(&mut*self.popup_map.get_mut(popup_id).unwrap().widget),
            None => None
        }
    }

    pub fn remove(&mut self, ident: WidgetIdent) -> Option<Box<Widget<A, F>>> {
        match self.valid_popups.remove(&ident) {
            Some(popup_id) => {
                let popup_removed = self.popup_map.remove(&popup_id).unwrap();
                self.removed.push(Removed::Popup(popup_id));

                if self.valid_popups.len() == 0 {
                    self.removed.push(Removed::Owner(popup_removed.owner_id));
                }

                Some(popup_removed.widget)
            }
            None => None
        }
    }
}
