#![feature(never_type)]

extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate dct;

use derin::ui::*;
use derin::ui::widgets::*;
use derin::ui::widgets::status::Orientation;
use derin::ui::widgets::status::{progbar, slider};
use derin::ui::hints::*;

use derin::native::{Window, WindowConfig};

use std::iter;
use std::borrow::Borrow;

enum GalleryEvent {
    AddButton,
    SliderMoved(u32)
}

struct AddButton(&'static str);

impl Borrow<str> for AddButton {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl ButtonControl for AddButton {
    type Action = GalleryEvent;

    fn on_mouse_event(&self, _: MouseEvent) -> Option<GalleryEvent> {
        Some(GalleryEvent::AddButton)
    }
}

struct BasicSlider(slider::Status);

impl SliderControl for BasicSlider {
    type Action = GalleryEvent;

    fn status(&self) -> slider::Status {
        self.0.clone()
    }
    fn status_mut(&mut self) -> &mut slider::Status {
        &mut self.0
    }
    fn on_range_event(&self, event: RangeEvent) -> Option<GalleryEvent> {
        if let RangeEvent::Move(moved_to) = event {
            Some(GalleryEvent::SliderMoved(moved_to))
        } else {
            None
        }
    }
}

#[derive(Parent)]
#[derin(child_action = "GalleryEvent")]
struct BasicParent {
    label: TextLabel<&'static str>,
    bar: ProgressBar,
    slider: Slider<BasicSlider>,
    button0: TextButton<AddButton>,
    #[derin(layout)]
    layout: BasicParentLayout
    // button_vec: Vec<TextButton<AddButton>>
}

impl BasicParent {
    fn new() -> BasicParent {
        BasicParent {
            label: TextLabel::new("A Label"),
            bar: ProgressBar::new(progbar::Status::new(progbar::Completion::Frac(0.5), Orientation::Horizontal)),
            slider: Slider::new(BasicSlider(slider::Status::default())),
            button0: TextButton::new(AddButton("Add Button")),
            layout: BasicParentLayout
            // button_vec: Vec::new()
        }
    }
}

struct BasicParentLayout;

impl<'a> GridLayout<'a> for BasicParentLayout {
    type ColHints = iter::Repeat<TrackHints>;
    type RowHints = iter::Repeat<TrackHints>;

    fn grid_size(&self) -> GridSize {
        GridSize::new(1, 4)
    }

    fn col_hints(&'a self) -> Self::ColHints {
        iter::repeat(TrackHints::default())
    }

    fn row_hints(&'a self) -> Self::RowHints {
        iter::repeat(TrackHints {
            fr_size: 1.0,
            ..TrackHints::default()
        })
    }

    fn get_hints(&self, id: ChildId) -> Option<WidgetHints> {
        match id {
            ChildId::Str("label") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 0),
                ..WidgetHints::default()
            }),
            ChildId::Str("bar") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 1),
                ..WidgetHints::default()
            }),
            ChildId::Str("slider") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 2),
                ..WidgetHints::default()
            }),
            ChildId::Str("button0") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 3),
                ..WidgetHints::default()
            }),
            _ => None
        }
    }
}

fn main() {
    let mut window = Window::new(WidgetGroup::new(BasicParent::new()), &WindowConfig::new());

    loop {
        let mut action = None;
        window.wait_actions(|new_act| {action = Some(new_act); false}).unwrap();
        match action.unwrap() {
            GalleryEvent::AddButton => (),// window.root.button_vec.push(TextButton::new(AddButton("Another Button"))),
            GalleryEvent::SliderMoved(moved_to) => window.root.bar.completion = progbar::Completion::Frac(moved_to as f32 / 128.0)
        }
    }
}
