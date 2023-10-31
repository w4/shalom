use std::sync::Arc;

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    widget::{column, component, scrollable, text, Column, Component, Row},
    Font, Renderer,
};
use itertools::Itertools;

use crate::{oracle::Oracle, theme::Image, widgets::image_card, ActivePage};

pub struct Omni<M> {
    oracle: Arc<Oracle>,
    open_page: fn(ActivePage) -> M,
}

impl<M> Omni<M> {
    pub fn new(oracle: Arc<Oracle>, open_page: fn(ActivePage) -> M) -> Self {
        Self { oracle, open_page }
    }
}

impl<M: Clone> Component<M, Renderer> for Omni<M> {
    type State = State;
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            Event::OpenRoom(room) => Some((self.open_page)(ActivePage::Room(room))),
        }
    }

    fn view(&self, _state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let greeting = text("Good Evening").size(60).font(Font {
            weight: Weight::Bold,
            stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let room = |room, image| {
            image_card::image_card(image, room).on_press(Event::OpenRoom(room))
            // .height(Length::Fixed(128.0))
            // .width(Length::FillPortion(1))
        };

        let rooms = self
            .oracle
            .rooms()
            .map(|r| room(r.name.as_ref(), determine_image(&r.name)))
            .chunks(2)
            .into_iter()
            .map(|children| children.into_iter().fold(Row::new().spacing(10), Row::push))
            .fold(Column::new().spacing(10), Column::push);

        scrollable(
            column![
                greeting,
                crate::widgets::cards::weather::WeatherCard::new(self.oracle.clone()),
                rooms,
            ]
            .spacing(20)
            .padding(40),
        )
        .into()
    }
}

fn determine_image(name: &str) -> Image {
    match name {
        "Kitchen" => Image::Kitchen,
        "Bathroom" => Image::Bathroom,
        "Bedroom" => Image::Bedroom,
        "Dining Room" => Image::DiningRoom,
        _ => Image::LivingRoom,
    }
}

#[derive(Default, Hash)]
pub struct State {}

#[derive(Clone)]
pub enum Event {
    OpenRoom(&'static str),
}

impl<'a, M> From<Omni<M>> for Element<'a, M, Renderer>
where
    M: 'a + Clone,
{
    fn from(card: Omni<M>) -> Self {
        component(card)
    }
}
