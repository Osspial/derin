use crate::tree::{WidgetID, WidgetIdent};
use std::collections::{
    VecDeque,
    hash_map::{HashMap, Entry}
};
use fnv::FnvBuildHasher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WidgetInsertError {
    ParentNotInTree,
    /// Returned if we tried to insert a widget that's the root widget.
    ///
    /// This in bad because completing the operation would result in there being no root widget!
    WidgetIsRoot
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WidgetRelationError {
    WidgetNotFound,
    RelationNotFound
}

#[derive(Debug, PartialEq, Eq)]
struct WidgetTreeNode {
    parent_id: WidgetID,
    children: Vec<WidgetID>,
    data: WidgetData
}

#[derive(Debug, PartialEq, Eq)]
pub struct WidgetData {
    pub ident: WidgetIdent
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct WidgetTree {
    root: WidgetID,
    root_data: WidgetData,
    root_children: Vec<WidgetID>,
    tree_data: HashMap<WidgetID, WidgetTreeNode, FnvBuildHasher>
}

fn find_index<T: PartialEq>(s: &[T], element: &T) -> usize {
    s.iter().enumerate().find(|&(_, e)| e == element).unwrap().0
}

fn vec_remove_element<T: PartialEq>(v: &mut Vec<T>, element: &T) -> T {
    v.remove(find_index(v, element))
}

impl WidgetTree {
    pub(crate) fn new(root: WidgetID, root_ident: WidgetIdent) -> WidgetTree {
        WidgetTree {
            root,
            root_data: WidgetData{ ident: root_ident },
            root_children: Vec::new(),
            tree_data: HashMap::default()
        }
    }

    /// Insert a widget ID into the tree. If the widget in already in the tree, change the widget's
    /// parent to the new parent.
    pub(crate) fn insert(&mut self, parent_id: WidgetID, widget_id: WidgetID, child_index: usize, widget_ident: WidgetIdent) -> Result<(), WidgetInsertError> {
        if widget_id == self.root {
            return Err(WidgetInsertError::WidgetIsRoot);
        }

        if let Some((_, children)) = self.get_widget_node_mut(parent_id) {

            children.insert(child_index, widget_id);

            match self.tree_data.entry(widget_id) {
                Entry::Occupied(mut occ) => {
                    let node = occ.get_mut();

                    let old_parent_id = node.parent_id;
                    node.parent_id = parent_id;
                    node.data.ident = widget_ident;

                    let (_, old_parent_children) = self.get_widget_node_mut(old_parent_id).expect("Bad tree state");
                    vec_remove_element(old_parent_children, &widget_id);
                },
                Entry::Vacant(vac) => {
                    vac.insert(WidgetTreeNode::new(parent_id, widget_ident));
                }
            }
            Ok(())
        } else {
            Err(WidgetInsertError::ParentNotInTree)
        }
    }

    pub(crate) fn remove(&mut self, widget_id: WidgetID) -> Option<WidgetData> {
        if let Entry::Occupied(occ) = self.tree_data.entry(widget_id) {
            let node = occ.remove();
            vec_remove_element(&mut self.get_widget_node_mut(node.parent_id).unwrap().1, &widget_id);

            // Remove all the child widgets.
            let mut widgets_to_remove = VecDeque::from(node.children);
            while let Some(remove_id) = widgets_to_remove.pop_front() {
                let removed_node = match self.tree_data.entry(remove_id) {
                    Entry::Occupied(occ) => occ.remove(),
                    Entry::Vacant(_) => panic!("Bad tree state")
                };
                widgets_to_remove.extend(removed_node.children);
            }

            Some(node.data)
        } else {
            None
        }
    }

    pub(crate) fn parent(&self, widget_id: WidgetID) -> Result<WidgetID, WidgetRelationError> {
        if widget_id == self.root {
            Err(WidgetRelationError::RelationNotFound)
        } else if let Some(node) = self.tree_data.get(&widget_id) {
            Ok(node.parent_id)
        } else {
            Err(WidgetRelationError::WidgetNotFound)
        }
    }

    pub(crate) fn sibling(&self, widget_id: WidgetID, offset: isize) -> Result<WidgetID, WidgetRelationError> {
        if widget_id == self.root {
            return if offset == 0 {
                Ok(self.root)
            } else {
                Err(WidgetRelationError::RelationNotFound)
            };
        }

        let node = self.tree_data.get(&widget_id).ok_or(WidgetRelationError::WidgetNotFound)?;

        // We have to do this check after getting the node so the proper error is returned if the
        // widget isn't in the tree.
        if offset == 0 {
            return Ok(widget_id);
        }

        let siblings = &self.get_widget_node(node.parent_id).unwrap().1;

        let sibling_index = find_index(&siblings, &widget_id) as isize + offset;
        siblings.get(sibling_index as usize).cloned().ok_or(WidgetRelationError::RelationNotFound)
    }

    pub(crate) fn sibling_wrapping(&self, widget_id: WidgetID, offset: isize) -> Option<WidgetID> {
        if widget_id == self.root {
            return Some(self.root);
        }

        let node = self.tree_data.get(&widget_id)?;

        // We have to do this check after getting the node so the proper error is returned if the
        // widget isn't in the tree.
        if offset == 0 {
            return Some(widget_id);
        }

        let siblings = &self.get_widget_node(node.parent_id).unwrap().1;

        let mod_euc = |i, rhs| {
            let r = i % rhs;
            if r < 0 {
                if rhs < 0 {
                    r - rhs
                } else {
                    r + rhs
                }
            } else {
                r
            }
        };

        let sibling_index = find_index(siblings, &widget_id) as isize + offset;
        Some(siblings[mod_euc(sibling_index, siblings.len() as isize) as usize])
    }

    pub(crate) fn child_from_start(&self, widget_id: WidgetID, child_index: usize) -> Result<WidgetID, WidgetRelationError> {
        let children = self.get_widget_node(widget_id).ok_or(WidgetRelationError::WidgetNotFound)?.1;

        children.get(child_index).cloned().ok_or(WidgetRelationError::RelationNotFound)
    }

    // pub(crate) fn child_from_end(&self, widget_id: WidgetID, offset: usize) -> Option<WidgetID> {unimplemented!()}

    pub(crate) fn children(&self, widget_id: WidgetID) -> Option<impl Iterator<Item=(WidgetID, &'_ WidgetData)>> {
        Some(self.children_nodes(widget_id)?.map(|(id, node)| (id, &node.data)))
    }

    fn children_nodes(&self, widget_id: WidgetID) -> Option<impl Iterator<Item=(WidgetID, &'_ WidgetTreeNode)>> {
        let (_, children) = self.get_widget_node(widget_id)?;
        Some(children.iter().map(move |c| (*c, self.tree_data.get(c).expect("Bad tree state"))))
    }

    fn all_nodes(&self) -> impl Iterator<Item=(WidgetID, &'_ WidgetData)> {
        Some((self.root, &self.root_data)).into_iter().chain(self.tree_data.iter().map(|(&k, v)| (k, &v.data)))
    }

    pub(crate) fn get_widget(&self, id: WidgetID) -> Option<&WidgetData> {
        self.get_widget_node(id).map(|(d, _)| d)
    }

    /// Returns `Option<WidgetData, Children>`
    fn get_widget_node(&self, id: WidgetID) -> Option<(&WidgetData, &[WidgetID])> {
        if self.root == id {
            Some((&self.root_data, &self.root_children))
        } else {
            self.tree_data.get(&id).map(|n| (&n.data, &n.children[..]))
        }
    }

    fn get_widget_node_mut(&mut self, id: WidgetID) -> Option<(&mut WidgetData, &mut Vec<WidgetID>)> {
        if self.root == id {
            Some((&mut self.root_data, &mut self.root_children))
        } else {
            self.tree_data.get_mut(&id).map(|n| (&mut n.data, &mut n.children))
        }
    }

    /// Gets the identifier chain of the widget, starting with the widget's identifier and ending
    /// with the root identifier.
    pub(crate) fn ident_chain_reversed(&self, id: WidgetID) -> Option<impl Iterator<Item=&'_ WidgetIdent>> {
        struct ClosureIterator<'a, F>(F)
            where F: FnMut() -> Option<&'a WidgetIdent>;
        impl<'a, F> Iterator for ClosureIterator<'a, F>
            where F: FnMut() -> Option<&'a WidgetIdent>
        {
            type Item = &'a WidgetIdent;
            fn next(&mut self) -> Option<&'a WidgetIdent> {
                (self.0)()
            }
        }

        let get_widget_and_parent = move |id| {
            if self.root == id {
                Some((&self.root_data.ident, None))
            } else if let Some(node) = self.tree_data.get(&id) {
                Some((&node.data.ident, Some(node.parent_id)))
            } else {
                None
            }
        };

        let mut finished = false;
        let (mut ident, mut parent_id_opt) = get_widget_and_parent(id)?;
        Some(ClosureIterator(move || {
            if finished {
                return None;
            }

            let old_ident = ident;
            if let Some(parent_id) = parent_id_opt {
                let (p_ident, p_id) = get_widget_and_parent(parent_id)?;
                ident = p_ident;
                parent_id_opt = p_id;
            } else {
                finished = true;
            }
            Some(old_ident)
        }))
    }
}

