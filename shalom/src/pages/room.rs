pub mod lights;
pub mod listen;

use std::sync::Arc;

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    theme,
    widget::{row, text, Column},
    Color, Font, Length, Renderer, Subscription,
};

use crate::{
    oracle::Oracle,
    widgets::{
        image_background::image_background,
        room_navigation::{Page, RoomNavigation},
    },
};

#[derive(Debug)]
pub struct Room {
    id: &'static str,
    room: crate::oracle::Room,
    lights: lights::Lights,
    listen: listen::Listen,
    current_page: Page,
}

impl Room {
    pub fn new(id: &'static str, oracle: Arc<Oracle>) -> Self {
        let room = oracle.room(id).clone();

        Self {
            id,
            listen: listen::Listen::new(oracle.clone(), &room),
            lights: lights::Lights::new(oracle, &room),
            room,
            current_page: Page::Listen,
        }
    }

    pub fn room_id(&self) -> &'static str {
        self.id
    }

    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::Lights(v) => self.lights.update(v).map(Event::Lights),
            Message::Listen(v) => self.listen.update(v).map(Event::Listen),
            Message::ChangePage(page) => {
                self.current_page = page;
                None
            }
            Message::Exit => Some(Event::Exit),
        }
    }

    pub fn view(&self) -> Element<'_, Message, Renderer> {
        let header = text(self.room.name.as_ref())
            .size(60)
            .font(Font {
                weight: Weight::Bold,
                stretch: Stretch::Condensed,
                ..Font::with_name("Helvetica Neue")
            })
            .style(theme::Text::Color(Color::WHITE));

        let mut col = Column::new().spacing(20).padding(40).push(header);

        col = col.push(match self.current_page {
            Page::Climate => Element::from(row![]),
            Page::Listen => self.listen.view().map(Message::Listen),
            Page::Lights => self.lights.view().map(Message::Lights),
        });

        row![
            RoomNavigation::new(self.current_page)
                .width(Length::FillPortion(2))
                .on_change(Message::ChangePage)
                .on_exit(Message::Exit),
            image_background(crate::theme::Image::Sunset, col.width(Length::Fill).into())
                .width(Length::FillPortion(15))
                .height(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.listen.subscription().map(Message::Listen),
            self.lights.subscription().map(Message::Lights),
        ])
    }
}

pub enum Event {
    Lights(lights::Event),
    Listen(listen::Event),
    Exit,
}

#[derive(Clone, Debug)]
pub enum Message {
    Lights(lights::Message),
    Listen(listen::Message),
    ChangePage(Page),
    Exit,
}
