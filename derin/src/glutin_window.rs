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

use glutin::*;
use glutin::{MouseButton as GMouseButton, WindowEvent as GWindowEvent, MouseScrollDelta};
use crate::gl_render::{GLRenderer, GLFrame};
use derin_common_types::buttons::{MouseButton, Key, ModifierKeys};
use crate::core::{
    Root, LoopFlow, EventLoopResult, WindowEvent,
    tree::{Widget, WidgetIdent},
    event::WidgetEvent,
    render::Renderer,
};
use crate::theme::Theme;
use gullery::ContextState;

use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;
use std::rc::Rc;
use crate::cgmath::{Point2, Vector2};
use cgmath_geometry::{D2, rect::{DimsBox, GeoBox}};

use parking_lot::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    pub dimensions: Option<DimsBox<D2, u32>>,
    pub title: String,

    pub multisampling: u16,
    pub depth_bits: Option<u8>,
    pub stencil_bits: Option<u8>,
}

impl Default for WindowConfig {
    fn default() -> WindowConfig {
        WindowConfig {
            dimensions: None,
            title: "Derin Window".to_string(),
            multisampling: 0,
            depth_bits: None,
            stencil_bits: None
        }
    }
}

/// A window displayed on the desktop, which contains a set of drawable widgets.
pub struct GlutinWindow<N: 'static + Widget<GLFrame>> {
    primary_renderer: GLRenderer,
    events_loop: EventsLoop,
    timer_sync: Arc<Mutex<TimerPark>>,
    timer_thread_handle: JoinHandle<()>,
    root: Root<N, GLFrame>
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum TimerPark {
    Indefinite,
    Timeout(Instant),
    Abort
}

impl<N: Widget<GLFrame>> GlutinWindow<N> {
    /// Creates a new window, with the given window configuration, root widget, and theme.
    ///
    /// This is unsafe, because it creates at least one OpenGL context. By calling this function,
    /// you hand all control over the thread's OpenGL context management to this window. In most
    /// cases this shouldn't be an issue, though.
    pub unsafe fn new(config: WindowConfig, root: N, theme: Theme) -> Result<GlutinWindow<N>, CreationError> {
        let mut window_builder = WindowBuilder::new();
        window_builder.window.dimensions = config.dimensions.map(|d| (d.width(), d.height()));
        window_builder.window.title = config.title.clone();
        let gen_context_builder = || {
            let mut context_builder = ContextBuilder::new();

            context_builder = context_builder.with_multisampling(config.multisampling);
            if let Some(depth_bits) = config.depth_bits {
                context_builder = context_builder.with_depth_buffer(depth_bits);
            }
            if let Some(stencil_bits) = config.stencil_bits {
                context_builder = context_builder.with_stencil_buffer(stencil_bits);
            }

            context_builder
        };

        let events_loop = EventsLoop::new();
        let renderer = GLRenderer::new(&events_loop, window_builder, gen_context_builder)?;

        let timer_sync = Arc::new(Mutex::new(TimerPark::Indefinite));
        let timer_sync_timer_thread = timer_sync.clone();
        let events_loop_proxy = events_loop.create_proxy();

        let timer_thread_handle = thread::spawn(move || {
            let timer_sync = timer_sync_timer_thread;
            loop {
                let park_type = *timer_sync.lock();

                match park_type {
                    TimerPark::Timeout(park_until) => {
                        let now = Instant::now();
                        if park_until > now {
                            thread::park_timeout(park_until - now);
                        }
                        if *timer_sync.lock() == TimerPark::Timeout(park_until) {
                            if events_loop_proxy.wakeup().is_err() {
                                return;
                            }
                        }
                    }
                    TimerPark::Indefinite => thread::park(),
                    TimerPark::Abort => return
                }
            }
        });

        Ok(GlutinWindow {
            root: Root::new(root, theme, renderer.dims()),
            primary_renderer: renderer,
            events_loop,
            timer_sync,
            timer_thread_handle,
        })
    }

    /// Retrieves a reference to the root widget.
    pub fn root(&self) -> &N {
        &self.root.root_widget
    }

