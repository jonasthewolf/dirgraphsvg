pub mod edges;
mod graph;
pub mod nodes;
mod util;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use edges::EdgeType;
use graph::{get_forced_levels, rank_nodes, NodePlace};
use nodes::{Node, Port};
use rusttype::Font;
use svg::{
    node::element::{path::Data, Link, Marker, Path, Polyline},
    Document,
};
use util::{
    font::{get_default_font, get_font},
    point2d::Point2D,
};

const MARKER_HEIGHT: u32 = 10;

pub struct Margin {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 20,
            right: 20,
            bottom: 20,
            left: 20,
        }
    }
}

pub struct FontInfo {
    font: Font<'static>,
    name: String,
    size: f32,
}

pub struct DirGraph<'a> {
    width: u32,
    height: u32,
    margin: Margin,
    wrap: u32,
    font: FontInfo,
    css_stylesheets: Vec<&'a str>,
    forced_levels: BTreeMap<&'a str, Vec<&'a str>>,
    nodes: BTreeMap<String, Rc<RefCell<dyn Node>>>,
    edges: BTreeMap<String, Vec<(String, EdgeType)>>,
    document: Document,
}

impl<'a> Default for DirGraph<'a> {
    fn default() -> Self {
        Self {
            width: 210,
            height: 297,
            margin: Margin::default(),
            wrap: 40,
            font: FontInfo {
                font: get_default_font().unwrap(),
                name: util::font::DEFAULT_FONT_FAMILY_NAME.to_owned(),
                size: 12.0,
            },
            css_stylesheets: Vec::new(),
            forced_levels: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            document: Document::new(),
        }
    }
}

impl<'a> DirGraph<'a> {
    pub fn set_wrap(mut self, wrap: u32) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn set_margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    pub fn set_font(mut self, font: &str, size: f32) -> Self {
        self.font = FontInfo {
            font: get_font(font).unwrap(),
            name: font.to_owned(),
            size,
        };
        self
    }

    pub fn add_css_sytlesheet(mut self, css: &'a str) -> Self {
        self.css_stylesheets.push(css);
        self
    }

    pub fn add_nodes(mut self, mut nodes: BTreeMap<String, Rc<RefCell<dyn Node>>>) -> Self {
        self.nodes.append(&mut nodes);
        self
    }

    pub fn add_node(mut self, node: Rc<RefCell<dyn Node>>) -> Self {
        self.nodes
            .insert(node.borrow().get_id().to_owned(), node.clone());
        self
    }

    pub fn add_edge(
        mut self,
        source: Rc<RefCell<dyn Node>>,
        target: Rc<RefCell<dyn Node>>,
        edge_type: EdgeType,
    ) -> Self {
        let entry = self
            .edges
            .entry(source.borrow().get_id().to_owned())
            .or_default();
        entry.push((target.borrow().get_id().to_owned(), edge_type));
        self
    }

    pub fn add_edges(mut self, edges: &mut BTreeMap<String, Vec<(String, EdgeType)>>) -> Self {
        self.edges.append(edges);
        self
    }

    pub fn add_levels(mut self, levels: &BTreeMap<&'a str, Vec<&'a str>>) -> Self {
        for (level, nodes) in levels {
            self.forced_levels.insert(level, nodes.to_vec());
        }
        self
    }

    pub fn write(mut self, output: impl std::io::Write) -> Result<(), std::io::Error> {
        self = self.setup_basics();
        self = self.setup_stylesheets();
        self = self.layout();
        self.document = self
            .document
            .set("viewBox", (0u32, 0u32, self.width, self.height));
        svg::write(output, &self.document)?;
        Ok(())
    }

