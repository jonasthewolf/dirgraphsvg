use svg::node::element::{path::Data, Group, Link, Path, Text, Title};

use crate::FontInfo;

use super::{get_port_default_coordinates, Node, Point2D};

const PADDING: u32 = 5;
const TEXT_OFFSET: u32 = 20;

#[derive(Clone)]
pub struct ContextNode {
    identifier: String,
    text: String,
    url: Option<String>,
    classes: Option<Vec<String>>,
    width: u32,
    height: u32,
    lines: Vec<(u32, u32)>,
    x: u32,
    y: u32,
}

impl Node for ContextNode {
    ///
    /// Width: 5 padding on each side, minimum 50, maximum line length of text or identifier
    /// Height: 5 padding on each side, minimum 30, id line height (max. 20) + height of each text line
    ///
    fn calculate_size(&mut self, font: &FontInfo, suggested_char_wrap: u32) {
        self.width = PADDING * 2 + 50; // Padding of 5 on both sides
        self.height = PADDING * 2 + 30; // Padding of 5 on both sides
        self.text = crate::util::wordwrap::wordwrap(&self.text, suggested_char_wrap, "\n");
        let (t_width, t_height) =
            crate::util::font::text_bounding_box(&font.font, &self.identifier, font.size);
        self.lines.push((t_width, t_height));
        let mut text_height = 0;
        let mut text_width = t_width + PADDING * 2;
        for t in self.text.lines() {
            let (width, height) = crate::util::font::text_bounding_box(&font.font, t, font.size);
            self.lines.push((width, height));
            text_height += height;
            text_width = std::cmp::max(text_width, width + PADDING * 2);
        }
        self.width = std::cmp::max(self.width, text_width) + 20;
        self.height = std::cmp::max(self.height, PADDING * 2 + TEXT_OFFSET + text_height + 3);
        // +3 to make padding at bottom larger
    }

    fn set_position(&mut self, pos: &Point2D) {
        self.x = pos.x;
        self.y = pos.y;
    }

    fn get_position(&self) -> Point2D {
        Point2D {
            x: self.x,
            y: self.y,
        }
    }

    fn get_id(&self) -> &str {
        self.identifier.as_ref()
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn get_height(&self) -> u32 {
        self.height
    }

    fn get_coordinates(&self, port: &super::Port) -> Point2D {
        get_port_default_coordinates(self.x, self.y, self.width, self.height, port)
    }

    fn render(&mut self, font: &FontInfo) -> svg::node::element::Group {
        // TODO escape id
        let mut g = Group::new().set("id", format!("node_{}", self.get_id()));
        if let Some(classes) = &self.classes {
            g = g.set("class", classes.join(" "))
        }
        if let Some(url) = &self.url {
            let link = Link::new();
            g = g.add(link.set("xlink:href", url.as_str()));
        }

        let title = Title::new().add(svg::node::Text::new(&self.identifier));

        let data = Data::new()
            .move_to((self.x + 10 - self.width / 2, self.y - self.height / 2))
            .line_to((self.x - 10 + self.width / 2, self.y - self.height / 2))
            .cubic_curve_to((
                self.x + self.width / 2,
                self.y - self.height / 2,
                self.x + self.width / 2,
                self.y + self.height / 2,
                self.x + self.width / 2 - 10,
                self.y + self.height / 2,
            ))
            .line_to((self.x - self.width / 2 + 10, self.y + self.height / 2))
            .cubic_curve_to((
                self.x - self.width / 2,
                self.y + self.height / 2,
                self.x - self.width / 2,
                self.y - self.height / 2,
                self.x - self.width / 2 + 10,
                self.y - self.height / 2,
            ));

        let border = Path::new()
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", 1u32)
            .set("d", data)
            .set("class", "border");

        let id = Text::new()
            .set("x", self.x - self.width / 2 + PADDING + 5)
            .set(
                "y",
                self.y - self.height / 2 + PADDING + self.lines.get(0).unwrap().1,
            )
            .set("font-weight", "bold")
            .set("font-size", font.size)
            .set("font-family", font.name.as_str())
            .add(svg::node::Text::new(&self.identifier));

        g = g.add(title).add(border).add(id);

        for (n, t) in self.text.lines().enumerate() {
            let text = Text::new()
                .set("x", self.x - self.width / 2 + PADDING + 5)
                .set(
                    "y",
                    self.y - self.height / 2
                        + PADDING
                        + TEXT_OFFSET
                        + (n as u32 + 1) * self.lines.get(n + 1).unwrap().1,
                )
                .set("textLength", self.lines.get(n + 1).unwrap().0)
                .set("font-size", font.size)
                .set("font-family", font.name.as_str())
                .add(svg::node::Text::new(t));
            g = g.add(text);
        }
        g
    }

    fn get_forced_level(&self) -> Option<usize> {
        None
    }

    fn set_forced_level(&mut self, _: usize) {}
}

impl ContextNode {
    pub fn new(id: &str, text: &str, url: Option<String>, classes: Option<Vec<String>>) -> Self {
        ContextNode {
            identifier: id.to_owned(),
            text: text.to_owned(),
            url,
            classes,
            width: 0,
            height: 0,
            lines: vec![],
            x: 0,
            y: 0,
        }
    }
}
