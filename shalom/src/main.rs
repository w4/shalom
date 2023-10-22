#![deny(clippy::pedantic)]

mod config;
mod hass_client;
mod oracle;
mod pages;
mod theme;
mod widgets;

use std::sync::Arc;

use iced::{
    alignment::{Horizontal, Vertical},
    font::{Stretch, Weight},
    widget::{column, container, row, scrollable, svg, text, vertical_slider, Column},
    Alignment, Application, Command, ContentFit, Element, Font, Length, Renderer, Settings, Theme,
};

use crate::{
    config::Config,
    theme::{Icon, Image},
    widgets::{context_menu::ContextMenu, mouse_area::mouse_area},
};

pub struct Shalom {
    page: ActivePage,
    context_menu: Option<ActiveContextMenu>,
    homepage: ActivePage,
}

impl Application for Shalom {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let this = Self {
            page: ActivePage::Loading,
            context_menu: None,
            homepage: ActivePage::Room("Living Room"),
        };

        // this is only best-effort to try and prevent blocking when loading
        // the omni-view, we don't need to block on this at boot
        tokio::task::spawn_blocking(Image::preload);

        let command = Command::perform(
            async {
                let config = load_config().await;
                let client = hass_client::create(config.home_assistant).await;

                Arc::new(client)
            },
            |_client| Message::Loaded,
        );

        (this, command)
    }

    fn title(&self) -> String {
        String::from("Shalom")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Loaded => {
                self.page = self.homepage.clone();
            }
            Message::CloseContextMenu => {
                self.context_menu = None;
            }
            Message::OpenContextMenu(menu) => {
                self.context_menu = Some(menu);
            }
            Message::ChangePage(page) => {
                self.page = page;
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let page_content = match &self.page {
            ActivePage::Loading => Element::from(column!["Loading...",].spacing(20)),
            ActivePage::Room(room) => {
                Element::from(pages::room::Room::new(room, Message::OpenContextMenu))
            }
            ActivePage::Omni => Element::from(pages::omni::Omni::new(Message::ChangePage)),
        };

        let mut content = Column::new().push(scrollable(page_content));

        let (show_back, show_home) = match &self.page {
            _ if self.page == self.homepage => (true, false),
            ActivePage::Loading => (false, false),
            ActivePage::Omni => (false, true),
            ActivePage::Room(_) => (true, true),
        };

        let back = mouse_area(
            svg(Icon::Back)
                .height(32)
                .width(32)
                .content_fit(ContentFit::None),
        )
        .on_press(Message::ChangePage(ActivePage::Omni));
        let home = mouse_area(
            svg(Icon::Home)
                .height(32)
                .width(32)
                .content_fit(ContentFit::None),
        )
        .on_press(Message::ChangePage(self.homepage.clone()));

        let navigation = match (show_back, show_home) {
            (true, true) => Some(Element::from(
                row![
                    back,
                    container(home)
                        .width(Length::Fill)
                        .align_x(Horizontal::Right),
                ]
                .height(32),
            )),
            (false, true) => Some(Element::from(
                row![container(home)
                    .width(Length::Fill)
                    .align_x(Horizontal::Right),]
                .height(32),
            )),
            (true, false) => Some(Element::from(back)),
            (false, false) => None,
        };

        if let Some(navigation) = navigation {
            content = content.push(
                container(navigation)
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .align_y(Vertical::Bottom)
                    .padding(40),
            );
        }

        if let Some(context_menu) = &self.context_menu {
            let context_menu = match context_menu {
                ActiveContextMenu::LightOptions(name) => container(column![
                    text(name).size(40).font(Font {
                        weight: Weight::Bold,
                        stretch: Stretch::Condensed,
                        ..Font::with_name("Helvetica Neue")
                    }),
                    row![vertical_slider(0..=100, 0, |_v| Message::Loaded).height(200)]
                        .align_items(Alignment::Center)
                ])
                .width(Length::Fill)
                .padding(40),
            };

            ContextMenu::new(content, context_menu)
                .on_close(Message::CloseContextMenu)
                .into()
        } else {
            content.into()
        }
    }
}

async fn load_config() -> Config {
    let content = tokio::fs::read_to_string("./config.toml").await.unwrap();
    toml::from_str(&content).unwrap()
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded,
    CloseContextMenu,
    ChangePage(ActivePage),
    OpenContextMenu(ActiveContextMenu),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivePage {
    Loading,
    Room(&'static str),
    Omni,
}

#[derive(Clone, Debug)]
pub enum ActiveContextMenu {
    LightOptions(&'static str),
}

fn main() {
    Shalom::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
    .unwrap();
}
