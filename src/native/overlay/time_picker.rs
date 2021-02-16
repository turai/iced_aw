//! Use a time picker as an input element for picking times.
//!
//! *This API requires the following crate features to be activated: time_picker*
use std::hash::Hash;

use crate::{
    core::clock::{
        NearestRadius, HOUR_RADIUS_PERCENTAGE, HOUR_RADIUS_PERCENTAGE_NO_SECONDS,
        MINUTE_RADIUS_PERCENTAGE, MINUTE_RADIUS_PERCENTAGE_NO_SECONDS, PERIOD_PERCENTAGE,
        SECOND_RADIUS_PERCENTAGE,
    },
    core::{renderer::DrawEnvironment, time::Period},
    graphics::icons::Icon,
    native::{
        icon_text,
        time_picker::{self, Time},
        IconText,
    },
};
use chrono::{Duration, Local, NaiveTime, Timelike};
use iced_graphics::{canvas, Size};
use iced_native::{
    button, column, container, event, keyboard,
    layout::{self, Limits},
    mouse, overlay, row, text, touch, Align, Button, Clipboard, Column, Container, Element, Event,
    Layout, Length, Point, Row, Text, Widget,
};

const PADDING: u16 = 10;
const SPACING: u16 = 15;
const BUTTON_SPACING: u16 = 5;

/// The overlay of the [`TimePicker`](crate::native::TimePicker).
#[allow(missing_debug_implementations)]
pub struct TimePickerOverlay<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + self::Renderer + button::Renderer,
{
    state: &'a mut State,
    cancel_button: Element<'a, Message, Renderer>,
    submit_button: Element<'a, Message, Renderer>,
    on_submit: &'a dyn Fn(Time) -> Message,
    position: Point,
    style: &'a <Renderer as self::Renderer>::Style,
}

