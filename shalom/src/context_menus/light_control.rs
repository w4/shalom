use iced::{
    font::{Stretch, Weight},
    widget::{column, container, row, text},
    Alignment, Element, Font, Length, Renderer,
};

use crate::{oracle::Light, widgets::colour_picker::ColourPicker};

#[derive(Debug, Clone)]
pub struct LightControl {
    id: &'static str,
    name: Box<str>,
    hue: f32,
    saturation: f32,
    brightness: f32,
}

impl LightControl {
    pub fn new(id: &'static str, light: Light) -> Self {
        let (hue, saturation) = light.hs_color.unwrap_or_default();
        let brightness = light.brightness.unwrap_or_default();

        Self {
            id,
            name: light.friendly_name,
            hue,
            saturation: saturation / 100.,
            brightness: brightness / 255.,
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
            Message::OnMouseUp => Some(Event::UpdateLightColour {
                id: self.id,
                hue: self.hue,
                saturation: self.saturation,
                brightness: self.brightness,
            }),
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let colour_picker = ColourPicker::new(
            self.hue,
            self.saturation,
            self.brightness,
            Message::OnColourChange,
            Message::OnMouseUp,
        );

        container(column![
            text(&self.name).size(40).font(Font {
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

pub enum Event {
    UpdateLightColour {
        id: &'static str,
        hue: f32,
        saturation: f32,
        brightness: f32,
    },
}

#[derive(Clone, Debug)]
pub enum Message {
    OnColourChange(f32, f32, f32),
    OnMouseUp,
}
