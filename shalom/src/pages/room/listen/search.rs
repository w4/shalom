use std::fmt::{Display, Formatter};

use iced::{
    alignment::Horizontal,
    theme,
    widget::{
        column, component, container, container::Appearance, image, image::Handle, mouse_area, row,
        text, Column, Component,
    },
    Alignment, Background, Color, Element, Length, Renderer, Theme,
};

use crate::widgets::spinner::CupertinoSpinner;

pub fn search<M: Clone + 'static>(theme: Theme, results: SearchState<'_>) -> Search<'_, M> {
    Search {
        on_track_press: None,
        theme,
        results,
    }
}

pub struct Search<'a, M> {
    on_track_press: Option<fn(String) -> M>,
    theme: Theme,
    results: SearchState<'a>,
}

impl<M> Search<'_, M> {
    pub fn on_track_press(mut self, f: fn(String) -> M) -> Self {
        self.on_track_press = Some(f);
        self
    }
}

impl<M: Clone + 'static> Component<M, Renderer> for Search<'_, M> {
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            Event::OnTrackPress(id) => {
                state.pressing = None;
                self.on_track_press.map(|f| (f)(id))
            }
            Event::OnDown(i) => {
                state.pressing = Some(i);
                None
            }
            Event::OnCancel => {
                state.pressing = None;
                None
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        match self.results {
            SearchState::Ready(results) if !results.is_empty() => {
                let mut col = Column::new();

                for (i, result) in results.iter().enumerate() {
                    let pressing = state.pressing == Some(i);

                    let track = mouse_area(search_item_container(
                        result_card(result, &self.theme),
                        pressing,
                    ))
                    .on_press(Event::OnDown(i))
                    .on_release(Event::OnTrackPress(result.uri.to_string()))
                    .on_cancel(Event::OnCancel);

                    col = col.push(track);
                }

                Element::from(col.spacing(10))
            }
            SearchState::Ready(_) => Element::from(search_item_container(
                container(text("No results found"))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
                false,
            )),
            SearchState::Error(error) => Element::from(search_item_container(
                container(text(error))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
                false,
            )),
            SearchState::NotReady => Element::from(search_item_container(
                container(CupertinoSpinner::new().width(40.into()).height(40.into()))
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
                false,
            )),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct State {
    pressing: Option<usize>,
}

impl<'a, M: 'static + Clone> From<Search<'a, M>> for Element<'a, M, Renderer> {
    fn from(value: Search<'a, M>) -> Self {
        component(value)
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
pub enum Event {
    OnTrackPress(String),
    OnDown(usize),
    OnCancel,
}

fn result_card<M: 'static>(result: &SearchResult, _style: &Theme) -> Element<'static, M, Renderer> {
    let main_text = text(&result.title);
    let sub_text = text(&result.metadata).style(Color {
        a: 0.7,
        ..Color::BLACK
    });

    row![
        image(result.image.clone()).width(64).height(64),
        column![main_text, sub_text,]
    ]
    .align_items(Alignment::Center)
    .spacing(10)
    .into()
}

fn search_item_container<'a, M: 'a>(
    elem: impl Into<Element<'a, M, Renderer>>,
    pressing: bool,
) -> Element<'a, M, Renderer> {
    container(elem)
        .padding([20, 20, 20, 20])
        .style(theme::Container::Custom(Box::new(SearchItemContainer(
            pressing,
        ))))
        .width(Length::Fill)
        .into()
}

#[allow(clippy::module_name_repetitions)]
pub struct SearchItemContainer(bool);

impl container::StyleSheet for SearchItemContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> Appearance {
        let base = Appearance {
            text_color: Some(Color {
                a: 0.9,
                ..Color::BLACK
            }),
            background: None,
            border_radius: 20.0.into(),
            border_width: 0.0,
            border_color: Color::default(),
        };

        if self.0 {
            Appearance {
                background: Some(Background::Color(Color {
                    a: 0.9,
                    ..Color::WHITE
                })),
                ..base
            }
        } else {
            Appearance {
                background: Some(Background::Color(Color {
                    a: 0.8,
                    ..Color::WHITE
                })),
                ..base
            }
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub enum SearchState<'a> {
    NotReady,
    Ready(&'a [SearchResult]),
    Error(&'a str),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Hash)]
pub struct SearchResult {
    image: Handle,
    title: String,
    uri: String,
    metadata: ResultMetadata,
}

impl SearchResult {
    pub fn track(image: Handle, title: String, artist: String, uri: String) -> Self {
        Self {
            image,
            title,
            uri,
            metadata: ResultMetadata::Track(artist),
        }
    }

    pub fn playlist(image: Handle, title: String, uri: String) -> Self {
        Self {
            image,
            title,
            uri,
            metadata: ResultMetadata::Playlist,
        }
    }

    pub fn artist(image: Handle, title: String, uri: String) -> Self {
        Self {
            image,
            title,
            uri,
            metadata: ResultMetadata::Artist,
        }
    }

    pub fn album(image: Handle, title: String, uri: String) -> Self {
        Self {
            image,
            title,
            uri,
            metadata: ResultMetadata::Album,
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub enum ResultMetadata {
    Track(String),
    Playlist,
    Album,
    Artist,
}

impl Display for ResultMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultMetadata::Track(v) => write!(f, "Track • {v}"),
            ResultMetadata::Playlist => write!(f, "Playlist"),
            ResultMetadata::Album => write!(f, "Album"),
            ResultMetadata::Artist => write!(f, "Artist"),
        }
    }
}
