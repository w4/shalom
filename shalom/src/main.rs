#![deny(clippy::pedantic)]
#![allow(clippy::struct_field_names)]

mod config;
mod context_menus;
mod hass_client;
mod magic;
mod oracle;
mod pages;
mod subscriptions;
mod theme;
mod widgets;

use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

use iced::{
    alignment::{Horizontal, Vertical},
    widget::container,
    window, Application, Command, Element, Length, Renderer, Settings, Size, Subscription, Theme,
};

use crate::{
    config::Config,
    oracle::Oracle,
    theme::Image,
    widgets::{
        context_menu::ContextMenu,
        floating_element::{Anchor, FloatingElement},
        spinner::CupertinoSpinner,
        toast::{Toast, ToastElement},
    },
};

pub struct Shalom {
    page: ActivePage,
    context_menu: Option<ActiveContextMenu>,
    oracle: Option<Arc<Oracle>>,
    home_room: Option<&'static str>,
    theme: Theme,
    config: Option<Arc<Config>>,
    toast: BTreeMap<u8, Toast>,
}

impl Shalom {
    fn push_toast(&mut self, toast: Toast) {
        let highest_key = self
            .toast
            .last_key_value()
            .map(|(i, _)| *i)
            .unwrap_or_default();

        self.toast.insert(highest_key, toast);
    }

    fn build_home_route(&self) -> ActivePage {
        self.home_room.map_or_else(
            || self.build_omni_route(),
            |room| self.build_room_route(room),
        )
    }

    fn build_room_route(&self, room: &'static str) -> ActivePage {
        ActivePage::Room(pages::room::Room::new(
            room,
            self.oracle.as_ref().unwrap().clone(),
            self.config.as_ref().unwrap().clone(),
        ))
    }

    fn build_omni_route(&self) -> ActivePage {
        ActivePage::Omni(pages::omni::Omni::new(
            self.oracle.as_ref().unwrap().clone(),
        ))
    }

    fn handle_room_event(&mut self, e: pages::room::Message) -> Command<Message> {
        let ActivePage::Room(r) = &mut self.page else {
            return Command::none();
        };

        match r.update(e) {
            Some(pages::room::Event::Lights(e)) => self.handle_light_event(e),
            Some(pages::room::Event::Listen(e)) => self.handle_listen_event(e),
            Some(pages::room::Event::Exit) => {
                self.page = self.build_omni_route();
                Command::none()
            }
            None => Command::none(),
        }
    }

    fn handle_light_event(&mut self, event: pages::room::lights::Event) -> Command<Message> {
        match event {
            pages::room::lights::Event::SetLightState(id, state) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.set_light_state(id, state).await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::lights::Event::OpenLightContextMenu(id) => {
                if let Some(light) = self.oracle.as_ref().and_then(|o| o.fetch_light(id)) {
                    self.context_menu = Some(ActiveContextMenu::LightControl(
                        context_menus::light_control::LightControl::new(id, light),
                    ));
                }

                Command::none()
            }
        }
    }