    ///
    /// Layout the graph on a pseudo-stack layout
    ///
    /// 1) Let each element calculate its size
    /// 2) Calculate forced levels
    /// 3) Rank the nodes
    /// 4) Position nodes initially
    /// 5) Center nodes and draw them
    /// 6) Draw the edges
    ///
    ///
    fn layout(mut self) -> Self {
        // Calculate node size
        self.nodes
            .values()
            .for_each(|n| n.borrow_mut().calculate_size(&self.font, self.wrap));

        // Create forced levels
        let forced_levels = get_forced_levels(&self.nodes, &self.edges, &self.forced_levels);
        for (node, level) in forced_levels {
            self.nodes
                .get(&*node)
                .unwrap()
                .borrow_mut()
                .set_forced_level(level);
        }

        // Rank nodes
        let ranks = rank_nodes(&mut self.nodes, &mut self.edges);

        self.width = 0;
        self.height = 0;

        // Position nodes
        let mut x = self.margin.left;
        let mut y = self.margin.top;
        for rank in ranks.values() {
            let height_max = rank
                .values()
                .map(|id| match id {
                    NodePlace::Node(id) => self.nodes.get(id).unwrap().borrow().get_height(),
                    NodePlace::MultipleNodes(ids) => {
                        ids.iter()
                            .map(|id| self.nodes.get(id).unwrap().borrow().get_height())
                            .sum::<u32>()
                            + (self.margin.top + self.margin.bottom) * (ids.len() - 1) as u32
                    }
                })
                .max()
                .unwrap();
            for np in rank.values() {
                match np {
                    NodePlace::Node(id) => {
                        let mut n = self.nodes.get(id).unwrap().borrow_mut();
                        x += n.get_width() / 2;
                        n.set_position(&Point2D {
                            x,
                            y: y + height_max / 2,
                        });
                        x += n.get_width() / 2 + self.margin.left + self.margin.right;
                    }
                    NodePlace::MultipleNodes(ids) => {
                        let x_max = ids
                            .iter()
                            .map(|id| self.nodes.get(id).unwrap().borrow().get_width())
                            .max()
                            .unwrap();
                        let mut y_n = y;
                        for id in ids {
                            let mut n = self.nodes.get(id).unwrap().borrow_mut();
                            let n_height = n.get_height();
                            n.set_position(&Point2D {
                                x: x + x_max / 2,
                                y: y_n + n_height / 2,
                            });
                            y_n += n_height + self.margin.top + self.margin.bottom;
                        }
                        x += x_max;
                    }
                }
            }
            self.width = std::cmp::max(self.width, x);
            x = self.margin.left;
            y += height_max + self.margin.top + self.margin.bottom;
        }
        self.height = y + self.margin.bottom;

        // Center nodes and draw them
        for rank in ranks.values() {
            let last_node_place = rank.iter().last().unwrap().1;
            let delta_x = (self.width
                - self.margin.left
                - self.margin.right
                - (last_node_place.get_x(&self.nodes)
                    + last_node_place.get_max_width(&self.nodes)))
                / 2;
            for np in rank.values() {
                match np {
                    NodePlace::Node(id) => {
                        let mut n = self.nodes.get(id).unwrap().borrow_mut();
                        let cur_pos = n.get_position();
                        n.set_position(&Point2D {
                            x: cur_pos.x + delta_x,
                            y: cur_pos.y,
                        });
                        self.document = self.document.add(n.render(&self.font));
                    }
                    NodePlace::MultipleNodes(ids) => {
                        for id in ids {
                            let mut n = self.nodes.get(id).unwrap().borrow_mut();
                            let cur_pos = n.get_position();
                            n.set_position(&Point2D {
                                x: cur_pos.x + delta_x,
                                y: cur_pos.y,
                            });
                            self.document = self.document.add(n.render(&self.font));
                        }
                    }
                }
            }
        }

        // Draw edges
        self.render_edges()
    }

