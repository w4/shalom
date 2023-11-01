use std::sync::Arc;

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    widget::{column, scrollable, text, Column, Row},
    Font, Renderer,
};
use itertools::Itertools;

use crate::{oracle::Oracle, theme::Image, widgets::image_card};

#[derive(Debug)]
pub struct Omni {
    oracle: Arc<Oracle>,
}

impl Omni {
    pub fn new(oracle: Arc<Oracle>) -> Self {
        Self { oracle }
    }
}

impl Omni {
    #[allow(
        clippy::unnecessary_wraps,
        clippy::needless_pass_by_value,
        clippy::unused_self
    )]
    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::OpenRoom(room) => Some(Event::OpenRoom(room)),
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let greeting = text("Good Evening").size(60).font(Font {
            weight: Weight::Bold,
            stretch: Stretch::Condensed,
            ..Font::with_name("Helvetica Neue")
        });

        let room = |id, room, image| {
            image_card::image_card(image, room).on_press(Message::OpenRoom(id))
            // .height(Length::Fixed(128.0))
            // .width(Length::FillPortion(1))
        };

        let rooms = self
            .oracle
            .rooms()
            .map(|(id, r)| room(id, r.name.as_ref(), determine_image(&r.name)))
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

#[derive(Clone, Debug)]
pub enum Event {
    OpenRoom(&'static str),
}

#[derive(Clone, Debug)]
pub enum Message {
    OpenRoom(&'static str),
}
