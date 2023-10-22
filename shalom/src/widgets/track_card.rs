use iced::{
    advanced::graphics::core::Element,
    theme::Text,
    widget::{
        column as icolumn, component,
        image::{self, Image},
        row, text, Component,
    },
    Alignment, Renderer,
};

use crate::theme::colours::SLATE_400;

pub fn track_card(artist: String, song: String, loved: bool) -> TrackCard {
    TrackCard {
        artist,
        song,
        loved,
    }
}

pub struct TrackCard {
    artist: String,
    song: String,
    loved: bool,
}

impl<M> Component<M, Renderer> for TrackCard {
    type State = State;
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {}
    }

    fn view(&self, _state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        row![
            Image::new(image::Handle::from_path("/tmp/tmp.jpg"))
                .width(64)
                .height(64),
            icolumn![
                text(&self.song).size(14),
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