    ///
    ///
    ///
    ///
    ///
    ///
    ///
    fn render_edges(mut self) -> Self {
        for (source, targets) in &self.edges {
            for (target, edge_type) in targets {
                let s = self.nodes.get(source).unwrap().borrow();
                let t = self.nodes.get(target).unwrap().borrow();
                let (marker_height, support_distance) = match edge_type {
                    EdgeType::InContextOf => (MARKER_HEIGHT, 3 * MARKER_HEIGHT),
                    EdgeType::SupportedBy => (MARKER_HEIGHT, 3 * MARKER_HEIGHT),
                    EdgeType::Invisible => (0, 3 * MARKER_HEIGHT),
                };
                let s_pos = s.get_position();
                let t_pos = t.get_position();
                let (start, start_sup, end, end_sup) =
                    if s_pos.y + s.get_height() / 2 < t_pos.y - t.get_height() / 2 {
                        (
                            s.get_coordinates(&Port::South),
                            s.get_coordinates(&Port::South)
                                .move_relative(0, support_distance as i32),
                            t.get_coordinates(&Port::North)
                                .move_relative(0, -(marker_height as i32)),
                            t.get_coordinates(&Port::North)
                                .move_relative(0, -(support_distance as i32)),
                        )
                    } else if s_pos.y - s.get_height() / 2 - self.margin.top
                        > t_pos.y + t.get_height() / 2
                    {
                        (
                            s.get_coordinates(&Port::North),
                            s.get_coordinates(&Port::North)
                                .move_relative(0, -(support_distance as i32)),
                            t.get_coordinates(&Port::South)
                                .move_relative(0, marker_height as i32),
                            t.get_coordinates(&Port::South)
                                .move_relative(0, support_distance as i32),
                        )
                    } else {
                        // s.get_vertical_rank() == t.get_vertical_rank()
                        if s_pos.x - s.get_width() / 2 > t_pos.x + t.get_width() / 2 {
                            (
                                s.get_coordinates(&Port::West),
                                s.get_coordinates(&Port::West),
                                t.get_coordinates(&Port::East)
                                    .move_relative(marker_height as i32, 0),
                                t.get_coordinates(&Port::East)
                                    .move_relative(support_distance as i32, 0),
                            )
                        } else {
                            (
                                s.get_coordinates(&Port::East),
                                s.get_coordinates(&Port::East),
                                t.get_coordinates(&Port::West)
                                    .move_relative(-(marker_height as i32), 0),
                                t.get_coordinates(&Port::West)
                                    .move_relative(-(support_distance as i32), 0),
                            )
                        }
                    };
                let parameters = (start_sup.x, start_sup.y, end_sup.x, end_sup.y, end.x, end.y);
                let data = Data::new()
                    .move_to((start.x, start.y))
                    .cubic_curve_to(parameters);
                let arrow_id = match edge_type {
                    edges::EdgeType::InContextOf => Some("url(#incontextof_arrow)"),
                    edges::EdgeType::SupportedBy => Some("url(#supportedby_arrow)"),
                    edges::EdgeType::Invisible => None,
                };
                let classes = match edge_type {
                    edges::EdgeType::InContextOf => Some("gsnedge gsnspby"),
                    edges::EdgeType::SupportedBy => Some("gsnedge gsninctxt"),
                    edges::EdgeType::Invisible => None,
                };
                let mut e = Path::new()
                    .set("d", data)
                    .set("fill", "none")
                    .set("stroke", "black")
                    .set("stroke-width", 1u32);
                if let Some(arrow_id) = arrow_id {
                    e = e.set("marker-end", arrow_id);
                }
                if let Some(classes) = classes {
                    e = e.set("class", classes);
                }
                self.document = self.document.add(e);
            }
        }
        self
    }

    fn setup_basics(mut self) -> Self {
        let supportedby_polyline = Polyline::new()
            .set("points", "0 0, 10 4.5, 0 9")
            .set("fill", "black");
        let supportedby_arrow = Marker::new()
            .set("id", "supportedby_arrow")
            .set("markerWidth", 10u32)
            .set("markerHeight", 9u32)
            .set("refX", 0f32)
            .set("refY", 4.5f32)
            .set("orient", "auto")
            .set("markerUnits", "users_posaceOnUse")
            .add(supportedby_polyline);

        let incontext_polyline = Polyline::new()
            .set("points", "0 0, 10 4.5, 0 9, 0 0")
            .set("stroke", "black")
            .set("stroke-width", 1u32)
            .set("fill", "none");
        let incontext_arrow = Marker::new()
            .set("id", "incontextof_arrow")
            .set("markerWidth", 10u32)
            .set("markerHeight", 9u32)
            .set("refX", 0f32)
            .set("refY", 4.5f32)
            .set("orient", "auto")
            .set("markerUnits", "users_posaceOnUse")
            .add(incontext_polyline);

        self.document = self
            .document
            .set("xmlns:xlink", "http://www.w3.org/1999/xlink");
        self.document = self.document.add(supportedby_arrow).add(incontext_arrow);
        self
    }

    fn setup_stylesheets(mut self) -> Self {
        for css in &self.css_stylesheets {
            let l = Link::default()
                .set("rel", "stylesheet")
                .set("href", *css)
                .set("type", "text/css");
            self.document = self.document.add(l);
        }
        self
    }
}
