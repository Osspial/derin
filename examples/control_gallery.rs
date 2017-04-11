#![feature(specialization)]

extern crate derin;
extern crate dct;

use derin::ui::*;
use derin::ui::widgets::*;
use derin::ui::widgets::status::Orientation;
use derin::ui::widgets::status::{progbar, slider};
use derin::native::{Window, WindowConfig};

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

struct BasicParent {
    label: TextLabel<&'static str>,
    bar: ProgressBar,
    slider: Slider<BasicSlider>,
    button0: TextButton<AddButton>,
    button_vec: Vec<TextButton<AddButton>>
}

impl BasicParent {
    fn new() -> BasicParent {
        BasicParent {
            label: TextLabel::new("A Label"),
            bar: ProgressBar::new(progbar::Status::new(progbar::Completion::Frac(0.5), Orientation::Horizontal)),
            slider: Slider::new(BasicSlider(slider::Status::default())),
            button0: TextButton::new(AddButton("Add Button")),
            button_vec: Vec::new()
        }
    }
}

impl<NPI> Parent<NPI> for BasicParent
        where NPI: NodeProcessorInit,
              NPI::GridProcessor: NodeProcessorGridMut<TextButton<AddButton>> +
                                  NodeProcessorGridMut<ProgressBar> +
                                  NodeProcessorGridMut<Slider<BasicSlider>> +
                                  NodeProcessorGridMut<TextLabel<&'static str>>
{
    type ChildAction = GalleryEvent;

    default fn children(&self, _: NPI) -> Result<(), NPI::Error> {
        panic!("Attempted children call when NPI doesn't implement NodeProcessorGrid")
    }

    fn children_mut(&mut self, npi: NPI) -> Result<(), NPI::Error> {
        use derin::ui::hints::{GridSize, NodeSpan, WidgetHints, TrackHints};
        use std::iter;

        let mut np = npi.init_grid(
            GridSize::new(1, 4 + self.button_vec.len() as u32),
            iter::empty(),
            iter::once(TrackHints {
                fr_size: 0.0,
                ..TrackHints::default()
            })
        );

        let label_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 0..1),
            ..WidgetHints::default()
        };
        np.add_child_mut(ChildId::Str("label"), label_hints, &mut self.label)?;

        let bar_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 1..2),
            ..WidgetHints::default()
        };
        np.add_child_mut(ChildId::Str("bar"), bar_hints, &mut self.bar)?;

        let slider_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 2..3),
            ..WidgetHints::default()
        };
        np.add_child_mut(ChildId::Str("slider"), slider_hints, &mut self.slider)?;

        let button0_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 3..4),
            ..WidgetHints::default()
        };
        np.add_child_mut(ChildId::Str("button0"), button0_hints, &mut self.button0)?;

        for (i, button) in self.button_vec.iter_mut().enumerate() {
            let button_hints = WidgetHints {
                node_span: NodeSpan::new(0..1, 4+i as u32..4+i as u32+1),
                ..WidgetHints::default()
            };
            np.add_child_mut(ChildId::Num(i as u32), button_hints, button)?;
        }
        Ok(())
    }
}

impl<NPI> Parent<NPI> for BasicParent
        where NPI: NodeProcessorInit,
              NPI::GridProcessor: NodeProcessorGrid<TextButton<AddButton>> +
                                  NodeProcessorGrid<ProgressBar> +
                                  NodeProcessorGrid<Slider<BasicSlider>> +
                                  NodeProcessorGrid<TextLabel<&'static str>>
{
    fn children(&self, npi: NPI) -> Result<(), NPI::Error> {
        use derin::ui::hints::{GridSize, NodeSpan, WidgetHints, TrackHints};
        use std::iter;

        let mut np = npi.init_grid(
            GridSize::new(1, 4 + self.button_vec.len() as u32),
            iter::empty(),
            iter::once(TrackHints {
                fr_size: 0.0,
                ..TrackHints::default()
            })
        );

        let label_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 0..1),
            ..WidgetHints::default()
        };
        np.add_child(ChildId::Str("label"), label_hints, &self.label)?;

        let bar_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 1..2),
            ..WidgetHints::default()
        };
        np.add_child(ChildId::Str("bar"),  bar_hints, &self.bar)?;

        let slider_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 2..3),
            ..WidgetHints::default()
        };
        np.add_child(ChildId::Str("slider"), slider_hints, &self.slider)?;

        let button0_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 3..4),
            ..WidgetHints::default()
        };
        np.add_child(ChildId::Str("button0"), button0_hints, &self.button0)?;

        for (i, button) in self.button_vec.iter().enumerate() {
            let button_hints = WidgetHints {
                node_span: NodeSpan::new(0..1, 4+i as u32..4+i as u32+1),
                ..WidgetHints::default()
            };
            np.add_child(ChildId::Num(i as u32), button_hints, button)?;
        }
        Ok(())
    }
}

fn main() {
    let mut window = Window::new(WidgetGroup::new(BasicParent::new()), &WindowConfig::new());

    loop {
        let mut action = None;
        window.wait_actions(|new_act| {action = Some(new_act); false}).unwrap();
        match action.unwrap() {
            GalleryEvent::AddButton => window.root.button_vec.push(TextButton::new(AddButton("Another Button"))),
            GalleryEvent::SliderMoved(moved_to) => window.root.bar.completion = progbar::Completion::Frac(moved_to as f32 / 128.0)
        }
    }
}