impl WidgetTreeNode {
    fn new(parent_id: WidgetID, ident: WidgetIdent) -> WidgetTreeNode {
        WidgetTreeNode {
            parent_id,
            children: Vec::new(),
            data: WidgetData { ident }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    macro_rules! extract_tree_idents {
        ($(
            $root:ident $(in $old:ident)* $({$($rest:tt)*})*
        ),*) => {$(
            if_tokens!{($($old)*) {} else {let $root = WidgetID::new();}}

            extract_tree_idents!{$($($rest)*)*}
        )*};
    }

    macro_rules! widget_tree {
        (
            let $tree_ident:pat = $root:ident $(in $old:ident)* $({$($rest:tt)*})*
        ) => {
            extract_tree_idents!{$root $(in $old)* $({$($rest)*})*}
            let $tree_ident = {
                #[allow(unused_mut)]
                {
                    let root_id = $root;
                    let mut tree = WidgetTree::new(root_id, WidgetIdent::Str(Arc::from(stringify!($root))));
                    let mut rolling_index = 0;
                    widget_tree!(@insert root_id, tree, rolling_index, $($($rest)*)*);
                    let _ = rolling_index; // Silences warnings
                    tree
                }
            };
        };
        (
            @insert $parent:expr, $tree:expr, $index:ident,
            $($child:ident $(in $old:ident)* $({$($children:tt)*})*),*
        ) => {$({
            $tree.insert(
                $parent,
                $child,
                $index,
                WidgetIdent::Str(Arc::from(stringify!($child)))
            ).unwrap();
            $index += 1;


            $(
                let mut rolling_index = 0;
                widget_tree!(
                    @insert
                        $child,
                        $tree,
                        rolling_index,
                        $($children)*
                );
                let _ = rolling_index; // Silences warnings
            )*
        })*};
    }

    #[test]
    fn test_create_macro() {
        widget_tree!{
            let macro_tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1,
                child_2
            }
        };

        let mut manual_tree = WidgetTree::new(root, WidgetIdent::new_str("root"));
        manual_tree.insert(root, child_0, 0, WidgetIdent::new_str("child_0")).unwrap();
        manual_tree.insert(root, child_1, 1, WidgetIdent::new_str("child_1")).unwrap();
        manual_tree.insert(child_0, child_0_1, 0, WidgetIdent::new_str("child_0_1")).unwrap();
        manual_tree.insert(root, child_2, 2, WidgetIdent::new_str("child_2")).unwrap();
        manual_tree.insert(child_0, child_0_2, 1, WidgetIdent::new_str("child_0_2")).unwrap();
        manual_tree.insert(child_0, child_0_3, 1, WidgetIdent::new_str("child_0_3")).unwrap();
        manual_tree.insert(child_0_2, child_0_2_0, 0, WidgetIdent::new_str("child_0_2_0")).unwrap();

        assert_eq!(manual_tree, macro_tree, "{:#?}\n!=\n{:#?}", manual_tree, macro_tree);
    }

    #[test]
    fn test_macro_in_old() {
        widget_tree!{
            let macro_tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1,
                child_2
            }
        };

        widget_tree!{
            let macro_tree_old = root in old {
                child_0 in old {
                    child_0_1 in old,
                    child_0_3 in old,
                    child_0_2 in old {
                        child_0_2_0 in old
                    }
                },
                child_1 in old,
                child_2 in old
            }
        };

        assert_eq!(macro_tree, macro_tree_old);
    }