impl<'a, Message, Renderer> TimePickerOverlay<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a
        + self::Renderer
        + button::Renderer
        + column::Renderer
        + container::Renderer
        + icon_text::Renderer
        + row::Renderer
        + text::Renderer,
{
    /// Creates a new [`TimePickerOverlay`](TimePickerOverlay) on the given
    /// position.
    pub fn new(
        state: &'a mut time_picker::State,
        on_cancel: Message,
        on_submit: &'a dyn Fn(Time) -> Message,
        position: Point,
        style: &'a <Renderer as self::Renderer>::Style,
    ) -> Self {
        let time_picker::State {
            overlay_state,
            cancel_button,
            submit_button,
            ..
        } = state;

        TimePickerOverlay {
            state: overlay_state,
            cancel_button: Button::new(cancel_button, IconText::new(Icon::X).width(Length::Fill))
                .width(Length::Fill)
                .on_press(on_cancel.clone())
                .into(),
            submit_button: Button::new(
                submit_button,
                IconText::new(Icon::Check).width(Length::Fill),
            )
            .width(Length::Fill)
            .on_press(on_cancel) // Sending a fake message
            .into(),
            on_submit,
            position,
            style,
        }
    }

    /// Turn this [`TimePickerOverlay`](TimePickerOverlay) into an overlay
    /// [`Element`](overlay::Element).
    pub fn overlay(self) -> overlay::Element<'a, Message, Renderer> {
        overlay::Element::new(self.position, Box::new(self))
    }

    /// The event handling for the clock.
    fn on_event_clock(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        _messages: &mut Vec<Message>,
        _renderer: &Renderer,
        _clipboard: Option<&dyn Clipboard>,
    ) -> event::Status {
        // TODO: Don't know why clock_status is never read?!
        #[allow(unused_assignments)]
        let mut clock_status = event::Status::Ignored;
        if layout.bounds().contains(cursor_position) {
            self.state.clock_cache_needs_clearance = true;
            self.state.clock_cache.clear();
        } else if self.state.clock_cache_needs_clearance {
            self.state.clock_cache.clear();
            self.state.clock_cache_needs_clearance = false;
        }

        // TODO: clean this up
        let clock_bounds = layout.bounds();
        if clock_bounds.contains(cursor_position) {
            let center = clock_bounds.center();
            let radius = clock_bounds.width.min(clock_bounds.height) * 0.5;

            let period_radius = radius * PERIOD_PERCENTAGE;

            let (hour_radius, minute_radius, second_radius) = if self.state.show_seconds {
                (
                    radius * HOUR_RADIUS_PERCENTAGE,
                    radius * MINUTE_RADIUS_PERCENTAGE,
                    radius * SECOND_RADIUS_PERCENTAGE,
                )
            } else {
                (
                    radius * HOUR_RADIUS_PERCENTAGE_NO_SECONDS,
                    radius * MINUTE_RADIUS_PERCENTAGE_NO_SECONDS,
                    f32::MAX,
                )
            };

            let nearest_radius = crate::core::clock::nearest_radius(
                &if self.state.show_seconds {
                    vec![
                        (period_radius, NearestRadius::Period),
                        (hour_radius, NearestRadius::Hour),
                        (minute_radius, NearestRadius::Minute),
                        (second_radius, NearestRadius::Second),
                    ]
                } else {
                    vec![
                        (period_radius, NearestRadius::Period),
                        (hour_radius, NearestRadius::Hour),
                        (minute_radius, NearestRadius::Minute),
                    ]
                },
                cursor_position,
                center,
            );

            let clock_clicked_status = match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }) => match nearest_radius {
                    NearestRadius::Period => {
                        let (pm, hour) = self.state.time.hour12();
                        let hour = if hour == 12 {
                            if pm {
                                12
                            } else {
                                0
                            }
                        } else {
                            hour
                        };

                        self.state.time = self
                            .state
                            .time
                            .with_hour(if pm && hour != 12 { hour } else { hour + 12 } % 24)
                            .unwrap();
                        event::Status::Captured
                    }
                    NearestRadius::Hour => {
                        self.state.focus = Focus::DigitalHour;
                        self.state.clock_dragged = ClockDragged::Hour;
                        event::Status::Captured
                    }
                    NearestRadius::Minute => {
                        self.state.focus = Focus::DigitalMinute;
                        self.state.clock_dragged = ClockDragged::Minute;
                        event::Status::Captured
                    }
                    NearestRadius::Second => {
                        self.state.focus = Focus::DigitalSecond;
                        self.state.clock_dragged = ClockDragged::Second;
                        event::Status::Captured
                    }
                    _ => event::Status::Ignored,
                },
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerLifted { .. })
                | Event::Touch(touch::Event::FingerLost { .. }) => {
                    self.state.clock_dragged = ClockDragged::None;
                    event::Status::Captured
                }
                _ => event::Status::Ignored,
            };

            let clock_dragged_status = match self.state.clock_dragged {
                ClockDragged::Hour => {
                    let hour_points = crate::core::clock::circle_points(hour_radius, center, 12);
                    let nearest_point =
                        crate::core::clock::nearest_point(&hour_points, cursor_position);

                    let (pm, _) = self.state.time.hour12();

                    self.state.time = self
                        .state
                        .time
                        .with_hour((nearest_point as u32 + if pm { 12 } else { 0 }) % 24)
                        .unwrap();
                    event::Status::Captured
                }
                ClockDragged::Minute => {
                    let minute_points =
                        crate::core::clock::circle_points(minute_radius, center, 60);
                    let nearest_point =
                        crate::core::clock::nearest_point(&minute_points, cursor_position);

                    self.state.time = self.state.time.with_minute(nearest_point as u32).unwrap();
                    event::Status::Captured
                }
                ClockDragged::Second => {
                    let second_points =
                        crate::core::clock::circle_points(second_radius, center, 60);
                    let nearest_point =
                        crate::core::clock::nearest_point(&second_points, cursor_position);

                    self.state.time = self.state.time.with_second(nearest_point as u32).unwrap();
                    event::Status::Captured
                }
                ClockDragged::None => event::Status::Ignored,
            };

            clock_status = clock_clicked_status.merge(clock_dragged_status);
        } else {
            match event {
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerLifted { .. })
                | Event::Touch(touch::Event::FingerLost { .. }) => {
                    self.state.clock_dragged = ClockDragged::None;
                    clock_status = event::Status::Captured
                }
                _ => clock_status = event::Status::Ignored,
            }
        }

        clock_status
    }

    /// The event handling for the digital clock.
    fn on_event_digital_clock(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        _messages: &mut Vec<Message>,
        _renderer: &Renderer,
        _clipboard: Option<&dyn Clipboard>,
    ) -> event::Status {
        let mut digital_clock_children = layout.children();

        if !self.state.use_24h {
            // Placeholder
            let _ = digital_clock_children.next();
        }

        let hour_layout = digital_clock_children.next().unwrap();
        let mut hour_children = hour_layout.children();

        let hour_up_arrow = hour_children.next().unwrap();
        let _ = hour_children.next();
        let hour_down_arrow = hour_children.next().unwrap();

        let _ = digital_clock_children.next();

        let minute_layout = digital_clock_children.next().unwrap();
        let mut minute_children = minute_layout.children();

        let minute_up_arrow = minute_children.next().unwrap();
        let _ = minute_children.next();
        let minute_down_arrow = minute_children.next().unwrap();

        let calculate_time = |time: &mut NaiveTime,
                              up_arrow: Layout<'_>,
                              down_arrow: Layout<'_>,
                              duration: Duration| {
            let mut status = event::Status::Ignored;

            if up_arrow.bounds().contains(cursor_position) {
                *time += duration;

                status = event::Status::Captured;
            }
            if down_arrow.bounds().contains(cursor_position) {
                *time -= duration;

                status = event::Status::Captured;
            }

            status
        };

        let digital_clock_status = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let mut status = event::Status::Ignored;

                if hour_layout.bounds().contains(cursor_position) {
                    self.state.focus = Focus::DigitalHour;

                    status = calculate_time(
                        &mut self.state.time,
                        hour_up_arrow,
                        hour_down_arrow,
                        Duration::hours(1),
                    );
                }

                if minute_layout.bounds().contains(cursor_position) {
                    self.state.focus = Focus::DigitalMinute;

                    status = calculate_time(
                        &mut self.state.time,
                        minute_up_arrow,
                        minute_down_arrow,
                        Duration::minutes(1),
                    );
                }

                status
            }
            _ => event::Status::Ignored,
        };

        let second_status = if self.state.show_seconds {
            let _ = digital_clock_children.next();

            let second_layout = digital_clock_children.next().unwrap();
            let mut second_children = second_layout.children();

            let second_up_arrow = second_children.next().unwrap();
            let _ = second_children.next();
            let second_down_arrow = second_children.next().unwrap();

            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. }) => {
                    let mut status = event::Status::Ignored;

                    if second_layout.bounds().contains(cursor_position) {
                        self.state.focus = Focus::DigitalSecond;

                        status = calculate_time(
                            &mut self.state.time,
                            second_up_arrow,
                            second_down_arrow,
                            Duration::seconds(1),
                        );
                    }

                    status
                }
                _ => event::Status::Ignored,
            }
        } else {
            event::Status::Ignored
        };

        let digital_clock_status = digital_clock_status.merge(second_status);

        if digital_clock_status == event::Status::Captured {
            self.state.clock_cache.clear()
        }

        digital_clock_status
    }

    fn on_event_keyboard(
        &mut self,
        event: Event,
        _layout: Layout<'_>,
        _cursor_position: Point,
        _messages: &mut Vec<Message>,
        _renderer: &Renderer,
        _clipboard: Option<&dyn Clipboard>,
    ) -> event::Status {
        // TODO: clean this up a bit
        if self.state.focus == Focus::None {
            return event::Status::Ignored;
        }

        if let Event::Keyboard(keyboard::Event::KeyPressed { key_code, .. }) = event {
            let mut status = event::Status::Ignored;

            match key_code {
                keyboard::KeyCode::Tab => {
                    if self.state.keyboard_modifiers.shift {
                        self.state.focus = self.state.focus.previous(self.state.show_seconds);
                    } else {
                        self.state.focus = self.state.focus.next(self.state.show_seconds);
                    }
                }
                _ => {
                    let mut keyboard_handle =
                        |key_code: keyboard::KeyCode, time: &mut NaiveTime, duration: Duration| {
                            match key_code {
                                keyboard::KeyCode::Left | keyboard::KeyCode::Down => {
                                    *time -= duration;
                                    status = event::Status::Captured;
                                }
                                keyboard::KeyCode::Right | keyboard::KeyCode::Up => {
                                    *time += duration;
                                    status = event::Status::Captured;
                                }
                                _ => {}
                            }
                        };

                    match self.state.focus {
                        Focus::Overlay => {}
                        Focus::DigitalHour => {
                            keyboard_handle(key_code, &mut self.state.time, Duration::hours(1))
                        }
                        Focus::DigitalMinute => {
                            keyboard_handle(key_code, &mut self.state.time, Duration::minutes(1))
                        }
                        Focus::DigitalSecond => {
                            keyboard_handle(key_code, &mut self.state.time, Duration::seconds(1))
                        }
                        Focus::Cancel => {}
                        Focus::Submit => {}
                        _ => {}
                    }
                }
            }

            if status == event::Status::Captured {
                self.state.clock_cache.clear()
            }

            status
        } else if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            self.state.keyboard_modifiers = modifiers;
            event::Status::Ignored
        } else {
            event::Status::Ignored
        }
    }
}

