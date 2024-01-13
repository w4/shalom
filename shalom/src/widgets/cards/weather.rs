use iced::{
    advanced::{
        graphics::text::Paragraph,
        layout::{Limits, Node},
        renderer::{Quad, Style},
        svg::Renderer as SvgRenderer,
        text::{LineHeight, Shaping},
        widget::{tree::Tag, Tree},
        Layout, Renderer as AdvancedRenderer, Widget,
    },
    alignment::{Horizontal, Vertical},
    font::Weight,
    gradient::Linear,
    mouse::Cursor,
    widget::{text, text::Appearance},
    Alignment, Background, Color, Degrees, Element, Font, Gradient, Length, Rectangle, Renderer,
    Size, Theme,
};
use time::OffsetDateTime;

use crate::oracle::Weather;

#[allow(clippy::module_name_repetitions)]
pub struct WeatherCard<M> {
    pub on_click: Option<M>,
    pub current_weather: Weather,
}

impl<M> WeatherCard<M> {
    pub fn new(current_weather: Weather) -> Self {
        Self {
            current_weather,
            on_click: None,
        }
    }

    fn build_temperature(&self) -> String {
        format!("{}°", self.current_weather.temperature)
    }

    fn build_conditions(&self) -> String {
        format!(
            "{}\nH:{}° L:{}°",
            self.current_weather.weather_condition(),
            self.current_weather.high,
            self.current_weather.low,
        )
    }
}

impl<M: Clone> Widget<M, Renderer> for WeatherCard<M> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(192.0), Length::Fixed(192.0))
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits
            .height(self.size().height)
            .width(self.size().width)
            .shrink([32, 32]);
        let container_size = limits.resolve(self.size().width, self.size().height, Size::ZERO);

        let local_state = tree.state.downcast_mut::<State>();
        let header_node = text::layout(
            &mut local_state.header,
            renderer,
            &limits,
            Length::Shrink,
            Length::Shrink,
            &self.build_temperature(),
            LineHeight::default(),
            Some(42.0.into()),
            Some(Font {
                weight: Weight::Normal,
                ..Font::with_name("Helvetica Neue")
            }),
            Horizontal::Left,
            Vertical::Top,
            Shaping::Basic,
        )
        .move_to([16., 16.])
        .align(Alignment::Start, Alignment::Start, container_size);

        let icon_node = Node::new(Size::new(16., 16.)).translate([16., -48.]).align(
            Alignment::Start,
            Alignment::End,
            container_size,
        );

        let conditions_node = text::layout(
            &mut local_state.conditions,
            renderer,
            &limits,
            Length::Shrink,
            Length::Shrink,
            &self.build_conditions(),
            LineHeight::default(),
            Some(12.0.into()),
            Some(Font {
                weight: Weight::Bold,
                ..Font::with_name("Helvetica Neue")
            }),
            Horizontal::Left,
            Vertical::Bottom,
            Shaping::Basic,
        )
        .move_to([16., -16.]);

        Node::with_children(
            container_size,
            vec![header_node, icon_node, conditions_node],
        )
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        viewport: &Rectangle,
    ) {
        // TODO: get sunrise/sunset from somewhere reasonable
        let day_time = matches!(OffsetDateTime::now_utc().hour(), 5..=19);
        let local_state = state.state.downcast_ref::<State>();

        let gradient = if day_time {
            Linear::new(Degrees(90.))
                .add_stop(0.0, Color::from_rgba8(104, 146, 190, 1.0))
                .add_stop(1.0, Color::from_rgba8(10, 54, 120, 1.0))
        } else {
            Linear::new(Degrees(90.))
                .add_stop(0.0, Color::from_rgba8(43, 44, 66, 1.0))
                .add_stop(1.0, Color::from_rgba8(15, 18, 27, 1.0))
        };

        renderer.fill_quad(
            Quad {
                bounds: layout.bounds(),
                border_radius: [20., 20., 20., 20.].into(),
                border_width: 0.,
                border_color: Color::WHITE,
            },
            Background::Gradient(Gradient::Linear(gradient)),
        );

        let mut children = layout.children();

        let header_layout = children.next().unwrap();
        text::draw(
            renderer,
            style,
            header_layout,
            &local_state.header,
            Appearance {
                color: Some(Color::WHITE),
            },
            viewport,
        );

        let icon_bounds = children.next().unwrap().bounds();
        if let Some(icon) = self.current_weather.weather_condition().icon(day_time) {
            renderer.draw(icon.handle(), None, icon_bounds);
        }

        let conditions_layout = children.next().unwrap();
        text::draw(
            renderer,
            style,
            conditions_layout,
            &local_state.conditions,
            Appearance {
                color: Some(Color::WHITE),
            },
            viewport,
        );
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(State::default())
    }

    fn tag(&self) -> Tag {
        Tag::of::<State>()
    }
}

#[derive(Default)]
pub struct State {
    header: text::State<Paragraph>,
    conditions: text::State<Paragraph>,
}

impl<'a, M> From<WeatherCard<M>> for Element<'a, M>
where
    M: 'a + Clone,
{
    fn from(modal: WeatherCard<M>) -> Self {
        Element::new(modal)
    }
}
