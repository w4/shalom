use iced::{
    advanced::graphics::core::Element,
    event::Status,
    mouse,
    mouse::{Button, Cursor},
    touch,
    widget::{
        canvas,
        canvas::{Cache, Event, Frame, Geometry, Path, Stroke, Style},
        component, Component, Row,
    },
    Color, Point, Rectangle, Renderer, Size, Theme,
};
use palette::IntoColor;

use crate::widgets::forced_rounded::forced_rounded;

pub struct ColourPicker<Event> {
    hue: f32,
    saturation: f32,
    brightness: f32,
    on_change: fn(f32, f32, f32) -> Event,
    on_mouse_up: Event,
}

impl<Event> ColourPicker<Event> {
    pub fn new(
        hue: f32,
        saturation: f32,
        brightness: f32,
        on_change: fn(f32, f32, f32) -> Event,
        on_mouse_up: Event,
    ) -> Self {
        Self {
            hue,
            saturation,
            brightness,
            on_change,
            on_mouse_up,
        }
    }
}

impl<Event: Clone> Component<Event, Renderer> for ColourPicker<Event> {
    type State = ();
    type Event = Message;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<Event> {
        match event {
            Message::SaturationBrightnessChange(saturation, brightness) => {
                Some((self.on_change)(self.hue, saturation, brightness))
            }
            Message::HueChanged(hue) => {
                Some((self.on_change)(hue, self.saturation, self.brightness))
            }
            Message::MouseUp => Some(self.on_mouse_up.clone()),
        }
    }

    fn view(&self, _state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let saturation_brightness_picker = forced_rounded(
            canvas(SaturationBrightnessPicker::new(
                self.hue,
                self.saturation,
                self.brightness,
                Message::SaturationBrightnessChange,
                Message::MouseUp,
            ))
            .height(192)
            .width(192),
        );

        let hue_slider = forced_rounded(
            canvas(HueSlider::new(
                self.hue,
                Message::HueChanged,
                Message::MouseUp,
            ))
            .height(192)
            .width(32),
        );

        Row::new()
            .push(saturation_brightness_picker)
            .push(hue_slider)
            .spacing(0)
            .into()
    }
}

impl<'a, M> From<ColourPicker<M>> for Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(card: ColourPicker<M>) -> Self {
        component(card)
    }
}

#[derive(Clone)]
pub enum Message {
    SaturationBrightnessChange(f32, f32),
    HueChanged(f32),
    MouseUp,
}

pub struct HueSlider<Message> {
    hue: f32,
    on_hue_change: fn(f32) -> Message,
    on_mouse_up: Message,
}

impl<Message> HueSlider<Message> {
    fn new(hue: f32, on_hue_change: fn(f32) -> Message, on_mouse_up: Message) -> Self {
        Self {
            hue,
            on_hue_change,
            on_mouse_up,
        }
    }
}

impl<Message: Clone> canvas::Program<Message> for HueSlider<Message> {
    type State = HueSliderState;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (Status, Option<Message>) {
        let (update, mouse_up) = match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if cursor.is_over(bounds) =>
            {
                state.is_dragging = true;
                (true, false)
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
                if state.is_dragging =>
            {
                state.is_dragging = false;
                (false, true)
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. })
                if state.is_dragging =>
            {
                (true, false)
            }
            _ => (false, false),
        };

        if update {
            if let Some(position) = cursor.position_in(bounds) {
                state.arrow_cache.clear();

                let hue = (position.y / bounds.height) * 360.;
                (Status::Captured, Some((self.on_hue_change)(hue)))
            } else {
                (Status::Captured, None)
            }
        } else if mouse_up {
            (Status::Captured, Some(self.on_mouse_up.clone()))
        } else {
            (Status::Ignored, None)
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // Draw the hue gradient
        let content = state
            .preview_cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let size = frame.size();

                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss
                )]
                for y in 0..size.height as u32 {
                    let hue = (y as f32 / size.height) * 360.0;
                    let color = colour_from_hsb(hue, 1.0, 1.0);
                    frame.fill_rectangle(
                        Point::new(0.0, y as f32),
                        Size::new(size.width, 1.0),
                        color,
                    );
                }
            });

        // Draw the user's selection on the gradient
        let arrow = state
            .arrow_cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let size = frame.size();

                let arrow_width = 10.0;
                let arrow_height = 10.0;
                let arrow_x = size.width;
                let arrow_y = (self.hue / 360.0) * size.height - (arrow_height / 2.0);

                let arrow = Path::new(|p| {
                    p.move_to(Point::new(arrow_x, arrow_y));
                    p.line_to(Point::new(arrow_x, arrow_y - arrow_width));
                    p.line_to(Point::new(
                        arrow_x - arrow_height,
                        arrow_y - (arrow_width / 2.0),
                    ));
                    p.line_to(Point::new(arrow_x, arrow_y));
                    p.close();
                });

                frame.fill(&arrow, Color::BLACK);
            });

        vec![content, arrow]
    }
}

