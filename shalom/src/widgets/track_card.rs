use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    theme::Text,
    widget::{
        column as icolumn, component, container,
        image::{self, Image},
        text, vertical_space, Component,
    },
    Background, Color, Font, Renderer, Theme,
};

use crate::theme::colours::SLATE_200;

pub fn track_card(
    artist: &str,
    song: &str,
    image: Option<image::Handle>,
    artist_logo: Option<image::Handle>,
) -> TrackCard {
    TrackCard {
        artist: artist.to_uppercase(),
        song: format!("\"{}\"", song.to_uppercase()),
        image,
        artist_logo,
    }
}

pub struct TrackCard {
    artist: String,
    song: String,
    image: Option<image::Handle>,
    artist_logo: Option<image::Handle>,
}

impl<M> Component<M, Renderer> for TrackCard {
    type State = State;
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {}
    }

    fn view(&self, _state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let image = if let Some(handle) = self.image.clone() {
            Element::from(Image::new(handle).width(192).height(192))
        } else {
            Element::from(container(vertical_space(0)).width(192).height(192).style(
                |_t: &Theme| container::Appearance {
                    background: Some(Background::Color(SLATE_200)),
                    ..container::Appearance::default()
                },
            ))
        };

        let artist = if let Some(handle) = self.artist_logo.clone() {
            Element::from(Image::new(handle).height(64))
        } else {
            Element::from(
                text(&self.artist)
                    .size(49)
                    .style(Text::Color(Color::WHITE))
                    .font(Font {
                        weight: Weight::Bold,
                        stretch: Stretch::Condensed,
                        ..Font::with_name("Helvetica Neue")
                    }),
            )
        };

        let song = text(&self.song)
            .size(24)
            .style(Text::Color(Color::WHITE))
            .font(Font {
                weight: Weight::Medium,
                ..Font::with_name("Helvetica Neue")
            });

        icolumn![icolumn![image, artist,].spacing(5), song,].into()
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