    #[test]
    fn test_remove() {
        widget_tree!{
            let mut tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };

        assert!(tree.remove(child_1).is_some());
        widget_tree!(
            let tree_removed_0 = root in old {
                child_0 in old {
                    child_0_1 in old,
                    child_0_3 in old,
                    child_0_2 in old {
                        child_0_2_0 in old
                    }
                },
                child_2 in old
            }
        );
        assert_eq!(tree, tree_removed_0);

        assert!(tree.remove(child_0).is_some());
        widget_tree!(
            let tree_removed_1 = root in old {
                child_2 in old
            }
        );
        assert_eq!(tree, tree_removed_1);

        assert!(tree.remove(child_2).is_some());
        widget_tree!(
            let tree_removed_2 = root in old
        );
        assert_eq!(tree, tree_removed_2);
    }

    #[test]
    fn test_move() {
        widget_tree!{
            let mut tree = root {
                child_0 {
                    child_0_1,
                    child_0_3,
                    child_0_2 {
                        child_0_2_0
                    }
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };

        let child_1_ident = tree.get_widget(child_1).unwrap().ident.clone();
        tree.insert(child_0_1, child_1, 0, child_1_ident).unwrap();
        widget_tree!{
            let tree_moved = root in old {
                child_0 in old {
                    child_0_1 in old {
                        child_1 in old {
                            child_1_0 in old,
                            child_1_1 in old
                        }
                    },
                    child_0_3 in old,
                    child_0_2 in old {
                        child_0_2_0 in old
                    }
                },
                child_2 in old
            }
        };
        assert_eq!(tree, tree_moved, "{:#?}\n!=\n{:#?}", tree, tree_moved);
    }

    #[test]
    fn test_relations() {
        widget_tree!{
            let tree = root {
                child_0 {
                    child_0_1,
                    child_0_2 {
                        child_0_2_0
                    },
                    child_0_3
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };
        println!("{:#?}", tree);

        assert_eq!(Err(WidgetRelationError::WidgetNotFound), tree.parent(WidgetID::new()));
        assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.parent(root));
        assert_eq!(Ok(root), tree.parent(child_0));
        assert_eq!(Ok(root), tree.parent(child_1));
        assert_eq!(Ok(root), tree.parent(child_2));
        assert_eq!(Ok(child_0), tree.parent(child_0_1));
        assert_eq!(Ok(child_0), tree.parent(child_0_2));
        assert_eq!(Ok(child_0), tree.parent(child_0_3));
        assert_eq!(Ok(child_0_2), tree.parent(child_0_2_0));
        assert_eq!(Ok(child_1), tree.parent(child_1_0));
        assert_eq!(Ok(child_1), tree.parent(child_1_1));

        for i in -16..16 {
            assert_eq!(Err(WidgetRelationError::WidgetNotFound), tree.sibling(WidgetID::new(), i), "{}", i);
            assert_eq!(None, tree.sibling_wrapping(WidgetID::new(), i), "{}", i);
            if i != 0 {
                assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.sibling(root, i), "{}", i);
                assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.sibling(child_0_2_0, i), "{}", i);
            }
            assert_eq!(Some(root), tree.sibling_wrapping(root, i), "{}", i);
            assert_eq!(Some(child_0_2_0), tree.sibling_wrapping(child_0_2_0, i), "{}", i);
        }

        assert_eq!(10, tree.all_nodes().count());
        for (id, _) in tree.all_nodes() {
            assert_eq!(Ok(id), tree.sibling(id, 0));
            assert_eq!(Some(id), tree.sibling_wrapping(id, 0));
        }

        assert_eq!(Ok(child_1), tree.sibling(child_0, 1));
        assert_eq!(Ok(child_2), tree.sibling(child_0, 2));
        assert_eq!(Ok(child_0), tree.sibling(child_1, -1));
        assert_eq!(Ok(child_2), tree.sibling(child_1, 1));
        assert_eq!(Ok(child_0), tree.sibling(child_2, -2));
        assert_eq!(Ok(child_1), tree.sibling(child_2, -1));

        for i in (-15..15).filter(|i| i % 3 == 0) {
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_0, i - 2), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_0, i - 1), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_0, i + 0), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_0, i + 1), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_0, i + 2), "{}", i);

            assert_eq!(Some(child_2), tree.sibling_wrapping(child_1, i - 2), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_1, i - 1), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_1, i + 0), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_1, i + 1), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_1, i + 2), "{}", i);

            assert_eq!(Some(child_0), tree.sibling_wrapping(child_2, i - 2), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_2, i - 1), "{}", i);
            assert_eq!(Some(child_2), tree.sibling_wrapping(child_2, i + 0), "{}", i);
            assert_eq!(Some(child_0), tree.sibling_wrapping(child_2, i + 1), "{}", i);
            assert_eq!(Some(child_1), tree.sibling_wrapping(child_2, i + 2), "{}", i);
        }

        for i in 0..16 {
            assert_eq!(Err(WidgetRelationError::WidgetNotFound), tree.child_from_start(WidgetID::new(), i));
        }
        assert_eq!(Ok(child_0), tree.child_from_start(root, 0));
        assert_eq!(Ok(child_1), tree.child_from_start(root, 1));
        assert_eq!(Ok(child_2), tree.child_from_start(root, 2));
        assert_eq!(Ok(child_0_1), tree.child_from_start(child_0, 0));
        assert_eq!(Ok(child_0_2), tree.child_from_start(child_0, 1));
        assert_eq!(Ok(child_0_3), tree.child_from_start(child_0, 2));
        assert_eq!(Ok(child_0_2_0), tree.child_from_start(child_0_2, 0));
        assert_eq!(Ok(child_1_0), tree.child_from_start(child_1, 0));
        assert_eq!(Ok(child_1_1), tree.child_from_start(child_1, 1));
        assert_eq!(Err(WidgetRelationError::RelationNotFound), tree.child_from_start(root, 3));
    }

    #[test]
    fn test_ident_chain() {
        widget_tree!{
            let tree = root {
                child_0 {
                    child_0_1,
                    child_0_2 {
                        child_0_2_0
                    },
                    child_0_3
                },
                child_1 {
                    child_1_0,
                    child_1_1
                },
                child_2
            }
        };

        macro_rules! ident_chain {
            ($($ident:ident),*) => {{
                vec![$(&WidgetIdent::new_str(stringify!($ident))),*]
            }}
        }

        assert!(tree.ident_chain_reversed(WidgetID::new()).is_none());
        assert_eq!(ident_chain![root], tree.ident_chain_reversed(root).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_0, root], tree.ident_chain_reversed(child_0).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_1, root], tree.ident_chain_reversed(child_1).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_2, root], tree.ident_chain_reversed(child_2).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_0_1, child_0, root], tree.ident_chain_reversed(child_0_1).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_0_2, child_0, root], tree.ident_chain_reversed(child_0_2).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_0_3, child_0, root], tree.ident_chain_reversed(child_0_3).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_0_2_0, child_0_2, child_0, root], tree.ident_chain_reversed(child_0_2_0).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_1_0, child_1, root], tree.ident_chain_reversed(child_1_0).unwrap().collect::<Vec<_>>());
        assert_eq!(ident_chain![child_1_1, child_1, root], tree.ident_chain_reversed(child_1_1).unwrap().collect::<Vec<_>>());
    }
}
