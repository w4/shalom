use iced::{
    advanced::{
        image::{Data, Renderer as ImageRenderer},
        layout::{Limits, Node},
        overlay,
        renderer::{Quad, Style},
        widget::Tree,
        Clipboard, Layout, Renderer as IRenderer, Shell, Widget,
    },
    event::Status,
    font::{Stretch, Weight},
    gradient::Linear,
    mouse,
    mouse::{Button, Cursor},
    theme::Text,
    touch,
    widget::{image, text},
    Alignment, Background, Color, ContentFit, Degrees, Element, Event, Font, Gradient, Length,
    Point, Rectangle, Renderer, Size, Theme, Vector,
};

pub fn image_card<'a, M: 'a>(
    handle: impl Into<image::Handle>,
    caption: &'a str,
) -> ImageCard<'a, M> {
    let image_handle = handle.into();

    ImageCard {
        image_handle: image_handle.clone(),
        text: text(caption)
            .size(14)
            .font(Font {
                weight: Weight::Bold,
                stretch: Stretch::Condensed,
                ..Font::with_name("Helvetica Neue")
            })
            .style(Text::Color(Color::WHITE))
            .into(),
        on_press: None,
        width: Length::FillPortion(1),
        height: Length::Fixed(128.0),
    }
}

pub struct ImageCard<'a, M> {
    image_handle: image::Handle,
    text: Element<'a, M, Renderer>,
    on_press: Option<M>,
    width: Length,
    height: Length,
}

impl<'a, M> ImageCard<'a, M> {
    pub fn on_press(mut self, msg: M) -> Self {
        self.on_press = Some(msg);
        self
    }
}

impl<'a, M: Clone> Widget<M, Renderer> for ImageCard<'a, M> {
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, _renderer: &Renderer, limits: &Limits) -> Node {
        let limits = limits.width(self.width).height(self.height);
        let size = limits.resolve(Size::ZERO);

        Node::new(size)
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        // The raw w/h of the underlying image, renderer.dimensions is _really_
        // slow so enforce the use of preparsed images from `theme`.
        #[allow(clippy::cast_precision_loss)]
        let image_size = match self.image_handle.data() {
            Data::Rgba { width, height, .. } => Size::new(*width as f32, *height as f32),
            Data::Path(_) | Data::Bytes(_) => panic!("only parsed images are supported"),
        };

        let adjusted_fit = ContentFit::Cover.fit(image_size, bounds.size());

        renderer.with_layer(bounds, |renderer| {
            let offset = Vector::new(
                (bounds.width - adjusted_fit.width).min(0.0) / 1.5,
                (bounds.height - adjusted_fit.height).min(0.0) / 1.5,
            );

            let drawing_bounds = Rectangle {
                width: adjusted_fit.width,
                height: adjusted_fit.height,
                ..bounds
            };

            renderer.draw(self.image_handle.clone(), drawing_bounds + offset);
        });
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.text)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.text]);
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
    ) -> Option<iced::advanced::overlay::Element<'b, M, Renderer>> {
        Some(overlay::Element::new(
            layout.position(),
            Box::new(Overlay {
                text: &mut self.text,
                tree: &mut state.children[0],
                size: layout.bounds().size(),
                on_press: self.on_press.as_ref(),
            }),
        ))
    }
}

struct Overlay<'a, 'b, M> {
    text: &'b mut Element<'a, M, Renderer>,
    tree: &'b mut Tree,
    size: Size,
    on_press: Option<&'b M>,
}

impl<'a, 'b, M: Clone> overlay::Overlay<M, Renderer> for Overlay<'a, 'b, M> {
    fn layout(&self, renderer: &Renderer, _bounds: Size, position: Point) -> Node {
        let limits = Limits::new(Size::ZERO, self.size).pad([0, 0, 10, 0].into());

        let mut child = self.text.as_widget().layout(renderer, &limits);
        child.align(Alignment::Center, Alignment::End, limits.max());

        let mut node = Node::with_children(self.size, vec![child]);
        node.move_to(position);

        node
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    height: self.size.height + 12.,
                    width: self.size.width + 12.,
                    x: layout.bounds().x - 6.,
                    y: layout.bounds().y - 6.,
                },
                border_radius: [20., 20., 20., 20.].into(),
                border_width: 10.,
                border_color: Color::WHITE,
            },
            Background::Gradient(Gradient::Linear(
                Linear::new(Degrees(270.))
                    .add_stop(0.0, Color::from_rgba8(0, 0, 0, 0.0))
                    .add_stop(0.4, Color::from_rgba8(0, 0, 0, 0.0))
                    .add_stop(1.0, Color::from_rgba8(0, 0, 0, 0.8)),
            )),
        );

        self.text.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
            &layout.bounds(),
        );
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
    ) -> Status {
        if !cursor.is_over(Rectangle {
            height: self.size.height,
            width: self.size.width,
            ..layout.bounds()
        }) {
            return Status::Ignored;
        }

        if let Some(on_press) = self.on_press {
            if let Event::Mouse(mouse::Event::ButtonPressed(Button::Left))
            | Event::Touch(touch::Event::FingerPressed { .. }) = &event
            {
                shell.publish(on_press.clone());

                return Status::Captured;
            }
        }

        Status::Ignored
    }
}

impl<'a, M> From<ImageCard<'a, M>> for Element<'a, M>
where
    M: 'a + Clone,
{
    fn from(modal: ImageCard<'a, M>) -> Self {
        Element::new(modal)
    }
}