    /// Retrieves a mutable reference to the root widget.
    pub fn root_mut(&mut self) -> &mut N {
        &mut self.root.root_widget
    }

    /// Starts the `derin` event loop, calling `on_action` whenever an action is triggered by a
    /// child widget. Aborts when `LoopFlow::Break` is returned by `on_action`.
    ///
    /// `on_fallthrough` is called whenever a raw event bubbles through the root widget.
    ///
    /// TODO: DOCUMENT HOW EVENT BUBBLING WORKS
    pub fn run_forever(&mut self)
    {
        let GlutinWindow {
            ref mut primary_renderer,
            ref mut events_loop,
            ref mut timer_sync,
            ref mut timer_thread_handle,
            ref mut root,
        } = *self;

        let map_modifiers = |g_modifiers: ModifiersState| {
            let mut modifiers = ModifierKeys::empty();
            modifiers.set(ModifierKeys::SHIFT, g_modifiers.shift);
            modifiers.set(ModifierKeys::CTRL, g_modifiers.ctrl);
            modifiers.set(ModifierKeys::ALT, g_modifiers.alt);
            modifiers.set(ModifierKeys::LOGO, g_modifiers.logo);
            modifiers
        };

        loop {
            let mut break_loop = false;

            let mut frame = root.start_frame();
            let mut process_glutin_event = |glutin_event| {
                let derin_event: WindowEvent = match glutin_event {
                    Event::WindowEvent{window_id, event} => {
                        let scale_factor = primary_renderer.window().hidpi_factor();
                        macro_rules! scale {
                            ($val:expr) => {{($val as f32 / scale_factor) as _}}
                        }
                        match event {
                            GWindowEvent::CursorMoved{position, modifiers, ..} => {
                                frame.set_modifiers(map_modifiers(modifiers));
                                WindowEvent::MouseMove(Point2::new(scale!(position.0), scale!(position.1)))
                            },
                            GWindowEvent::CursorEntered{..} => WindowEvent::MouseEnter,
                            GWindowEvent::CursorLeft{..} => WindowEvent::MouseExit,
                            GWindowEvent::MouseInput{state, button: g_button, modifiers, ..} => {
                                frame.set_modifiers(map_modifiers(modifiers));
                                let button = match g_button {
                                    GMouseButton::Left => MouseButton::Left,
                                    GMouseButton::Right => MouseButton::Right,
                                    GMouseButton::Middle => MouseButton::Middle,
                                    GMouseButton::Other(1) => MouseButton::X1,
                                    GMouseButton::Other(2) => MouseButton::X2,
                                    GMouseButton::Other(_) => return
                                };
                                match state {
                                    ElementState::Pressed => WindowEvent::MouseDown(button),
                                    ElementState::Released => WindowEvent::MouseUp(button)
                                }
                            }
                            GWindowEvent::MouseWheel{delta, modifiers, ..} => {
                                frame.set_modifiers(map_modifiers(modifiers));
                                match delta {
                                    MouseScrollDelta::LineDelta(x, y) => WindowEvent::MouseScrollLines(Vector2::new(x as i32, y as i32)),
                                    MouseScrollDelta::PixelDelta(x, y) => WindowEvent::MouseScrollPx(Vector2::new(x as i32, y as i32)),
                                }
                            }
                            GWindowEvent::Resized(width, height) => WindowEvent::WindowResize(DimsBox::new2(scale!(width), scale!(height))),
                            GWindowEvent::ReceivedCharacter(c) => WindowEvent::Char(c),
                            GWindowEvent::KeyboardInput{ input, .. } => {
                                if let Some(key) = input.virtual_keycode.and_then(map_key) {
                                    frame.set_modifiers(map_modifiers(input.modifiers));
                                    match input.state {
                                        ElementState::Pressed => WindowEvent::KeyDown(key),
                                        ElementState::Released => WindowEvent::KeyUp(key)
                                    }
                                } else {
                                    return;
                                }
                            }
                            GWindowEvent::Closed => {
                                break_loop = true;
                                return
                            },
                            GWindowEvent::Refresh => WindowEvent::Redraw,
                            _ => return
                        }
                    },
                    Event::Awakened => WindowEvent::Timer,
                    Event::Suspended(..) |
                    Event::DeviceEvent{..} => return
                };

                frame.process_event(derin_event);
            };

            events_loop.run_forever(|e| {process_glutin_event(e); ControlFlow::Break});
            events_loop.poll_events(process_glutin_event);

            let EventLoopResult {
                next_timer,
                set_cursor_pos,
                set_cursor_icon,
            } = frame.finish();

            match next_timer {
                None => *timer_sync.lock() = TimerPark::Indefinite,
                Some(park_until) => *timer_sync.lock() = TimerPark::Timeout(park_until)
            }
            if let Some(cursor_pos) = set_cursor_pos {
                primary_renderer.set_cursor_pos(cursor_pos);
            }
            if let Some(cursor_icon) = set_cursor_icon {
                primary_renderer.set_cursor_icon(cursor_icon);
            }
            timer_thread_handle.thread().unpark();

            if break_loop {
                break;
            }

            let size_bounds = root.relayout();
            primary_renderer.set_size_bounds(size_bounds);
            root.redraw(primary_renderer);
        }
    }

