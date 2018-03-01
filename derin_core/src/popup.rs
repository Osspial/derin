use cgmath::Point2;
use cgmath_geometry::BoundBox;

use tree::{NodeID, Node, NodeIdent};
use render::RenderFrame;

use std::collections::HashMap;

id!(pub PopupID);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupAttributes {
    pub rect: BoundBox<Point2<i32>>,
    pub title: String,
    pub decorations: bool,
    pub tool_window: bool,
    pub focusable: bool,
    pub ident: NodeIdent
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
            tool_window: false,
            focusable: true,
            ident: NodeIdent::Num(0)
        }
    }
}

enum Removed {
    Popup(PopupID),
    Owner(NodeID)
}

pub(crate) struct PopupMap<A, F: RenderFrame> {
    popups: HashMap<PopupID, PopupNode<A, F>>,
    owners: HashMap<NodeID, HashMap<NodeIdent, PopupID>>,
    removed: Vec<Removed>
}

pub(crate) struct PopupNode<A, F: RenderFrame> {
    pub node: Box<Node<A, F>>,
    pub mouse_pos: Point2<i32>,
    pub owner_id: NodeID,
    pub ident: NodeIdent
}

pub struct ChildPopupsMut<'a, A: 'a, F: 'a + RenderFrame> {
    valid_popups: &'a mut HashMap<NodeIdent, PopupID>,
    popup_map: &'a mut HashMap<PopupID, PopupNode<A, F>>,
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

    pub fn insert(&mut self, owner_id: NodeID, ident: NodeIdent, node: Box<Node<A, F>>) -> PopupID {
        let ident_map = self.owners.entry(owner_id).or_insert(HashMap::new());
        let popup_id = *ident_map.entry(ident).or_insert(PopupID::new());
        match self.popups.get_mut(&popup_id) {
            Some(popup) => popup.node = node,
            None => {
                self.popups.insert(popup_id, PopupNode {
                    node,
                    mouse_pos: Point2::new(0, 0),
                    owner_id,
                    ident
                });
            }
        }

        popup_id
    }

    // pub fn get(&self, popup_id: PopupID) -> Option<&PopupNode<A, F>> {
    //     self.popups.get(&popup_id)
    // }

    // pub fn get_mut(&mut self, popup_id: PopupID) -> Option<&mut PopupNode<A, F>> {
    //     self.popups.get_mut(&popup_id)
    // }

    pub fn popups_owned_by_mut(&mut self, owner_id: NodeID) -> Option<ChildPopupsMut<A, F>> {
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

    pub fn remove(&mut self, popup_id: PopupID) -> Option<PopupNode<A, F>> {
        let popup = self.popups.remove(&popup_id)?;
        let owner_popups = self.owners.get_mut(&popup.owner_id).unwrap();
        owner_popups.remove(&popup.ident);
        if owner_popups.len() == 0 {
            self.owners.remove(&popup.owner_id);
        }

        Some(popup)
    }

    pub fn take(&mut self, popup_id: PopupID) -> Option<PopupNode<A, F>> {
        let popup = self.popups.remove(&popup_id)?;
        self.owners.get_mut(&popup.owner_id).unwrap().remove(&popup.ident);

        Some(popup)
    }

    pub fn replace(&mut self, popup_id: PopupID, popup: PopupNode<A, F>) {
        self.owners.get_mut(&popup.owner_id).unwrap().insert(popup.ident, popup_id);
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
}

impl<'a, A, F: RenderFrame> ChildPopupsMut<'a, A, F> {
    pub fn idents<'b>(&'b self) -> impl 'b + Iterator<Item=NodeIdent> {
        self.valid_popups.keys().cloned()
    }

    pub fn get(&self, ident: NodeIdent) -> Option<&Node<A, F>> {
        match self.valid_popups.get(&ident) {
            Some(popup_id) => self.popup_map.get(popup_id).map(|p| &*p.node),
            None => None
        }
    }

    pub fn get_mut(&mut self, ident: NodeIdent) -> Option<&mut Node<A, F>> {
        match self.valid_popups.get(&ident) {
            Some(popup_id) => Some(&mut*self.popup_map.get_mut(popup_id).unwrap().node),
            None => None
        }
    }

    pub fn remove(&mut self, ident: NodeIdent) -> Option<Box<Node<A, F>>> {
        match self.valid_popups.remove(&ident) {
            Some(popup_id) => {
                let popup_removed = self.popup_map.remove(&popup_id).unwrap();
                self.removed.push(Removed::Popup(popup_id));

                if self.valid_popups.len() == 0 {
                    self.removed.push(Removed::Owner(popup_removed.owner_id));
                }

                Some(popup_removed.node)
            }
            None => None
        }
    }
}
