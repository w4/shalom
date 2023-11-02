#![deny(clippy::pedantic)]

mod config;
mod context_menus;
mod hass_client;
mod oracle;
mod pages;
mod subscriptions;
mod theme;
mod widgets;

use std::sync::Arc;

use iced::{
    alignment::{Horizontal, Vertical},
    widget::{column, container, row, scrollable, svg, Column},
    window, Application, Command, ContentFit, Element, Length, Renderer, Settings, Subscription,
    Theme,
};

use crate::{
    config::Config,
    oracle::Oracle,
    theme::{Icon, Image},
    widgets::{context_menu::ContextMenu, mouse_area::mouse_area},
};

pub struct Shalom {
    page: ActivePage,
    context_menu: Option<ActiveContextMenu>,
    oracle: Option<Arc<Oracle>>,
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
            oracle: None,
        };

        // this is only best-effort to try and prevent blocking when loading
        // the omni-view, we don't need to block on this at boot
        tokio::task::spawn_blocking(Image::preload);

        let command = Command::perform(
            async {
                let config = load_config().await;
                let client = hass_client::create(config.home_assistant).await;
                Oracle::new(client.clone()).await
            },
            Message::Loaded,
        );

        (this, command)
    }

    fn title(&self) -> String {
        String::from("Shalom")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        #[allow(clippy::single_match)]
        match (message, &mut self.page, &mut self.context_menu) {
            (Message::Loaded(oracle), _, _) => {
                self.oracle = Some(oracle);
                self.page = ActivePage::Room(pages::room::Room::new(
                    "living_room",
                    self.oracle.clone().unwrap(),
                ));
            }
            (Message::CloseContextMenu, _, _) => {
                self.context_menu = None;
            }
            (Message::OpenOmniPage, _, _) => {
                self.page = ActivePage::Omni(pages::omni::Omni::new(self.oracle.clone().unwrap()));
            }
            (Message::OmniEvent(e), ActivePage::Omni(r), _) => match r.update(e) {
                Some(pages::omni::Event::OpenRoom(room)) => {
                    self.page = ActivePage::Room(pages::room::Room::new(
                        room,
                        self.oracle.clone().unwrap(),
                    ));
                }
                None => {}
            },
            (Message::RoomEvent(e), ActivePage::Room(r), _) => match r.update(e) {
                Some(pages::room::Event::OpenLightContextMenu(light)) => {
                    self.context_menu = Some(ActiveContextMenu::LightControl(
                        context_menus::light_control::LightControl::new(light),
                    ));
                }
                None => {}
            },
            (Message::LightControlMenu(e), _, Some(ActiveContextMenu::LightControl(menu))) => {
                match menu.update(e) {
                    Some(_) | None => {}
                }
            }
            _ => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let page_content = match &self.page {
            ActivePage::Loading => Element::from(column!["Loading...",].spacing(20)),
            ActivePage::Room(room) => room.view().map(Message::RoomEvent),
            ActivePage::Omni(omni) => omni.view().map(Message::OmniEvent),
        };

        let mut content = Column::new().push(scrollable(page_content));

        let (show_back, show_home) = match &self.page {
            // _ if self.page == self.homepage => (true, false),
            ActivePage::Loading => (false, false),
            ActivePage::Omni(_) => (false, true),
            ActivePage::Room(_) => (true, true),
        };

        let back = mouse_area(
            svg(Icon::Back)
                .height(32)
                .width(32)
                .content_fit(ContentFit::None),
        )
        .on_press(Message::OpenOmniPage);
        let home = mouse_area(
            svg(Icon::Home)
                .height(32)
                .width(32)
                .content_fit(ContentFit::None),
        );
        // .on_press(Message::ChangePage(self.homepage.clone()));

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
                ActiveContextMenu::LightControl(menu) => menu.view().map(Message::LightControlMenu),
            };

            ContextMenu::new(content, context_menu)
                .on_close(Message::CloseContextMenu)
                .into()
        } else {
            content.into()
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        match &self.page {
            ActivePage::Room(room) => room.subscription().map(Message::RoomEvent),
            ActivePage::Omni(omni) => omni.subscription().map(Message::OmniEvent),
            ActivePage::Loading => Subscription::none(),
        }
    }
}

async fn load_config() -> Config {
    let content = tokio::fs::read_to_string("./config.toml").await.unwrap();
    toml::from_str(&content).unwrap()
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Arc<Oracle>),
    CloseContextMenu,
    OpenOmniPage,
    OmniEvent(pages::omni::Message),
    RoomEvent(pages::room::Message),
    LightControlMenu(context_menus::light_control::Message),
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ActivePage {
    Loading,
    Room(pages::room::Room),
    Omni(pages::omni::Omni),
}

#[derive(Clone, Debug)]
pub enum ActiveContextMenu {
    LightControl(context_menus::light_control::LightControl),
}

fn main() {
    Shalom::run(Settings {
        antialiasing: true,
        window: window::Settings {
            min_size: Some((600, 600)),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
    .unwrap();
}