    /// Retrieves the `gullery` context state.
    pub fn context_state(&self) -> Rc<ContextState> {
        self.primary_renderer.context_state()
    }
}

impl<N: Widget<GLFrame>> Drop for GlutinWindow<N> {
    fn drop(&mut self) {
        *self.timer_sync.lock() = TimerPark::Abort;
        self.timer_thread_handle.thread().unpark();
    }
}

fn map_key(k: VirtualKeyCode) -> Option<Key> {
    match k {
        VirtualKeyCode::Back => Some(Key::Back),
        VirtualKeyCode::Tab => Some(Key::Tab),
        // VirtualKeyCode::Clear => Some(Key::Clear),
        VirtualKeyCode::Return => Some(Key::Enter),
        // VirtualKeyCode::Pause => Some(Key::Pause),
        VirtualKeyCode::Escape => Some(Key::Escape),
        VirtualKeyCode::Space => Some(Key::Space),
        VirtualKeyCode::PageUp => Some(Key::PageUp),
        VirtualKeyCode::PageDown => Some(Key::PageDown),
        VirtualKeyCode::End => Some(Key::End),
        VirtualKeyCode::Home => Some(Key::Home),
        // VirtualKeyCode::Select => Some(Key::Select),
        // VirtualKeyCode::Print => Some(Key::Print),
        // VirtualKeyCode::Execute => Some(Key::Execute),
        VirtualKeyCode::Snapshot => Some(Key::PrntScr),
        VirtualKeyCode::Insert => Some(Key::Insert),
        VirtualKeyCode::Delete => Some(Key::Delete),
        // VirtualKeyCode::Help => Some(Key::Help),
        VirtualKeyCode::Key0 => Some(Key::Alpha0),
        VirtualKeyCode::Key1 => Some(Key::Alpha1),
        VirtualKeyCode::Key2 => Some(Key::Alpha2),
        VirtualKeyCode::Key3 => Some(Key::Alpha3),
        VirtualKeyCode::Key4 => Some(Key::Alpha4),
        VirtualKeyCode::Key5 => Some(Key::Alpha5),
        VirtualKeyCode::Key6 => Some(Key::Alpha6),
        VirtualKeyCode::Key7 => Some(Key::Alpha7),
        VirtualKeyCode::Key8 => Some(Key::Alpha8),
        VirtualKeyCode::Key9 => Some(Key::Alpha9),
        VirtualKeyCode::A => Some(Key::A),
        VirtualKeyCode::B => Some(Key::B),
        VirtualKeyCode::C => Some(Key::C),
        VirtualKeyCode::D => Some(Key::D),
        VirtualKeyCode::E => Some(Key::E),
        VirtualKeyCode::F => Some(Key::F),
        VirtualKeyCode::G => Some(Key::G),
        VirtualKeyCode::H => Some(Key::H),
        VirtualKeyCode::I => Some(Key::I),
        VirtualKeyCode::J => Some(Key::J),
        VirtualKeyCode::K => Some(Key::K),
        VirtualKeyCode::L => Some(Key::L),
        VirtualKeyCode::M => Some(Key::M),
        VirtualKeyCode::N => Some(Key::N),
        VirtualKeyCode::O => Some(Key::O),
        VirtualKeyCode::P => Some(Key::P),
        VirtualKeyCode::Q => Some(Key::Q),
        VirtualKeyCode::R => Some(Key::R),
        VirtualKeyCode::S => Some(Key::S),
        VirtualKeyCode::T => Some(Key::T),
        VirtualKeyCode::U => Some(Key::U),
        VirtualKeyCode::V => Some(Key::V),
        VirtualKeyCode::W => Some(Key::W),
        VirtualKeyCode::X => Some(Key::X),
        VirtualKeyCode::Y => Some(Key::Y),
        VirtualKeyCode::Z => Some(Key::Z),
        VirtualKeyCode::Semicolon => Some(Key::Semicolon),
        VirtualKeyCode::Equals => Some(Key::Equals),
        VirtualKeyCode::Comma => Some(Key::Comma),
        VirtualKeyCode::Minus => Some(Key::Minus),
        VirtualKeyCode::Period => Some(Key::Period),
        VirtualKeyCode::Slash => Some(Key::Slash),
        VirtualKeyCode::Grave => Some(Key::Accent),
        VirtualKeyCode::LBracket => Some(Key::LBracket),
        VirtualKeyCode::RBracket => Some(Key::RBracket),
        VirtualKeyCode::Backslash => Some(Key::Backslash),
        VirtualKeyCode::Apostrophe => Some(Key::Apostrophe),
        VirtualKeyCode::Sleep => Some(Key::Sleep),
        VirtualKeyCode::Numpad0 => Some(Key::Num0),
        VirtualKeyCode::Numpad1 => Some(Key::Num1),
        VirtualKeyCode::Numpad2 => Some(Key::Num2),
        VirtualKeyCode::Numpad3 => Some(Key::Num3),
        VirtualKeyCode::Numpad4 => Some(Key::Num4),
        VirtualKeyCode::Numpad5 => Some(Key::Num5),
        VirtualKeyCode::Numpad6 => Some(Key::Num6),
        VirtualKeyCode::Numpad7 => Some(Key::Num7),
        VirtualKeyCode::Numpad8 => Some(Key::Num8),
        VirtualKeyCode::Numpad9 => Some(Key::Num9),
        VirtualKeyCode::Multiply => Some(Key::NumStar),
        VirtualKeyCode::Add => Some(Key::NumPlus),
        VirtualKeyCode::Subtract => Some(Key::NumSub),
        VirtualKeyCode::Decimal => Some(Key::NumDot),
        VirtualKeyCode::Divide => Some(Key::NumSlash),
        VirtualKeyCode::F1 => Some(Key::F1),
        VirtualKeyCode::F2 => Some(Key::F2),
        VirtualKeyCode::F3 => Some(Key::F3),
        VirtualKeyCode::F4 => Some(Key::F4),
        VirtualKeyCode::F5 => Some(Key::F5),
        VirtualKeyCode::F6 => Some(Key::F6),
        VirtualKeyCode::F7 => Some(Key::F7),
        VirtualKeyCode::F8 => Some(Key::F8),
        VirtualKeyCode::F9 => Some(Key::F9),
        VirtualKeyCode::F10 => Some(Key::F10),
        VirtualKeyCode::F11 => Some(Key::F11),
        VirtualKeyCode::F12 => Some(Key::F12),
        VirtualKeyCode::F13 => Some(Key::F13),
        VirtualKeyCode::F14 => Some(Key::F14),
        VirtualKeyCode::F15 => Some(Key::F15),
        // VirtualKeyCode::F16 => Some(Key::F16),
        // VirtualKeyCode::F17 => Some(Key::F17),
        // VirtualKeyCode::F18 => Some(Key::F18),
        // VirtualKeyCode::F19 => Some(Key::F19),
        // VirtualKeyCode::F20 => Some(Key::F20),
        // VirtualKeyCode::F21 => Some(Key::F21),
        // VirtualKeyCode::F22 => Some(Key::F22),
        // VirtualKeyCode::F23 => Some(Key::F23),
        // VirtualKeyCode::F24 => Some(Key::F24),
        VirtualKeyCode::Numlock => Some(Key::NumLock),
        // VirtualKeyCode::Caps => Some(Key::Caps),
        VirtualKeyCode::Scroll => Some(Key::ScrollLock),
        VirtualKeyCode::LShift => Some(Key::LShift),
        VirtualKeyCode::RShift => Some(Key::RShift),
        VirtualKeyCode::LControl => Some(Key::LCtrl),
        VirtualKeyCode::RControl => Some(Key::RCtrl),
        VirtualKeyCode::LAlt => Some(Key::LAlt),
        VirtualKeyCode::RAlt => Some(Key::RAlt),
        VirtualKeyCode::NavigateBackward => Some(Key::BrowserBack),
        VirtualKeyCode::NavigateForward => Some(Key::BrowserFwd),
        VirtualKeyCode::WebRefresh => Some(Key::BrowserRef),
        VirtualKeyCode::WebStop => Some(Key::BrowserStop),
        VirtualKeyCode::WebSearch => Some(Key::BrowserSearch),
        VirtualKeyCode::WebFavorites => Some(Key::BrowserFav),
        VirtualKeyCode::WebHome => Some(Key::BrowserHome),
        VirtualKeyCode::NextTrack => Some(Key::MediaNextTrack),
        VirtualKeyCode::PrevTrack => Some(Key::MediaPrevTrack),
        VirtualKeyCode::Stop => Some(Key::MediaStop),
        VirtualKeyCode::Pause => Some(Key::Pause),
        VirtualKeyCode::Left => Some(Key::LArrow),
        VirtualKeyCode::Up => Some(Key::UArrow),
        VirtualKeyCode::Right => Some(Key::RArrow),
        VirtualKeyCode::Down => Some(Key::DArrow),
        VirtualKeyCode::Kana => Some(Key::Kana),
        // VirtualKeyCode::Junja => Some(Key::Junja),
        // VirtualKeyCode::Final => Some(Key::Final),
        VirtualKeyCode::Kanji => Some(Key::Kanji),
        VirtualKeyCode::Convert => Some(Key::Convert),
        // VirtualKeyCode::Nonconvert => Some(Key::Nonconvert),
        // VirtualKeyCode::Accept => Some(Key::Accept),
        // VirtualKeyCode::ModeChange => Some(Key::ModeChange),
        // VirtualKeyCode::Process => Some(Key::Process),
        // VirtualKeyCode::LShift => Some(Key::Shift),
        // VirtualKeyCode::Control => Some(Key::Control),
        // VirtualKeyCode::Menu => Some(Key::Menu),
        VirtualKeyCode::Caret |
        VirtualKeyCode::Compose |
        VirtualKeyCode::AbntC1 |
        VirtualKeyCode::AbntC2 |
        VirtualKeyCode::Apps |
        VirtualKeyCode::At |
        VirtualKeyCode::Ax |
        VirtualKeyCode::Calculator |
        VirtualKeyCode::Capital |
        VirtualKeyCode::Colon |
        VirtualKeyCode::LMenu |
        VirtualKeyCode::LWin |
        VirtualKeyCode::Mail |
        VirtualKeyCode::MediaSelect |
        VirtualKeyCode::MediaStop |
        VirtualKeyCode::Mute |
        VirtualKeyCode::MyComputer |
        VirtualKeyCode::NoConvert |
        VirtualKeyCode::NumpadComma |
        VirtualKeyCode::NumpadEnter |
        VirtualKeyCode::NumpadEquals |
        VirtualKeyCode::OEM102 |
        VirtualKeyCode::PlayPause |
        VirtualKeyCode::Power |
        VirtualKeyCode::RMenu |
        VirtualKeyCode::RWin |
        VirtualKeyCode::Sysrq |
        VirtualKeyCode::Underline |
        VirtualKeyCode::Unlabeled |
        VirtualKeyCode::VolumeDown |
        VirtualKeyCode::VolumeUp |
        VirtualKeyCode::Wake |
        VirtualKeyCode::WebBack |
        VirtualKeyCode::WebForward |
        VirtualKeyCode::Yen => None
    }
}
