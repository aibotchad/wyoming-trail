//! The state map: the picture, the roads, the stops and their labels, and the
//! vehicle part-way along its road.

use crate::core::map_graph::{point_along, MapBounds};
use crate::core::state::Screen;

use super::draw::{font, Column, Tooltip, ACCENT, BLOCK_GAP, DIM, MISSION};
use super::page::{Action, Hotspot, View};
use super::scene::Leg;
use super::App;

const ROAD: &str = "#7a5c3a";
const ROAD_OPEN: &str = "#c0392b";
const TRAVELLED: &str = "#2e8b57";
const DOT: &str = "#2b2118";
/// Pointer distance (px) that counts as touching a map dot.
const DOT_REACH: f64 = 11.0;
/// A fingertip needs a bigger target than a mouse pointer.
const DOT_REACH_TOUCH: f64 = 22.0;

/// Where the map sits on screen, and so where any place on it falls.
#[derive(Clone, Copy)]
struct Frame {
    bounds: MapBounds,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Frame {
    fn at(&self, lat: f64, lon: f64) -> (f64, f64) {
        let (fx, fy) = self.bounds.project(lat, lon);
        (self.x + fx * self.w, self.y + fy * self.h)
    }
}

/// What the run, if there is one, picks out on the map.
struct Highlights {
    in_run: bool,
    /// The stop the run last reached.
    here: Option<String>,
    /// Stops that can be reached next.
    next: Vec<String>,
    leg: Option<Leg>,
}

/// A stop's name waiting to be placed.
struct Label {
    name: String,
    x: f64,
    y: f64,
    important: bool,
}

/// The box a map label occupies.
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
}

impl App {
    pub(super) fn map(
        &mut self,
        col: &mut Column,
        view_height: f64,
        tooltip: &mut Option<Tooltip>,
    ) {
        let bounds = self.game.content.map.bounds;
        let max_h = (view_height - 90.0).max(260.0);
        let w = col.w.min(max_h * bounds.aspect());
        let h = w / bounds.aspect();
        col.y += 6.0;
        let frame = Frame {
            bounds,
            x: col.x + (col.w - w) / 2.0,
            y: col.y - self.scroll,
            w,
            h,
        };
        // Out of sight, it takes up the room and draws nothing.
        if self.on_screen(frame.y, h) {
            // A day's replay moves the vehicle on it.
            self.layout.animated |= self.drive.is_some();
            self.map_picture(frame);
            let marked = self.map_highlights();
            self.map_roads(frame, &marked);
            let labels = self.map_stops(frame, &marked, view_height, tooltip);
            if let Some(leg) = &marked.leg {
                self.map_vehicle(frame, leg);
            }
            self.map_labels(frame, labels);
            self.frame((frame.x, frame.y, w, h));
        }
        col.y += h + 13.0;
        self.map_credit(frame.x, col, w);
    }

    fn map_picture(&mut self, f: Frame) {
        match self.image("map.jpg") {
            Some(img) => {
                let _ = self
                    .ctx
                    .draw_image_with_html_image_element_and_dw_and_dh(&img, f.x, f.y, f.w, f.h);
            }
            None => {
                self.ctx.set_fill_style_str("#d8ccb0");
                self.ctx.fill_rect(f.x, f.y, f.w, f.h);
            }
        }
    }

    fn map_highlights(&self) -> Highlights {
        // On the title-screen map there is no run to highlight.
        let in_run = self.view == View::Game || self.history.contains(&View::Game);
        let leg = self.current_leg().filter(|_| in_run);
        let here = in_run.then(|| self.game.state.current_node.clone());
        let next = match (&here, &leg) {
            (Some(_), Some(l)) => vec![l.to.clone()],
            (Some(from), None) => {
                let roads = self.game.content.map.outgoing(from);
                roads.iter().map(|r| r.to.clone()).collect()
            }
            (None, _) => Vec::new(),
        };
        Highlights {
            in_run,
            here,
            next,
            leg,
        }
    }