impl<'a, Message, Renderer> iced_native::Overlay<Message, Renderer>
    for TimePickerOverlay<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a
        + self::Renderer
        + button::Renderer
        + column::Renderer
        + container::Renderer
        + icon_text::Renderer
        + row::Renderer
        + text::Renderer,
{
    fn layout(
        &self,
        renderer: &Renderer,
        bounds: iced_graphics::Size,
        position: Point,
    ) -> iced_native::layout::Node {
        let limits = Limits::new(Size::ZERO, bounds)
            .pad(PADDING as f32)
            .width(Length::Fill)
            .height(Length::Fill)
            .max_width(300)
            .max_height(350);

        let arrow_size = text::Renderer::default_size(renderer);
        let font_size = (1.2 * (text::Renderer::default_size(renderer) as f32)) as u16;

        // Digital Clock
        let digital_clock_limits = limits;

        let mut digital_clock_row = Row::<(), Renderer>::new()
            .align_items(Align::Center)
            .height(Length::Shrink)
            .width(Length::Shrink)
            .spacing(1);

        if !self.state.use_24h {
            digital_clock_row = digital_clock_row.push(
                Column::new() // Just a placeholder
                    .height(Length::Shrink)
                    .push(Text::new("AM").size(font_size)),
            )
        }

        digital_clock_row = digital_clock_row
            .push(
                // Hour
                Column::new()
                    .align_items(Align::Center)
                    .height(Length::Shrink)
                    .push(
                        // Up Hour arrow
                        Row::new()
                            .width(Length::Units(arrow_size))
                            .height(Length::Units(arrow_size)),
                    )
                    .push(Text::new(format!("{:02}", self.state.time.hour())).size(font_size))
                    .push(
                        // Down Hour arrow
                        Row::new()
                            .width(Length::Units(arrow_size))
                            .height(Length::Units(arrow_size)),
                    ),
            )
            .push(
                Column::new()
                    .height(Length::Shrink)
                    .push(Text::new(":").size(font_size)),
            )
            .push(
                Column::new()
                    .align_items(Align::Center)
                    .height(Length::Shrink)
                    .push(
                        // Up Minute arrow
                        Row::new()
                            .width(Length::Units(arrow_size))
                            .height(Length::Units(arrow_size)),
                    )
                    .push(Text::new(format!("{:02}", self.state.time.hour())).size(font_size))
                    .push(
                        // Down Minute arrow
                        Row::new()
                            .width(Length::Units(arrow_size))
                            .height(Length::Units(arrow_size)),
                    ),
            );

        if self.state.show_seconds {
            digital_clock_row = digital_clock_row
                .push(
                    Column::new()
                        .height(Length::Shrink)
                        .push(Text::new(":").size(font_size)),
                )
                .push(
                    Column::new()
                        .align_items(Align::Center)
                        .height(Length::Shrink)
                        .push(
                            // Up Minute arrow
                            Row::new()
                                .width(Length::Units(arrow_size))
                                .height(Length::Units(arrow_size)),
                        )
                        .push(Text::new(format!("{:02}", self.state.time.hour())).size(font_size))
                        .push(
                            // Down Minute arrow
                            Row::new()
                                .width(Length::Units(arrow_size))
                                .height(Length::Units(arrow_size)),
                        ),
                )
        }

        if !self.state.use_24h {
            digital_clock_row = digital_clock_row.push(
                Column::new()
                    .height(Length::Shrink)
                    .push(Text::new("AM").size(font_size)),
            );
        }

        let mut digital_clock = Container::new(digital_clock_row)
            .width(Length::Fill)
            .height(Length::Shrink)
            .center_x()
            .center_y()
            .layout(renderer, &digital_clock_limits);

        // Pre-Buttons TODO: get rid of it
        let cancel_limits = limits;
        let cancel_button = self.cancel_button.layout(renderer, &cancel_limits);

        let limits = limits.shrink(Size::new(
            0.0,
            digital_clock.bounds().height + cancel_button.bounds().height + 2.0 * SPACING as f32,
        ));

        // Clock-Canvas
        let mut clock = Row::<(), Renderer>::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .layout(renderer, &limits);

        clock.move_to(Point::new(
            clock.bounds().x + PADDING as f32,
            clock.bounds().y + PADDING as f32,
        ));

        digital_clock.move_to(Point::new(
            digital_clock.bounds().x + PADDING as f32,
            digital_clock.bounds().y + PADDING as f32 + SPACING as f32 + clock.bounds().height,
        ));

        // Buttons
        let cancel_limits = limits
            .clone()
            .max_width(((clock.bounds().width / 2.0) - BUTTON_SPACING as f32).max(0.0) as u32);

        let mut cancel_button = self.cancel_button.layout(renderer, &cancel_limits);

        let submit_limits = limits
            .clone()
            .max_width(((clock.bounds().width / 2.0) - BUTTON_SPACING as f32).max(0.0) as u32);

        let mut submit_button = self.submit_button.layout(renderer, &submit_limits);

        cancel_button.move_to(Point {
            x: cancel_button.bounds().x + PADDING as f32,
            y: cancel_button.bounds().y
                + clock.bounds().height
                + PADDING as f32
                + digital_clock.bounds().height
                + 2.0 * SPACING as f32,
        });

        submit_button.move_to(Point {
            x: submit_button.bounds().x + clock.bounds().width - submit_button.bounds().width
                + PADDING as f32,
            y: submit_button.bounds().y
                + clock.bounds().height
                + PADDING as f32
                + digital_clock.bounds().height
                + 2.0 * SPACING as f32,
        });

        let mut node = layout::Node::with_children(
            Size::new(
                clock.bounds().width + (2.0 * PADDING as f32),
                clock.bounds().height
                    + digital_clock.bounds().height
                    + cancel_button.bounds().height
                    + (2.0 * PADDING as f32)
                    + 2.0 * SPACING as f32,
            ),
            vec![clock, digital_clock, cancel_button, submit_button],
        );

        node.move_to(Point::new(
            (position.x - node.size().width / 2.0).max(0.0),
            (position.y - node.size().height / 2.0).max(0.0),
        ));

        node.move_to(Point::new(
            if node.bounds().x + node.bounds().width > bounds.width {
                (node.bounds().x - (node.bounds().width - (bounds.width - node.bounds().x)))
                    .max(0.0)
            } else {
                node.bounds().x
            },
            //node.bounds().x,
            if node.bounds().y + node.bounds().height > bounds.height {
                (node.bounds().y - (node.bounds().height - (bounds.height - node.bounds().y)))
                    .max(0.0)
            } else {
                node.bounds().y
            },
        ));

        node
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        messages: &mut Vec<Message>,
        renderer: &Renderer,
        clipboard: Option<&dyn Clipboard>,
    ) -> event::Status {
        if let event::Status::Captured = self.on_event_keyboard(
            event.clone(),
            layout,
            cursor_position,
            messages,
            renderer,
            clipboard,
        ) {
            return event::Status::Captured;
        }

        let mut children = layout.children();

        // Clock canvas
        let clock_layout = children.next().unwrap();
        let clock_status = self.on_event_clock(
            event.clone(),
            clock_layout,
            cursor_position,
            messages,
            renderer,
            clipboard,
        );

        // ----------- Digital clock ------------------
        let digital_clock_layout = children.next().unwrap().children().next().unwrap();
        let digital_clock_status = self.on_event_digital_clock(
            event.clone(),
            digital_clock_layout,
            cursor_position,
            messages,
            renderer,
            clipboard,
        );

        // ----------- Buttons ------------------------
        let cancel_button_layout = children.next().unwrap();

        let cancel_status = self.cancel_button.on_event(
            event.clone(),
            cancel_button_layout,
            cursor_position,
            messages,
            renderer,
            clipboard,
        );

        let submit_button_layout = children.next().unwrap();

        let mut fake_messages: Vec<Message> = Vec::new();

        let submit_status = self.submit_button.on_event(
            event,
            submit_button_layout,
            cursor_position,
            //messages,
            &mut fake_messages,
            renderer,
            clipboard,
        );

        if !fake_messages.is_empty() {
            let (hour, period) = if self.state.use_24h {
                (self.state.time.hour(), Period::H24)
            } else {
                let (period, hour) = self.state.time.hour12();
                (hour, if period { Period::Pm } else { Period::Am })
            };

            let time = if self.state.show_seconds {
                Time::Hms {
                    hour,
                    minute: self.state.time.minute(),
                    second: self.state.time.second(),
                    period,
                }
            } else {
                Time::Hm {
                    hour,
                    minute: self.state.time.minute(),
                    period,
                }
            };

            messages.push((self.on_submit)(time))
        }

        clock_status
            .merge(digital_clock_status)
            .merge(cancel_status)
            .merge(submit_status)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        defaults: &Renderer::Defaults,
        layout: iced_native::Layout<'_>,
        cursor_position: Point,
    ) -> Renderer::Output {
        <Renderer as self::Renderer>::draw(
            renderer,
            DrawEnvironment {
                defaults,
                layout,
                cursor_position,
                style_sheet: &self.style,
                viewport: None,
                focus: self.state.focus,
            },
            &self.state,
            &self.cancel_button,
            &self.submit_button,
        )
    }

    fn hash_layout(&self, state: &mut iced_native::Hasher, position: Point) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        (position.x as u32).hash(state);
        (position.y as u32).hash(state);
    }
}

