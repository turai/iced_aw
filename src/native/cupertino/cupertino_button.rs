use iced_widget::{
    core::{
        self, event,
        layout::{Limits, Node},
        mouse::{self, Cursor},
        renderer::{self, Quad},
        touch,
        widget::Tree,
        Background, Clipboard, Color, Element, Event, Font, Layout, Length, Point, Rectangle,
        Shell, Size, Widget,
    },
    style::application,
    text, Text,
};

// INTERNAL //
use crate::graphics::SF_UI_ROUNDED;
use crate::native::cupertino::cupertino_colours::{secondary_system_fill, system_blue};
//

/**
 * `CupertinoButton`
 *
 * See
 *
 * <https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/cupertino/button.dart>
 *
 * for constants, and
 *
 * <https://api.flutter.dev/flutter/cupertino/CupertinoButton-class.html>
 *
 * for the Flutter example / expected usage.
 *
 */
#[allow(missing_debug_implementations)]
pub struct CupertinoButton<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: core::Renderer,
    Renderer::Theme: application::StyleSheet,
{
    on_pressed: Option<Message>,
    is_filled: bool,
    body: Element<'a, Message, Renderer>,

    /// `colour` is an option here because there is already logic to set the colour
    /// depending on if the button is enabled/disabled. But if the button causes a
    /// "destructive" behavior (e.g. a delete action), allow the user to override the
    /// colour to e.g. red.
    colour: Option<Color>,
}

impl<'a, Message, Renderer> Default for CupertinoButton<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: core::Renderer + core::text::Renderer<Font = Font> + 'a,
    Renderer::Theme: application::StyleSheet + text::StyleSheet,
{
    fn default() -> Self {
        Self {
            on_pressed: None,
            is_filled: false,
            body: Text::new("Hello").font(SF_UI_ROUNDED).into(),
            colour: None,
        }
    }
}

impl<'a, Message, Renderer> CupertinoButton<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: core::Renderer + core::text::Renderer<Font = Font> + 'a,
    Renderer::Theme: application::StyleSheet + text::StyleSheet,
{
    /// Creates a new [`CupertinoButton`] widget.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `on_pressed` callback of the [`CupertinoButton`].
    #[must_use]
    pub fn on_pressed(mut self, on_pressed: Option<Message>) -> Self {
        self.on_pressed = on_pressed;
        self
    }

    /// Sets the `is_filled` of the [`CupertinoButton`].
    #[must_use]
    pub fn is_filled(mut self, is_filled: bool) -> Self {
        self.is_filled = is_filled;
        self
    }

    /// Sets the `body` of the [`CupertinoButton`].
    #[must_use]
    pub fn body<T>(mut self, body: T) -> Self
    where
        Message: Clone,
        T: Into<Text<'a, Renderer>>,
    {
        let as_text = body.into().font(SF_UI_ROUNDED);

        self.body = Element::from(as_text);
        self
    }

    /// Sets the `colour` of the [`CupertinoButton`].
    #[must_use]
    pub fn colour(mut self, colour: Option<Color>) -> Self {
        self.colour = colour;
        self
    }
}

const VERTICAL_PADDING: f32 = 14.0;
// const HORIZONTAL_PADDING: f32 = 64.0;

impl<'a, Message, Renderer> Widget<Message, Renderer> for CupertinoButton<'a, Message, Renderer>
where
    Message: Clone,
    Renderer: core::Renderer + core::text::Renderer,
    Renderer::Theme: application::StyleSheet,
{
    fn width(&self) -> Length {
        self.body.as_widget().width()
    }
    fn height(&self) -> Length {
        self.body.as_widget().height()
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        return self.body.as_widget().layout(renderer, limits);
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        if self.is_filled {
            let colour: Color = if self.on_pressed.is_none() {
                secondary_system_fill()
            } else {
                system_blue(1.0)
            };

            let bounds: Rectangle = layout.bounds();
            let center: Point = bounds.center();

            let rectangle: Rectangle = Rectangle::new(
                Point {
                    x: bounds.x,
                    y: center.y - 3.0 * VERTICAL_PADDING,
                },
                Size {
                    width: bounds.width,
                    height: bounds.height + VERTICAL_PADDING,
                },
            );

            renderer.fill_quad(
                Quad {
                    bounds: rectangle,
                    border_radius: [16.0, 16.0, 16.0, 16.0].into(),
                    border_width: 5.0,
                    border_color: Color::TRANSPARENT,
                },
                Background::Color(colour),
            );
        }

        let new_style: &mut renderer::Style = &mut renderer::Style::default();

        new_style.clone_from(style);

        if self.colour.is_some() {
            new_style.text_color = self.colour.expect("Unable to retrieve the text colour");
        } else if self.is_filled && self.on_pressed.is_some() {
            new_style.text_color = Color::WHITE;
        } else if !self.is_filled && self.on_pressed.is_some() {
            new_style.text_color = system_blue(1.0);
        } else if self.is_filled && self.on_pressed.is_none() {
            new_style.text_color = Color::WHITE;
        } else {
            new_style.text_color = secondary_system_fill();
        }

        self.body
            .as_widget()
            .draw(state, renderer, theme, new_style, layout, cursor, viewport);
    }

    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                let bounds: Rectangle = layout.bounds();

                if self.on_pressed.as_ref().is_some() {
                    let cur_pos = cursor.position().unwrap_or_default();
                    let hit_x: bool =
                        ((bounds.x + 10.0)..(bounds.x + bounds.width)).contains(&cur_pos.x);

                    let hit_y: bool =
                        ((bounds.y - 14.0)..(bounds.y + bounds.height - 10.0)).contains(&cur_pos.y);

                    if hit_x && hit_y {
                        shell.publish(
                            self.on_pressed
                                .clone()
                                .expect("Unable to retrieve the pressed message"),
                        );
                        return event::Status::Captured;
                    }
                }
            }

            _ => {}
        }

        event::Status::Ignored
    }
}

impl<'a, Message, Renderer> From<CupertinoButton<'a, Message, Renderer>>
    for Element<'a, Message, Renderer>
where
    Message: Clone + 'a,
    Renderer: core::Renderer + core::text::Renderer<Font = Font> + 'a,
    Renderer::Theme: application::StyleSheet,
{
    fn from(alert: CupertinoButton<'a, Message, Renderer>) -> Self {
        Self::new(alert)
    }
}
