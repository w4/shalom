use iced::{
    theme,
    widget::{
        column, container, container::Appearance, horizontal_rule, image, image::Handle, row,
        scrollable, text, Column,
    },
    Alignment, Background, Color, Element, Length, Renderer, Theme,
};

use crate::{theme::Image, widgets::mouse_area::mouse_area};

pub fn search<M: Clone + 'static>() -> Search<M> {
    Search {
        on_track_press: None,
    }
}

pub struct Search<M> {
    on_track_press: Option<fn(String) -> M>,
}

impl<M: Clone + 'static> Search<M> {
    pub fn view(&self, style: &Theme) -> Element<'static, M, Renderer> {
        let mut col = Column::new();

        for i in 0..20 {
            if i != 0 {
                col = col.push(hr());
            }

            let track = mouse_area(search_item_container(track_card(
                "title",
                "artist",
                Image::AlbumArtTest,
                style,
            )))
            .on_press(self.on_track_press.map(|f| (f)("hello world".to_string())));

            col = col.push(track);
        }

        search_container(scrollable(col.spacing(10)))
    }
}

fn track_card<M: 'static>(
    title: &str,
    artist: &str,
    image_handle: impl Into<Handle>,
    style: &Theme,
) -> Element<'static, M, Renderer> {
    let title = text(title).style(style.extended_palette().background.base.text);
    let artist = text(artist).style(style.extended_palette().background.strong.color);

    row![
        image(image_handle).width(64).height(64),
        column![title, artist,]
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