/// The renderer of a [`TimePickerOverlay`](TimePickerOverlay).
///
/// Your renderer fill need to implement this trait before being
/// able to use a [`TimePicker`](crate::native::TimePicker) in your user
/// interface.
pub trait Renderer: iced_native::Renderer {
    /// The style supported by this renderer.
    type Style: Default;

    /// Draws a [`TimePickerOverlay`](TimePickerOverlay).
    fn draw<Message>(
        &mut self,
        env: DrawEnvironment<Self::Defaults, Self::Style, Focus>,
        state: &State,
        cancel_button: &Element<'_, Message, Self>,
        submit_button: &Element<'_, Message, Self>,
    ) -> Self::Output;
}

#[cfg(debug_assertions)]
impl Renderer for iced_native::renderer::Null {
    type Style = ();

    fn draw<Message>(
        &mut self,
        _env: DrawEnvironment<Self::Defaults, Self::Style, Focus>,
        _state: &State,
        _cancel_button: &Element<'_, Message, Self>,
        _submit_button: &Element<'_, Message, Self>,
    ) -> Self::Output {
    }
}

/// The state of the [`TimePickerOverlay`](TimePickerOverlay).
#[derive(Debug)]
pub struct State {
    pub(crate) time: NaiveTime,
    pub(crate) clock_cache_needs_clearance: bool,
    pub(crate) clock_cache: canvas::Cache,
    pub(crate) use_24h: bool,
    pub(crate) show_seconds: bool,
    pub(crate) clock_dragged: ClockDragged,
    pub(crate) focus: Focus,
    pub(crate) keyboard_modifiers: keyboard::Modifiers,
}

