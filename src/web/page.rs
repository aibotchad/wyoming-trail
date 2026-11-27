//! Plain data describing what is on screen: a [`Page`] of blocks, and the
//! [`Layout`] rectangles `draw` produced for it (used for hit testing).

/// Which screen family is showing.
use crate::core::defs::FinaleDef;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Title,
    /// Whatever `GameState::screen` says.
    Game,
    /// Out poaching or foraging.
    Gather,
    Map,
    /// Photo and stories for one node.
    Place(String),
    Credits,
    /// Bar graphs of how the trip is going.
    Stats,
    /// The top ten.
    Scores,
    /// The pause menu, over a run in progress.
    Pause,
    /// How to play.
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    NewRun,
    Continue,
    PlayAgain,
    MainMenu,
    ShowMap,
    ShowPlace(String),
    ShowCredits,
    ShowStats,
    ShowScores,
    Pause,
    ShowHelp,
    /// Strike a drum at the finale.
    Drum,
    /// Camp this many nights running.
    Camp(u32),
    ClearScores,
    /// Pop up the figure behind one column of a graph.
    Reveal(String),
    Back,
    ChooseProfession(usize),
    Rename(usize),
    ConfirmParty,
    BuyTruck(usize),
    Buy(String),
    FillTank,
    OpenStore,
    LeaveStore,
    Rest(bool),
    /// Index into `MapGraph::roads`.
    ChooseRoad(usize),
    CyclePace,
    CycleMeal,
    DriveDay,
    /// End live driving where it stands.
    StopDriving,
    SkipDrive,
    /// Unchoose a road before any miles are driven.
    TurnBack,
    /// Previous question during setup.
    SetupBack,
    StartGather,
    /// Start the clock on a round that is waiting.
    BeginGather,
    /// Leave a waiting round; nothing is spent.
    CancelGather,
    SetFemale(bool),
    ConfirmYes,
    ConfirmNo,
    ChooseMonth(u32),
    PickUp(bool),
    OpenTrade,
    OpenWork,
    SellFood(u32),
    SellItem(String),
    Work(String),
    FinishGather,
    ContinueJourney,
}

/// How a paragraph is styled.
#[derive(Debug, Clone, Copy)]
pub enum Tone {
    /// The opening warning.
    Banner,
    Heading,
    Body,
    Dim,
    /// Section label above a paragraph.
    Label,
    /// Invented local colour.
    Joke,
    /// The current mission.
    Mission,
    /// A mission step already met.
    GoalMet,
    /// A mission step still to come.
    GoalLater,
    /// The title over a group of things.
    Section,
    Warn,
}

#[derive(Debug, Clone)]
pub enum Block {
    Text(String, Tone),
    /// A node photo, by node id.
    Photo(String),
    Map,
    /// The animated side view with its progress bars.
    Scene,
    /// The poaching or foraging field.
    Field,
    /// A sprite with a name beside it.
    Specimen {
        sprite: String,
        frame: u32,
        note: String,
    },
    /// A species: photo, field sprite and a note.
    Species {
        photo: String,
        sprite: String,
        note: String,
    },
    Button {
        label: String,
        detail: String,
        action: Action,
    },
    Gap,
    /// A button laid out as a card: a title, a line under it, then named
    /// sections.
    Card {
        title: String,
        summary: String,
        sections: Vec<(&'static str, String)>,
        action: Action,
    },
    /// The drum circle at the end of a mission.
    Finale(FinaleDef),
    /// A titled set of horizontal bars.
    Bars {
        title: String,
        rows: Vec<Bar>,
    },
    /// A titled column graph, one column a day.
    Columns {
        title: String,
        columns: Vec<DayColumn>,
    },
}

#[derive(Debug, Clone)]
pub struct Bar {
    pub label: String,
    pub value: f64,
    /// The value that fills the bar.
    pub max: f64,
    /// The figure written at its end.
    pub figure: String,
}

#[derive(Debug, Clone)]
pub struct DayColumn {
    pub day: u32,
    pub value: f64,
    pub figure: String,
}

#[derive(Debug, Clone, Default)]
pub struct Page {
    /// Selection and scroll persist while this is unchanged.
    pub id: String,
    /// Full-width blocks at the top.
    pub head: Vec<Block>,
    /// The main column: text and buttons.
    pub body: Vec<Block>,
    /// Pictures and maps: a right-hand column on wide screens.
    pub side: Vec<Block>,
    pub default_selected: usize,
    /// What Esc does.
    pub back: Option<Action>,
}

impl Page {
    fn actions(&self) -> impl Iterator<Item = &Action> {
        // In painting order, which numbers them: the main column, then the side.
        self.body.iter().chain(&self.side).filter_map(|b| match b {
            Block::Button { action, .. } | Block::Card { action, .. } => Some(action),
            _ => None,
        })
    }