    /// Trace a road's points as one line.
    fn polyline<'a>(&self, f: Frame, points: impl IntoIterator<Item = &'a [f64; 2]>) {
        self.ctx.begin_path();
        for (i, &[lon, lat]) in points.into_iter().enumerate() {
            let (x, y) = f.at(lat, lon);
            if i == 0 {
                self.ctx.move_to(x, y);
            } else {
                self.ctx.line_to(x, y);
            }
        }
        self.ctx.stroke();
    }

    fn map_roads(&self, f: Frame, marked: &Highlights) {
        for (key, path) in &self.game.content.road_paths {
            let Some((a, b)) = key.split_once('>') else {
                continue;
            };
            let active = match (&marked.here, &marked.leg) {
                (_, Some(l)) => (a == l.from && b == l.to) || (b == l.from && a == l.to),
                (Some(from), None) => a == from || b == from,
                _ => false,
            };
            self.ctx
                .set_stroke_style_str(if active { ROAD_OPEN } else { ROAD });
            self.ctx.set_line_width(if active { 3.5 } else { 1.6 });
            self.polyline(f, path);
        }
    }

    /// Draw the dots, make them tappable and note which one is pointed at.
    /// Returns the names to be placed once everything under them is drawn.
    fn map_stops(
        &mut self,
        f: Frame,
        marked: &Highlights,
        view_height: f64,
        tooltip: &mut Option<Tooltip>,
    ) -> Vec<Label> {
        let reach = if self.touch {
            DOT_REACH_TOUCH
        } else {
            DOT_REACH
        };
        let pointer = self.pointer.filter(|&(_, py)| py < view_height);
        let content = self.game.content.clone();
        let hovered = pointer.and_then(|(px, py)| {
            let distance = content.map.nodes.iter().map(|n| {
                let (x, y) = f.at(n.lat, n.lon);
                ((x - px).hypot(y - py), n.id.as_str())
            });
            distance
                .filter(|(d, _)| *d <= reach)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, id)| id)
        });
        let state = &self.game.state;
        let goals = marked.in_run && state.screen != Screen::Profession;
        let label_all = f.w >= 800.0;
        let mut labels = Vec::new();
        for n in &content.map.nodes {
            let (x, y) = f.at(n.lat, n.lon);
            let is_here = marked.here.as_deref() == Some(n.id.as_str()) && marked.leg.is_none();
            let is_next = marked.next.contains(&n.id);
            let is_goal = goals && self.game.targets().contains(&n.id);
            let seen = marked.in_run && self.game.state.visited.contains(&n.id);
            let radius = if is_here || is_goal { 7.5 } else { 4.5 };
            let fill = if is_here {
                ROAD_OPEN
            } else if is_goal {
                MISSION
            } else if is_next {
                ACCENT
            } else if seen {
                "#4f7d3a"
            } else {
                DOT
            };
            self.ctx.begin_path();
            let _ = self.ctx.arc(x, y, radius, 0.0, std::f64::consts::TAU);
            self.ctx.set_fill_style_str(fill);
            self.ctx.fill();
            self.ctx.set_line_width(1.5);
            self.ctx.set_stroke_style_str("#fff8e8");
            self.ctx.stroke();

            let important = is_here || is_next || is_goal;
            if label_all || important {
                let star = if is_goal { "★ " } else { "" };
                labels.push(Label {
                    name: format!("{star}{}", n.short_name()),
                    x,
                    y,
                    important,
                });
            }
            self.layout.hotspots.push(Hotspot {
                x: x - reach,
                y: y + self.scroll - reach,
                w: reach * 2.0,
                h: reach * 2.0,
                button: None,
                action: Action::ShowPlace(n.id.clone()),
            });
            if hovered == Some(n.id.as_str()) {
                *tooltip = Some(Tooltip {
                    id: n.id.clone(),
                    x,
                    y,
                });
            }
        }
        labels
    }

    /// The vehicle, part-way along its road, with the miles behind it.
    fn map_vehicle(&mut self, f: Frame, leg: &Leg) {
        let paths = &self.game.content.road_paths;
        // Each road's line is stored once, in one direction.
        let forward = paths.get(&format!("{}>{}", leg.from, leg.to)).cloned();
        let path = forward.or_else(|| {
            let back = paths.get(&format!("{}>{}", leg.to, leg.from))?;
            Some(back.iter().rev().copied().collect())
        });
        let Some(path) = path else {
            return;
        };
        let Some((here, end)) = point_along(&path, leg.done) else {
            return;
        };
        let (Some(done), Some(&ahead)) = (path.get(..end), path.get(end)) else {
            return;
        };
        self.ctx.set_stroke_style_str(TRAVELLED);
        self.ctx.set_line_width(4.5);
        self.polyline(f, done.iter().chain([&here]));
        // Face the way the current stretch of road runs.
        let behind = done.last().unwrap_or(&ahead);
        let west = ahead[0] < behind[0];
        let [lon, lat] = here;
        let (x, y) = f.at(lat, lon);
        let scale = if f.w < 500.0 { 0.75 } else { 1.0 };
        let frame = if self.drive.is_some() { leg.frame } else { 0 };
        let car = self.vehicle_sprite();
        // Tyres on the road.
        let top = y - self.feet(&car) * scale;
        self.sprite(&car, frame, x - 24.0 * scale, top, scale, west);
    }

    /// Write the names, each in the first place near its dot that is free.
    fn map_labels(&self, f: Frame, mut labels: Vec<Label>) {
        let size = if f.w < 500.0 { 15.0 } else { 19.0 };
        self.ctx.set_font(&font(size));
        // Important labels claim their spot first.
        labels.sort_by_key(|l| !l.important);
        let mut taken: Vec<Rect> = Vec::new();
        for label in labels {
            let tw = self
                .ctx
                .measure_text(&label.name)
                .map(|t| t.width())
                .unwrap_or(0.0);
            let spots = spots(label.x, label.y, tw + 4.0, size + 4.0);
            let free = spots.iter().find(|r| {
                r.x >= f.x && r.x + r.w <= f.x + f.w && !taken.iter().any(|t| t.overlaps(r))
            });
            // A crowded minor label is dropped; its dot still has a tooltip.
            let fallback = label.important.then_some(&spots[0]);
            let Some(r) = free.or(fallback).copied() else {
                continue;
            };
            taken.push(r);
            self.ctx.set_fill_style_str(if label.important {
                "rgba(232,176,74,0.95)"
            } else {
                "rgba(255,248,232,0.82)"
            });
            self.ctx.fill_rect(r.x, r.y, r.w, r.h);
            self.ctx.set_fill_style_str(DOT);
            let _ = self.ctx.fill_text(&label.name, r.x + 2.0, r.y + size);
        }
    }

    fn map_credit(&mut self, x: f64, col: &mut Column, w: f64) {
        self.ctx.set_font(&font(15.0));
        let credit = "Map: USGS The National Map · Roads © OpenStreetMap contributors";
        col.y += self.paragraph(credit, x, col.y, w, 15.0, DIM);
        col.y += BLOCK_GAP;
    }
}

/// Places a `bw`×`bh` label may sit beside a dot, best first: right, left,
/// above, below, the corners, then further off.
fn spots(x: f64, y: f64, bw: f64, bh: f64) -> [Rect; 10] {
    [
        (x + 6.0, y - bh / 2.0),
        (x - 6.0 - bw, y - bh / 2.0),
        (x - bw / 2.0, y - 6.0 - bh),
        (x - bw / 2.0, y + 6.0),
        (x + 5.0, y - 5.0 - bh),
        (x + 5.0, y + 5.0),
        (x - 5.0 - bw, y - 5.0 - bh),
        (x - 5.0 - bw, y + 5.0),
        (x - bw / 2.0, y - 8.0 - bh * 2.0),
        (x - bw / 2.0, y + 8.0 + bh),
    ]
    .map(|(left, top)| Rect {
        x: left,
        y: top,
        w: bw,
        h: bh,
    })
}