impl Default for State {
    fn default() -> Self {
        Self {
            time: Local::now().naive_local().time(),
            clock_cache_needs_clearance: false,
            clock_cache: canvas::Cache::new(),
            use_24h: false,
            show_seconds: false,
            clock_dragged: ClockDragged::None,
            focus: Focus::default(),
            keyboard_modifiers: keyboard::Modifiers::default(),
        }
    }
}

/// TODO
#[derive(Copy, Clone, Debug)]
pub enum ClockDragged {
    /// TODO
    None,

    /// TODO
    Hour,

    /// TODO
    Minute,

    /// TODO
    Second,
}

/// An enumeration of all focusable elements of the [`TimePickerOverlay`](TimePickerOverlay).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Focus {
    /// Nothing is in focus.
    None,

    /// The overlay itself is in focus.
    Overlay,

    /// The digital hour is in focus.
    DigitalHour,

    /// The digital minute is in focus.
    DigitalMinute,

    /// The digital second is in focus.
    DigitalSecond,

    /// The cancel button is in focus.
    Cancel,

    /// The submit button is in focus.
    Submit,
}

impl Focus {
    /// Gets the next focusable element.
    pub fn next(self, show_seconds: bool) -> Self {
        match self {
            Focus::None => Focus::Overlay,
            Focus::Overlay => Focus::DigitalHour,
            Focus::DigitalHour => Focus::DigitalMinute,
            Focus::DigitalMinute => {
                if show_seconds {
                    Focus::DigitalSecond
                } else {
                    Focus::Cancel
                }
            }
            Focus::DigitalSecond => Focus::Cancel,
            Focus::Cancel => Focus::Submit,
            Focus::Submit => Focus::Overlay,
        }
    }

    /// Gets the previous focusable element.
    pub fn previous(self, show_seconds: bool) -> Self {
        match self {
            Focus::None => Focus::None,
            Focus::Overlay => Focus::Submit,
            Focus::DigitalHour => Focus::Overlay,
            Focus::DigitalMinute => Focus::DigitalHour,
            Focus::DigitalSecond => Focus::DigitalMinute,
            Focus::Cancel => {
                if show_seconds {
                    Focus::DigitalSecond
                } else {
                    Focus::DigitalMinute
                }
            }
            Focus::Submit => Focus::Cancel,
        }
    }
}

impl Default for Focus {
    fn default() -> Self {
        Focus::None
    }
}