    fn handle_listen_event(&mut self, event: pages::room::listen::Event) -> Command<Message> {
        match event {
            pages::room::listen::Event::SetSpeakerVolume(id, new) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.speaker(id).set_volume(new).await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::SetSpeakerPosition(id, new) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.speaker(id).seek(new).await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::SetSpeakerPlaying(id, new) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move {
                        let speaker = oracle.speaker(id);
                        if new {
                            speaker.play().await;
                        } else {
                            speaker.pause().await;
                        }
                    },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::SetSpeakerMuted(id, new) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.speaker(id).set_mute(new).await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::SetSpeakerRepeat(id, new) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.speaker(id).set_repeat(new).await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::SpeakerNextTrack(id) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.speaker(id).next().await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::SpeakerPreviousTrack(id) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.speaker(id).previous().await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::SetSpeakerShuffle(id, new) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                Command::perform(
                    async move { oracle.speaker(id).set_shuffle(new).await },
                    Message::UpdateLightResult,
                )
            }
            pages::room::listen::Event::PlayTrack(id, uri) => {
                let oracle = self.oracle.as_ref().unwrap().clone();

                self.push_toast(Toast {
                    text: "Song added to queue".to_string(),
                    start: Instant::now(),
                    ttl: Duration::from_secs(5),
                });

                Command::perform(
                    async move { oracle.speaker(id).play_track(uri).await },
                    Message::PlayTrackResult,
                )
            }
        }
    }
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
            home_room: Some("living_room"),
            theme: Theme::default(),
            config: None,
            toast: BTreeMap::new(),
        };

        // this is only best-effort to try and prevent blocking when loading
        // the omni-view, we don't need to block on this at boot
        tokio::task::spawn_blocking(Image::preload);

        let command = Command::perform(
            async {
                let config = load_config().await;
                let client = hass_client::create(config.home_assistant.clone()).await;
                (Oracle::new(client.clone()).await, config)
            },
            Message::Loaded,
        );

        (this, command)
    }

    fn title(&self) -> String {
        String::from("Shalom")
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        #[allow(clippy::single_match)]
        match (message, &mut self.page, &mut self.context_menu) {
            (Message::Loaded((oracle, config)), _, _) => {
                self.oracle = Some(oracle);
                self.config = Some(Arc::new(config));
                self.page = self.build_home_route();
                Command::none()
            }
            (Message::CloseContextMenu, _, _) => {
                self.context_menu = None;
                Command::none()
            }
            (Message::OpenOmniPage, _, _) => {
                self.page = self.build_omni_route();
                Command::none()
            }
            (Message::OpenHomePage, _, _) => {
                self.page = self.build_home_route();
                Command::none()
            }
            (Message::OmniEvent(e), ActivePage::Omni(r), _) => match r.update(e) {
                Some(pages::omni::Event::OpenRoom(room)) => {
                    self.page = self.build_room_route(room);
                    Command::none()
                }
                None => Command::none(),
            },
            (Message::RoomEvent(e), _, _) => self.handle_room_event(e),
            (Message::LightControlMenu(e), _, Some(ActiveContextMenu::LightControl(menu))) => {
                match menu.update(e) {
                    Some(context_menus::light_control::Event::UpdateLightColour {
                        id,
                        hue,
                        saturation,
                        brightness,
                    }) => {
                        let oracle = self.oracle.as_ref().unwrap().clone();

                        Command::perform(
                            async move { oracle.update_light(id, hue, saturation, brightness).await },
                            Message::UpdateLightResult,
                        )
                    }
                    None => Command::none(),
                }
            }
            (Message::ToastTtlExpired(k), _, _) => {
                self.toast.remove(&k);
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let page_content = match &self.page {
            ActivePage::Loading => Element::from(
                container(CupertinoSpinner::new().width(40.into()).height(40.into()))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            ),
            ActivePage::Room(room) => room.view(&self.theme).map(Message::RoomEvent),
            ActivePage::Omni(omni) => omni.view().map(Message::OmniEvent),
        };

        let mut content = Element::from(page_content);

        for (i, (idx, toast)) in self.toast.iter().enumerate() {
            let offs = f32::from(u8::try_from(i).unwrap_or(u8::MAX));

            content = FloatingElement::new(
                content,
                ToastElement::new(toast).on_expiry(Message::ToastTtlExpired(*idx)),
            )
            .anchor(Anchor::SouthEast)
            .offset([20.0, 20.0 + (80.0 * offs)])
            .into();
        }

        if let Some(context_menu) = &self.context_menu {
            let context_menu = match context_menu {
                ActiveContextMenu::LightControl(menu) => menu.view().map(Message::LightControlMenu),
            };

            ContextMenu::new(content, context_menu)
                .on_close(Message::CloseContextMenu)
                .into()
        } else {
            content
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
    Loaded((Arc<Oracle>, Config)),
    CloseContextMenu,
    OpenOmniPage,
    OpenHomePage,
    OmniEvent(pages::omni::Message),
    RoomEvent(pages::room::Message),
    LightControlMenu(context_menus::light_control::Message),
    UpdateLightResult(()),
    PlayTrackResult(()),
    ToastTtlExpired(u8),
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
            min_size: Some(Size::new(600.0, 600.0)),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
    .unwrap();
}