#[derive(Default)]
pub struct HueSliderState {
    is_dragging: bool,
    preview_cache: Cache,
    arrow_cache: Cache,
}

pub struct SaturationBrightnessPicker<Message> {
    hue: f32,
    saturation: f32,
    brightness: f32,
    on_change: fn(f32, f32) -> Message,
    on_mouse_up: Message,
}

impl<Message> SaturationBrightnessPicker<Message> {
    pub fn new(
        hue: f32,
        saturation: f32,
        brightness: f32,
        on_change: fn(f32, f32) -> Message,
        on_mouse_up: Message,
    ) -> Self {
        Self {
            hue,
            saturation,
            brightness,
            on_change,
            on_mouse_up,
        }
    }
}

impl<Message: Clone> canvas::Program<Message> for SaturationBrightnessPicker<Message> {
    type State = SaturationBrightnessPickerState;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (Status, Option<Message>) {
        // copy hue from self to state to figure out if the box needs to be
        // rerendered
        #[allow(clippy::float_cmp)]
        if self.hue != state.hue {
            state.hue = self.hue;
            state.content_cache.clear();
        }

        let (update, mouse_up) = match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if cursor.is_over(bounds) =>
            {
                state.is_dragging = true;
                (true, false)
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left))
            | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
                if state.is_dragging =>
            {
                state.is_dragging = false;
                (false, true)
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(touch::Event::FingerMoved { .. })
                if state.is_dragging =>
            {
                (true, false)
            }
            _ => (false, false),
        };

        if update {
            if let Some(position) = cursor.position_in(bounds) {
                state.circle_cache.clear();

                let saturation = position.x / bounds.width;
                let brightness = 1.0 - (position.y / bounds.height);

                (
                    Status::Captured,
                    Some((self.on_change)(saturation, brightness)),
                )
            } else {
                (Status::Ignored, None)
            }
        } else if mouse_up {
            (Status::Captured, Some(self.on_mouse_up.clone()))
        } else {
            (Status::Ignored, None)
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // Draw the saturation-brightness box
        let content = state
            .content_cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let size = frame.size();

                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    clippy::cast_precision_loss
                )]
                for x in 0..size.width as u32 {
                    for y in 0..size.height as u32 {
                        let saturation = x as f32 / size.width;
                        let brightness = 1.0 - (y as f32 / size.height);
                        let color = colour_from_hsb(self.hue, saturation, brightness);

                        frame.fill_rectangle(
                            Point::new(x as f32, y as f32),
                            Size::new(1.0, 1.0),
                            color,
                        );
                    }
                }
            });

        // Draw the user's selection on the box
        let circle = state
            .circle_cache
            .draw(renderer, bounds.size(), |frame: &mut Frame| {
                let size = frame.size();

                let circle_x = self.saturation * size.width;
                let circle_y = (1.0 - self.brightness) * size.height;
                let circle_radius = 5.0;

                let circle = Path::circle(Point::new(circle_x, circle_y), circle_radius);

                frame.stroke(
                    &circle,
                    Stroke {
                        style: Style::Solid(Color::BLACK),
                        width: 1.,
                        ..Stroke::default()
                    },
                );
            });

        vec![content, circle]
    }
}

#[derive(Default)]
pub struct SaturationBrightnessPickerState {
    is_dragging: bool,
    content_cache: Cache,
    circle_cache: Cache,
    hue: f32,
}

pub fn colour_from_hsb(hue: f32, saturation: f32, brightness: f32) -> Color {
    let rgb: palette::Srgb = palette::Hsv::new(hue, saturation, brightness).into_color();
    Color::from_rgb(rgb.red, rgb.green, rgb.blue)
}

pub fn clamp_to_u8(v: f32) -> u8 {
    let clamped = v.clamp(0., 1.);
    let scaled = (clamped * 255.).round();

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        scaled as u8
    }
}
