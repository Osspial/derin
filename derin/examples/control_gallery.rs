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

extern crate derin;
#[macro_use]
extern crate derin_macros;

use derin::{LoopFlow, Window, WindowConfig};
use derin::layout::{Margins, LayoutHorizontal, LayoutVertical};
use derin::widgets::*;
use derin::geometry::{D2, rect::DimsBox};

#[derive(Debug, Clone, Copy, PartialEq)]
enum GalleryEvent {
    NewButton,
    Checked,
    SliderMove(f32)
}

#[derive(WidgetContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<Option<GalleryEvent>>,
    nested: ScrollBox<Group<NestedContainer, LayoutVertical>>,
    tabs: TabList<Button<Option<GalleryEvent>>>
}

#[derive(WidgetContainer)]
#[derin(action = "GalleryEvent")]
struct NestedContainer {
    label: Label,
    edit_box: LineBox,
    progress_bar: ProgressBar,
    slider: Slider<SliderH>,
    check_box: CheckBox<Option<GalleryEvent>>,
    radio_buttons: RadioButtonList<Vec<RadioButton>, LayoutVertical>,
    #[derin(collection = "Button<Option<GalleryEvent>>")]
    buttons: Vec<Button<Option<GalleryEvent>>>
}

struct SliderH;
impl SliderHandler for SliderH {
    type Action = GalleryEvent;
    fn on_move(&mut self, _: f32, new_value: f32) -> Option<GalleryEvent> {
        Some(GalleryEvent::SliderMove(new_value))
    }
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new(Contents::Text("New Button".to_string()), Some(GalleryEvent::NewButton)),
            nested: ScrollBox::new(Group::new(
                NestedContainer {
                    label: Label::new(Contents::Text("Nested Container".to_string())),
                    slider: Slider::new(0.0, 1.0, 0.0, 100.0, SliderH),
                    progress_bar: ProgressBar::new(0.0, 0.0, 100.0),
                    check_box: CheckBox::new(true, Contents::Text("Checkable".to_string()), Some(GalleryEvent::Checked)),
                    radio_buttons: RadioButtonList::new(
                        vec![
                            RadioButton::new(true, Contents::Text("Radio 1".to_string())),
                            RadioButton::new(false, Contents::Text("Radio 2".to_string()))
                        ],
                        LayoutVertical::new(Margins::new(0, 2, 0, 8), Default::default())
                    ),
                    edit_box: LineBox::new("Edit Me!".to_string()),
                    buttons: Vec::new(),
                },
                LayoutVertical::new(Margins::new(8, 8, 8, 8), Default::default())
            )),
            tabs: TabList::new(vec![
                TabPage::new("Tab 1".to_string(), Button::new(Contents::Text("Tab 1".to_string()), None)),
                TabPage::new("Tab No.2".to_string(), Button::new(Contents::Text("Tab 2".to_string()), None)),
            ])
        },
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_config = WindowConfig {
        dimensions: Some(DimsBox::new2(512, 512)),
        title: "Derin Control Gallery".to_string(),
        ..WindowConfig::default()
    };

    let mut window = unsafe{ Window::new(window_config, group, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |event, root, _| {
            match event {
                GalleryEvent::NewButton =>
                    root.container_mut().nested.widget_mut().container_mut().buttons.push(Button::new(Contents::Text("An added button".to_string()), None)),
                GalleryEvent::SliderMove(move_to) =>
                    *root.container_mut().nested.widget_mut().container_mut().progress_bar.value_mut() = move_to,
                _ => ()
            }
            println!("{:?}", event);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
