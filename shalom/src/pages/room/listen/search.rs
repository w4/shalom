use std::fmt::{Display, Formatter};

use iced::{
    theme,
    widget::{
        column, component, container, container::Appearance, horizontal_rule, image, image::Handle,
        row, scrollable, text, Column, Component,
    },
    Alignment, Background, Color, Element, Length, Renderer, Theme,
};

use crate::widgets::mouse_area::mouse_area;

pub fn search<M: Clone + 'static>(theme: Theme, results: Vec<SearchResult>) -> Search<M> {
    Search {
        on_track_press: None,
        theme,
        results,
    }
}

pub struct Search<M> {
    on_track_press: Option<fn(String) -> M>,
    theme: Theme,
    results: Vec<SearchResult>,
}

impl<M: Clone + 'static> Component<M, Renderer> for Search<M> {
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            Event::OnTrackPress(id) => self.on_track_press.map(|f| (f)(id)),
        }
    }

    fn view(&self, _state: &Self::State) -> Element<'_, Self::Event, Renderer> {
        let mut col = Column::new();

        for (i, result) in self.results.iter().enumerate() {
            if i != 0 {
                col = col.push(hr());
            }

            let track = mouse_area(search_item_container(result_card(result, &self.theme)))
                .on_press(Event::OnTrackPress("hello world".to_string()));

            col = col.push(track);
        }

        search_container(scrollable(col.spacing(10)))
    }
}

impl<M: 'static + Clone> From<Search<M>> for Element<'static, M, Renderer> {
    fn from(value: Search<M>) -> Self {
        component(value)
    }
}

#[derive(Clone)]
pub enum Event {
    OnTrackPress(String),
}

fn result_card<M: 'static>(result: &SearchResult, style: &Theme) -> Element<'static, M, Renderer> {
    let main_text = text(&result.title).style(style.extended_palette().background.base.text);
    let sub_text = text(&result.metadata).style(style.extended_palette().background.strong.color);

    row![
        image(result.image.clone()).width(64).height(64),
        column![main_text, sub_text,]
    ]
    .align_items(Alignment::Center)
    .spacing(10)
    .into()
}

fn hr<M: 'static>() -> Element<'static, M, Renderer> {
    container(horizontal_rule(1))
        .width(Length::Fill)
        .padding([10, 0, 10, 0])
        .into()
}

fn search_item_container<'a, M: 'a>(
    elem: impl Into<Element<'a, M, Renderer>>,
) -> Element<'a, M, Renderer> {
    container(elem).padding([0, 20, 0, 20]).into()
}

fn search_container<'a, M: 'a>(
    elem: impl Into<Element<'a, M, Renderer>>,
) -> Element<'a, M, Renderer> {
    container(elem)
        .padding([20, 0, 20, 0])
        .width(Length::Fill)
        .style(theme::Container::Custom(Box::new(SearchContainer)))
        .into()
}

#[allow(clippy::module_name_repetitions)]
pub struct SearchContainer;

impl container::StyleSheet for SearchContainer {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> Appearance {
        Appearance {
            text_color: Some(Color::BLACK),
            background: Some(Background::Color(Color::WHITE)),
            border_radius: 20.0.into(),
            border_width: 0.0,
            border_color: Color::default(),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct SearchResult {
    image: Handle,
    title: String,
    metadata: ResultMetadata,
}

impl SearchResult {
    pub fn track(image: Handle, title: String, artist: String) -> Self {
        Self {
            image,
            title,
            metadata: ResultMetadata::Track(artist),
        }
    }

    pub fn playlist(image: Handle, title: String) -> Self {
        Self {
            image,
            title,
            metadata: ResultMetadata::Playlist,
        }
    }

    pub fn album(image: Handle, title: String) -> Self {
        Self {
            image,
            title,
            metadata: ResultMetadata::Album,
        }
    }
}

pub enum ResultMetadata {
    Track(String),
    Playlist,
    Album,
}

impl Display for ResultMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResultMetadata::Track(v) => write!(f, "Track • {v}"),
            ResultMetadata::Playlist => write!(f, "Playlist"),
            ResultMetadata::Album => write!(f, "Album"),
        }
    }
}
