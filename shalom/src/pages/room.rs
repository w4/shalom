pub mod lights;
pub mod listen;

use std::sync::Arc;

use iced::{
    advanced::graphics::core::Element,
    font::{Stretch, Weight},
    theme,
    widget::{
        container, row, scrollable,
        scrollable::{Direction, Properties, Viewport},
        text, Column,
    },
    Color, Font, Length, Renderer, Subscription, Theme,
};

use crate::{
    config::Config,
    oracle::Oracle,
    subscriptions::MaybePendingImage,
    widgets::{
        image_background::image_background,
        room_navigation::{Page, RoomNavigation},
    },
};

const PADDING: u16 = 40;
const SPACE_TOP: u16 = 51;

#[derive(Debug)]
pub struct Room {
    id: &'static str,
    room: crate::oracle::Room,
    lights: lights::Lights,
    listen: listen::Listen,
    current_page: Page,
    dy: f32,
    pending_visible_toggle: bool,
}

impl Room {
    pub fn new(id: &'static str, oracle: Arc<Oracle>, config: Arc<Config>) -> Self {
        let room = oracle.room(id).clone();

        Self {
            id,
            listen: listen::Listen::new(oracle.clone(), &room, config),
            lights: lights::Lights::new(oracle, &room),
            room,
            current_page: Page::Listen,
            dy: 0.0,
            pending_visible_toggle: false,
        }
    }

    pub fn room_id(&self) -> &'static str {
        self.id
    }

    pub fn update(&mut self, event: Message) -> Option<Event> {
        match event {
            Message::Lights(v) => self.lights.update(v).map(Event::Lights),
            Message::Listen(listen::Message::OnSearchVisibleToggle)
                if self.listen.search.is_open() && self.dy > 0.0 =>
            {
                // intercept search toggles on listen so we can scroll our scrollable to
                // the top first
                self.pending_visible_toggle = true;
                None
            }
            Message::Listen(v) => self.listen.update(v).map(Event::Listen),
            Message::ChangePage(page) => {
                self.dy = 0.0;
                self.current_page = page;
                None
            }
            Message::Exit => {
                self.dy = 0.0;
                Some(Event::Exit)
            }
            Message::OnContentScroll(viewport) => {
                self.dy = viewport.absolute_offset().y;
                None
            }
            Message::OnContentAnimateFinished => {
                if self.pending_visible_toggle {
                    self.pending_visible_toggle = false;
                    self.listen
                        .update(listen::Message::OnSearchVisibleToggle)
                        .map(Event::Listen)
                } else {
                    None
                }
            }
        }
    }

    pub fn view(&self, style: &Theme) -> Element<'_, Message, Renderer> {
        let header = text(self.room.name.as_ref())
            .size(60)
            .font(Font {
                weight: Weight::Bold,
                stretch: Stretch::Condensed,
                ..Font::with_name("Helvetica Neue")
            })
            .style(theme::Text::Color(Color::WHITE));

        let (mut current, needs_scrollable) = match self.current_page {
            Page::Climate => (Element::from(row![]), false),
            Page::Listen => (
                self.listen.view(style).map(Message::Listen),
                self.listen.search.is_open(),
            ),
            Page::Lights => (
                container(self.lights.view().map(Message::Lights))
                    .padding([0, PADDING, 0, PADDING])
                    .into(),
                false,
            ),
        };

        let (header, padding_mult) = if let Page::Listen = self.current_page {
            let padding_mult = if needs_scrollable {
                (self.dy / f32::from(SPACE_TOP)).min(1.0)
            } else {
                0.0
            };

            (
                self.listen
                    .header_magic(header.clone(), padding_mult)
                    .map(Message::Listen),
                padding_mult,
            )
        } else {
            (Element::from(header), 0.0)
        };

        let padding = f32::from(PADDING) * (1.0 - padding_mult);
        let header = container(header).padding([padding, padding, 0.0, padding]);

        let mut col = Column::new()
            .spacing(20.0 * (1.0 - padding_mult))
            .push(header);

        if needs_scrollable {
            current = scrollable(container(current).width(Length::Fill).padding([
                f32::from(PADDING + 30) * padding_mult,
                0.0,
                0.0,
                0.0,
            ]))
            .direction(Direction::Vertical(
                Properties::default().scroller_width(0).width(0),
            ))
            .on_scroll(Message::OnContentScroll)
            .on_animate_finished(Message::OnContentAnimateFinished)
            .scroll_to_top(self.pending_visible_toggle)
            .into();
        }

        col = col.push(current);

        let background = match self.current_page {
            Page::Listen => self
                .listen
                .background
                .as_ref()
                .and_then(MaybePendingImage::handle),
            _ => None,
        };

        row![
            RoomNavigation::new(self.current_page)
                .width(Length::FillPortion(2))
                .on_change(Message::ChangePage)
                .on_exit(Message::Exit),
            image_background(
                background.unwrap_or_else(|| crate::theme::Image::Sunset.into()),
                col.width(Length::Fill).into(),
            )
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
    OnContentScroll(Viewport),
    OnContentAnimateFinished,
    Exit,
}
