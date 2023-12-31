use iced::{
    advanced::graphics::core::Element,
    theme::Text,
    widget::{
        column as icolumn, component, container,
        image::{self, Image},
        row, text, vertical_space, Component,
    },
    Alignment, Background, Color, Renderer, Theme,
};

use crate::theme::colours::{SLATE_200, SLATE_400};

pub fn track_card(artist: String, song: String, image: Option<image::Handle>) -> TrackCard {
    TrackCard {
        artist,
        song,
        image,
    }
}

pub struct TrackCard {
    artist: String,
    song: String,
    image: Option<image::Handle>,
}

impl<M> Component<M, Renderer> for TrackCard {
    type State = State;
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {}
    }

    fn view(&self, _state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let image =
            if let Some(handle) = self.image.clone() {
                Element::from(Image::new(handle).width(64).height(64))
            } else {
                Element::from(container(vertical_space(0)).width(64).height(64).style(
                    |_t: &Theme| container::Appearance {
                        background: Some(Background::Color(SLATE_200)),
                        ..container::Appearance::default()
                    },
                ))
            };

        row![
            image,
            icolumn![
                text(&self.song).size(14).style(Text::Color(Color::WHITE)),
                text(&self.artist).style(Text::Color(SLATE_400)).size(14)
            ]
        ]
        .align_items(Alignment::Center)
        .spacing(10)
        .into()
    }
}

#[derive(Default)]
pub struct State {}

#[derive(Clone)]
pub enum Event {}

impl<'a, M> From<TrackCard> for Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(card: TrackCard) -> Self {
        component(card)
    }
}