    pub fn button_count(&self) -> usize {
        self.actions().count()
    }

    pub fn action(&self, index: usize) -> Option<&Action> {
        self.actions().nth(index)
    }

    pub fn text(&mut self, text: impl Into<String>, tone: Tone) {
        self.body.push(Block::Text(text.into(), tone));
    }

    pub fn button(&mut self, label: impl Into<String>, detail: impl Into<String>, action: Action) {
        self.body.push(Block::Button {
            label: label.into(),
            detail: detail.into(),
            action,
        });
    }
}

/// "1 day", "2 days".
pub fn days(n: u32) -> String {
    format!("{n} day{}", if n == 1 { "" } else { "s" })
}

/// A clickable rectangle in content coordinates (before scrolling).
#[derive(Debug, Clone)]
pub struct Hotspot {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Position among the page's buttons; `None` for map dots.
    pub button: Option<usize>,
    pub action: Action,
}

impl Hotspot {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

#[derive(Debug, Default)]
pub struct Layout {
    pub hotspots: Vec<Hotspot>,
    /// Height of the scrollable area (the window minus the status bar).
    pub view_height: f64,
    /// Screen rectangle (x, y, w, h) of the gathering field, if shown.
    pub field: Option<(f64, f64, f64, f64)>,
    /// Screen rectangle of the pop-up message, if one is up.
    pub toast: Option<(f64, f64, f64, f64)>,
    /// Something that moves by itself is in view.
    pub animated: bool,
    /// Screen rectangles of the status bar's buttons, if shown.
    pub stats_button: Option<(f64, f64, f64, f64)>,
    pub menu_button: Option<(f64, f64, f64, f64)>,
}

impl Layout {
    fn find(&self, x: f64, y: f64) -> Option<&Hotspot> {
        // Later hotspots are drawn on top.
        let top = self.hotspots.iter().rev().find(|h| h.contains(x, y))?;
        if top.button.is_some() || !matches!(top.action, Action::ShowPlace(_)) {
            return Some(top);
        }
        // Map dots have generous, overlapping targets: take the nearest.
        let off = |h: &Hotspot| (h.x + h.w / 2.0 - x).hypot(h.y + h.h / 2.0 - y);
        self.hotspots
            .iter()
            .filter(|h| matches!(h.action, Action::ShowPlace(_)) && h.contains(x, y))
            .min_by(|a, b| off(a).total_cmp(&off(b)))
    }

    pub fn hit(&self, x: f64, y: f64) -> Option<&Action> {
        self.find(x, y).map(|h| &h.action)
    }

    pub fn button_at(&self, x: f64, y: f64) -> Option<usize> {
        self.find(x, y).and_then(|h| h.button)
    }

    /// Top and bottom of a button.
    pub fn button_span(&self, index: usize) -> Option<(f64, f64)> {
        self.hotspots
            .iter()
            .find(|h| h.button == Some(index))
            .map(|h| (h.y, h.y + h.h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tap_between_map_dots_picks_the_nearer_one() {
        let dot = |id: &str, x: f64| Hotspot {
            x: x - 22.0,
            y: 0.0,
            w: 44.0,
            h: 44.0,
            button: None,
            action: Action::ShowPlace(id.into()),
        };
        let layout = Layout {
            hotspots: vec![dot("west", 100.0), dot("east", 120.0)],
            ..Layout::default()
        };
        let hit = |x| layout.hit(x, 22.0).cloned();
        assert_eq!(hit(104.0), Some(Action::ShowPlace("west".into())));
        assert_eq!(hit(117.0), Some(Action::ShowPlace("east".into())));
        assert_eq!(hit(200.0), None);
    }
}
