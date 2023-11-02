use iced::{
    font::{Stretch, Weight},
    widget::{column, container, row, text},
    Alignment, Element, Font, Length, Renderer,
};

use crate::widgets::colour_picker::ColourPicker;

#[derive(Debug, Clone)]
pub struct LightControl {
    id: &'static str,
    hue: f32,
    saturation: f32,
    brightness: f32,
}

impl LightControl {
    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            hue: 0.0,
            saturation: 0.0,
            brightness: 0.0,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::OnColourChange(hue, saturation, brightness) => {
                self.hue = hue;
                self.saturation = saturation;
                self.brightness = brightness;
                None
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let colour_picker = ColourPicker::new(
            self.hue,
            self.saturation,
            self.brightness,
            Message::OnColourChange,
        );

        container(column![
            text(self.id).size(40).font(Font {
                weight: Weight::Bold,
                stretch: Stretch::Condensed,
                ..Font::with_name("Helvetica Neue")
            }),
            row![colour_picker,]
                .align_items(Alignment::Center)
                .spacing(20)
        ])
        .width(Length::Fill)
        .padding(40)
        .into()
    }
}

pub enum Event {}

#[derive(Clone, Debug)]
pub enum Message {
    OnColourChange(f32, f32, f32),
}
