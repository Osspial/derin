use std::mem;
use render::RenderFrame;
use tree::{Widget, WidgetIdent, WidgetSummary, RootID, Update};

use cgmath::{Bounded, EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{BoundBox, GeoBox};

use offset_widget::{OffsetWidget, OffsetWidgetTrait};

// TODO: GET CODE REVIEWED FOR SAFETY

struct StackElement<'a, A, F: RenderFrame> {
    widget: *mut (Widget<A, F> + 'a),
    rectangles: Option<ElementRects>,
    index: usize
}

#[derive(Clone, Copy)]
struct ElementRects {
    bounds: BoundBox<Point2<i32>>,
    bounds_clipped: Option<BoundBox<Point2<i32>>>
}

pub(crate) struct NRAllocCache<A, F: RenderFrame> {
    vec: Vec<StackElement<'static, A, F>>,
    ident_vec: Vec<WidgetIdent>
}

pub struct NRVec<'a, A: 'a, F: 'a + RenderFrame> {
    cache: &'a mut Vec<StackElement<'static, A, F>>,
    vec: Vec<StackElement<'a, A, F>>,
    ident_vec: &'a mut Vec<WidgetIdent>,
    clip_rect: BoundBox<Point2<i32>>,
    top_parent_offset: Vector2<i32>,
    root_id: RootID
}

#[derive(Debug)]
pub struct WidgetPath<'a, N: 'a + ?Sized> {
    pub widget: OffsetWidget<'a, N>, // WHAT YOU'RE DOING: REPLACING WIDGET WITH OFFSETWIDGET
    pub path: &'a [WidgetIdent]
}

impl<A, F: RenderFrame> NRAllocCache<A, F> {
    pub fn new() -> NRAllocCache<A, F> {
        NRAllocCache {
            vec: Vec::new(),
            ident_vec: Vec::new()
        }
    }

    pub fn use_cache<'a>(&'a mut self, widget: &mut (Widget<A, F> + 'a), root_id: RootID) -> NRVec<'a, A, F> {
        let mut cache_swap = Vec::new();
        mem::swap(&mut cache_swap, &mut self.vec);

        let mut vec = unsafe {
            let (ptr, len, cap) = (cache_swap.as_ptr(), cache_swap.len(), cache_swap.capacity());
            mem::forget(cache_swap);
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<A, F>>(ptr), len, cap)
        };
        let ident_vec = &mut self.ident_vec;

        vec.push(StackElement {
            widget: widget,
            rectangles: None,
            index: 0
        });
        ident_vec.push(WidgetIdent::Num(0));

        NRVec {
            cache: &mut self.vec,
            vec, ident_vec,
            clip_rect: BoundBox::new(Point2::new(0, 0), Point2::max_value()),
            top_parent_offset: Vector2::new(0, 0),
            root_id
        }
    }
}

impl<'a, A, F: RenderFrame> NRVec<'a, A, F> {
    #[inline]
    pub fn top(&mut self) -> WidgetPath<Widget<A, F>> {
        let widget = self.vec.last_mut().map(|n| unsafe{ &mut *n.widget }).unwrap();
        WidgetPath {
            widget: OffsetWidget::new(widget, self.top_parent_offset()),
            path: &self.ident_vec
        }
    }

    #[inline]
    pub fn top_ident(&self) -> WidgetIdent {
        *self.ident_vec.last().unwrap()
    }

    #[inline]
    pub fn top_index(&self) -> usize {
        self.vec.last().unwrap().index
    }

    #[inline]
    pub fn clip_rect(&self) -> Option<BoundBox<Point2<i32>>> {
        self.vec.get(self.vec.len().wrapping_sub(2))
            .and_then(|widget| widget.rectangles)
            .and_then(|rectangles| rectangles.bounds_clipped)
            .map(|r| r + self.top_parent_offset)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        assert_ne!(0, len);
        for widget_slice in self.vec[len-1..].windows(2).rev() {
            let parent = unsafe{ &*widget_slice[0].widget };
            let child = unsafe{ &*widget_slice[1].widget };

            if child.update_tag().needs_update(self.root_id) != Update::default() {
                parent.update_tag().mark_update_child_immutable();
            }
        }

        self.vec.truncate(len);
        self.ident_vec.truncate(len);

        self.top_parent_offset = Vector2::new(0, 0);
        self.clip_rect = BoundBox::new(Point2::new(0, 0), Point2::max_value());
        for bounds in self.vec[..len-1].iter().filter_map(|n| n.rectangles).map(|r| r.bounds) {
            self.clip_rect = self.clip_rect.intersect_rect(bounds + self.top_parent_offset)
                .unwrap_or(BoundBox::new2(0, 0, 0, 0));
            self.top_parent_offset += bounds.min().to_vec();
        }
    }

