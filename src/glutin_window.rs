use glutin::*;
use glutin::{MouseButton as GMouseButton, WindowEvent as GWindowEvent};
use gl_render::{GLRenderer, GLFrame};
use dct::buttons::{MouseButton, Key, ModifierKeys};
use core::{Root, LoopFlow, WindowEvent, EventLoopResult};
use core::tree::{Node, NodeIdent, PopupID};
use core::event::NodeEvent;
use theme::Theme;
use gullery::ContextState;

use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::cell::{Cell, RefCell};
use std::time::Duration;
use std::collections::HashMap;
use std::rc::Rc;
use cgmath::Point2;
use cgmath_geometry::{DimsBox, GeoBox};

use parking_lot::Mutex;

pub struct GlutinWindow<A: 'static, N: 'static + Node<A, GLFrame>> {
    primary_renderer: RefCell<GLRenderer>,
    window_popup_map: HashMap<WindowId, PopupID>,
    popup_renderers: RefCell<HashMap<PopupID, GLRenderer>>,
    active_renderer: Cell<Option<PopupID>>,
    events_loop: EventsLoop,
    timer_sync: Arc<Mutex<TimerPark>>,
    timer_thread_handle: JoinHandle<()>,
    root: Root<A, N, GLFrame>
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum TimerPark {
    Indefinite,
    Timeout(Duration),
    Abort
}

impl<A, N: Node<A, GLFrame>> GlutinWindow<A, N> {
    pub unsafe fn new(window_builder: WindowBuilder, root: N, theme: Theme) -> Result<GlutinWindow<A, N>, CreationError> {
        let events_loop = EventsLoop::new();
        let renderer = GLRenderer::new(&events_loop, window_builder)?;

        let timer_sync = Arc::new(Mutex::new(TimerPark::Indefinite));
        let timer_sync_timer_thread = timer_sync.clone();
        let events_loop_proxy = events_loop.create_proxy();

        let timer_thread_handle = thread::spawn(move || {
            let timer_sync = timer_sync_timer_thread;
            loop {
                let park_type = *timer_sync.lock();

                match park_type {
                    TimerPark::Timeout(park_duration) => {
                        thread::park_timeout(park_duration);
                        if *timer_sync.lock() == TimerPark::Timeout(park_duration) {
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
            primary_renderer: RefCell::new(renderer),
            active_renderer: Cell::new(None),
            window_popup_map: HashMap::new(),
            popup_renderers: RefCell::new(HashMap::new()),
            events_loop,
            timer_sync,
            timer_thread_handle,
        })
    }

    pub fn context_state(&self) -> Rc<ContextState> {
        self.primary_renderer.borrow().context_state()
    }

    pub fn root(&self) -> &N {
        &self.root.root_node
    }

    pub fn root_mut(&mut self) -> &mut N {
        &mut self.root.root_node
    }

    pub fn run_forever<F, FF, R>(&mut self, on_action: F, on_fallthrough: FF) -> Option<R>
        where F: FnMut(A, &mut N, &mut Theme) -> LoopFlow<R>,
              FF: FnMut(NodeEvent, &[NodeIdent]) -> Option<A>
    {
        let GlutinWindow {
            ref mut primary_renderer,
            ref popup_renderers,
            ref active_renderer,
            ref mut window_popup_map,
            ref mut events_loop,
            ref mut timer_sync,
            ref mut timer_thread_handle,
            ref mut root,
        } = *self;

        let adapt_window_events = |for_each_event: &mut FnMut(Option<PopupID>, WindowEvent, Option<ModifierKeys>) -> EventLoopResult<R>| {
            let mut ret: Option<R> = None;
            loop {
                let mut add_popups = Vec::new();
                events_loop.run_forever(|glutin_event| {
                    let mut g_modifiers = None;
                    let mut popup_id = None;
                    let derin_event: WindowEvent = match glutin_event {
                        Event::WindowEvent{window_id, event} => {
                            popup_id = window_popup_map.get(&window_id).cloned();
                            match event {
                                GWindowEvent::CursorMoved{position, modifiers, ..} => {
                                    g_modifiers = Some(modifiers);
                                    WindowEvent::MouseMove(Point2::new(position.0 as i32, position.1 as i32))
                                },
                                GWindowEvent::CursorEntered{..} => WindowEvent::MouseEnter(Point2::new(0, 0)),
                                GWindowEvent::CursorLeft{..} => WindowEvent::MouseExit(Point2::new(0, 0)),
                                GWindowEvent::MouseInput{state, button: g_button, modifiers, ..} => {
                                    g_modifiers = Some(modifiers);
                                    let button = match g_button {
                                        GMouseButton::Left => MouseButton::Left,
                                        GMouseButton::Right => MouseButton::Right,
                                        GMouseButton::Middle => MouseButton::Middle,
                                        GMouseButton::Other(1) => MouseButton::X1,
                                        GMouseButton::Other(2) => MouseButton::X2,
                                        GMouseButton::Other(_) => return ControlFlow::Continue
                                    };
                                    match state {
                                        ElementState::Pressed => WindowEvent::MouseDown(button),
                                        ElementState::Released => WindowEvent::MouseUp(button)
                                    }
                                }
                                GWindowEvent::Resized(width, height) => WindowEvent::WindowResize(DimsBox::new2(width, height)),
                                GWindowEvent::ReceivedCharacter(c) => WindowEvent::Char(c),
                                GWindowEvent::KeyboardInput{ input, .. } => {
                                    if let Some(key) = input.virtual_keycode.and_then(map_key) {
                                        g_modifiers = Some(input.modifiers);
                                        match input.state {
                                            ElementState::Pressed => WindowEvent::KeyDown(key),
                                            ElementState::Released => WindowEvent::KeyUp(key)
                                        }
                                    } else {
                                        return ControlFlow::Continue
                                    }
                                }
                                GWindowEvent::Closed => return match popup_id {
                                    Some(_) => ControlFlow::Continue,
                                    None => ControlFlow::Break
                                },
                                _ => return ControlFlow::Continue
                            }
                        },
                        Event::Awakened => WindowEvent::Timer,
                        Event::Suspended(..) |
                        Event::DeviceEvent{..} => return ControlFlow::Continue
                    };

                    let modifiers_opt = g_modifiers.map(|g_modifiers| {
                        let mut modifiers = ModifierKeys::empty();
                        modifiers.set(ModifierKeys::SHIFT, g_modifiers.shift);
                        modifiers.set(ModifierKeys::CTRL, g_modifiers.ctrl);
                        modifiers.set(ModifierKeys::ALT, g_modifiers.alt);
                        modifiers.set(ModifierKeys::LOGO, g_modifiers.logo);
                        modifiers
                    });
                    let event_result = for_each_event(popup_id, derin_event, modifiers_opt);

                    match event_result.wait_until_call_timer {
                        None => *timer_sync.lock() = TimerPark::Indefinite,
                        Some(park_duration) => *timer_sync.lock() = TimerPark::Timeout(park_duration)
                    }
                    timer_thread_handle.thread().unpark();

                    match event_result.flow {
                        LoopFlow::Break(b) => {
                            ret = Some(b);
                            return ControlFlow::Break;
                        },
                        LoopFlow::Continue => ()
                    }
                    if event_result.popups.len() > 0 {
                        add_popups = event_result.popups;
                        return ControlFlow::Break;
                    }

                    ControlFlow::Continue
                });
                if add_popups.len() == 0 {
                    break;
                }

                let mut popup_renderers = popup_renderers.borrow_mut();
                for popup_attrs in add_popups.drain(..) {
                    let builder = WindowBuilder::new()
                        .with_dimensions(popup_attrs.rect.width() as u32, popup_attrs.rect.height() as u32)
                        .with_visibility(false)
                        .with_focusability(popup_attrs.focusable)
                        .with_title(popup_attrs.title)
                        .is_popup(popup_attrs.tool_window)
                        .with_decorations(popup_attrs.decorations);
                    let popup_renderer = unsafe{ GLRenderer::new(events_loop, builder).unwrap() };
                    let window_pos = primary_renderer.borrow().window().get_inner_position().unwrap();
                    popup_renderer.window().set_position(popup_attrs.rect.min().x + window_pos.0, popup_attrs.rect.min().y + window_pos.1);
                    popup_renderer.window().show();

                    window_popup_map.insert(popup_renderer.window().id(), popup_attrs.id);
                    popup_renderers.insert(popup_attrs.id, popup_renderer);
                    active_renderer.set(Some(popup_attrs.id));
                }
            }
            ret
        };

        root.run_forever(
            adapt_window_events,
            on_action,
            on_fallthrough,
            |popup_id_opt, with_renderer| {
                let (mut hashmap_cell, mut primary_cell);

                let renderer_ref = match popup_id_opt {
                    Some(popup_id) => {
                        hashmap_cell = popup_renderers.borrow_mut();
                        hashmap_cell.get_mut(&popup_id).unwrap()
                    },
                    _ => {
                        primary_cell = primary_renderer.borrow_mut();
                        &mut *primary_cell
                    }
                };
                if popup_id_opt != active_renderer.get() {
                    unsafe{ renderer_ref.window().make_current().unwrap() };
                    active_renderer.set(popup_id_opt);
                }
                with_renderer(renderer_ref)
            }
        )
    }
}

impl<A, N: Node<A, GLFrame>> Drop for GlutinWindow<A, N> {
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
        VirtualKeyCode::Key0 => Some(Key::Key0),
        VirtualKeyCode::Key1 => Some(Key::Key1),
        VirtualKeyCode::Key2 => Some(Key::Key2),
        VirtualKeyCode::Key3 => Some(Key::Key3),
        VirtualKeyCode::Key4 => Some(Key::Key4),
        VirtualKeyCode::Key5 => Some(Key::Key5),
        VirtualKeyCode::Key6 => Some(Key::Key6),
        VirtualKeyCode::Key7 => Some(Key::Key7),
        VirtualKeyCode::Key8 => Some(Key::Key8),
        VirtualKeyCode::Key9 => Some(Key::Key9),
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