    #[inline]
    pub fn top_parent_offset(&self) -> Vector2<i32> {
        self.top_parent_offset
    }

    #[inline]
    pub fn widgets<'b>(&'b self) -> impl 'b + Iterator<Item=&'a Widget<A, F>> + DoubleEndedIterator + ExactSizeIterator {
        self.vec.iter().map(|n| unsafe{ &*n.widget })
    }

    #[inline]
    pub fn ident(&self) -> &[WidgetIdent] {
        debug_assert_eq!(self.ident_vec.len(), self.vec.len());
        &self.ident_vec
    }

    #[inline]
    pub fn try_push<G>(&mut self, with_top: G) -> Option<WidgetSummary<&'a mut Widget<A, F>>>
        where G: FnOnce(&'a mut Widget<A, F>, &[WidgetIdent]) -> Option<WidgetSummary<&'a mut Widget<A, F>>>
    {
        let new_top_opt = with_top(unsafe{ mem::transmute(self.top().widget.inner_mut()) }, &self.ident_vec );
        if let Some(new_top_summary) = new_top_opt {
            assert_ne!(new_top_summary.widget as *mut Widget<A, F>, self.top().widget.inner_mut() as *mut Widget<A, F>);
            {
                let current_clip_rect = self.clip_rect();
                let cur_top = self.vec.last_mut().unwrap();

                let top_rect = unsafe{ &*cur_top.widget }.rect();
                cur_top.rectangles = Some(ElementRects {
                    bounds: top_rect,
                    bounds_clipped: current_clip_rect.and_then(|r| r.intersect_rect(top_rect))
                });
                // if let Some(parent_rect) = parent_bounds_opt {
                //     cur_top.bounds = parent_rect.intersect_rect(cur_top.bounds)
                //         .unwrap_or(BoundBox::new(cur_top.bounds.min, cur_top.bounds.min));
                // }

                self.top_parent_offset += top_rect.min().to_vec();
            }

            self.vec.push(StackElement {
                widget: new_top_summary.widget,
                rectangles: None,
                index: new_top_summary.index
            });
            self.ident_vec.push(new_top_summary.ident);
            Some(new_top_summary)
        } else {
            None
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&'a mut Widget<A, F>> {
        // Ensure the base is never popped
        if self.vec.len() == 1 {
            return None;
        }

        let popped = self.vec.pop().map(|n| unsafe{ &mut *n.widget }).unwrap();
        self.ident_vec.pop();
        let last_mut = self.vec.last_mut().unwrap();
        self.top_parent_offset -= last_mut.rectangles.expect("Bad widget stack bounds").bounds.min().to_vec();
        last_mut.rectangles = None;

        if popped.update_tag().needs_update(self.root_id) != Update::default() {
            self.top().widget.update_tag().mark_update_child_immutable();
        }


        Some(popped)
    }
}

impl<'a, A, F: RenderFrame> Drop for NRVec<'a, A, F> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
        self.vec.clear();
        self.ident_vec.clear();

        let mut vec = unsafe {
            let (ptr, len, cap) = (self.vec.as_ptr(), self.vec.len(), self.vec.capacity());
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<'static, A, F>>(ptr), len, cap)
        };
        let mut empty_vec = unsafe {
            let (ptr, len, cap) = (self.cache.as_ptr(), self.cache.len(), self.cache.capacity());
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<'a, A, F>>(ptr), len, cap)
        };

        mem::swap(self.cache, &mut vec);
        mem::swap(&mut self.vec, &mut empty_vec);

        mem::forget(vec);
        mem::forget(empty_vec);
    }
}

