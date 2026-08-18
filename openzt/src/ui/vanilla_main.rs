use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};

use egui::{
    pos2, text::LayoutJob, vec2, Align, Align2, Color32, ColorImage, Context, FontData, FontDefinitions, FontFamily, FontId, Painter, PointerButton, Pos2,
    Rect, Stroke, StrokeKind, TextureHandle, Ui, Vec2,
};
use openzt_configparser::ini::Ini;
use tracing::{info, warn};

use crate::animation::Animation;

use super::{tga, zt_image};

static TEXTURES: OnceLock<Mutex<TextureCache>> = OnceLock::new();
static BUTTONS: OnceLock<Mutex<ButtonState>> = OnceLock::new();
static HIT_REGIONS: OnceLock<Mutex<Vec<HitRegion>>> = OnceLock::new();
static BOLD_FONT_REGISTERED: AtomicBool = AtomicBool::new(false);
static BOLD_FONT_ACTIVE: AtomicBool = AtomicBool::new(false);
static HELP_HOVERED_THIS_FRAME: AtomicBool = AtomicBool::new(false);
static HELP_HOVERED_LAST_FRAME: AtomicBool = AtomicBool::new(false);
static OPTIONS_MENU_VISIBLE: AtomicBool = AtomicBool::new(false);
static OPTIONS_MENU_TAB: OnceLock<Mutex<OptionsTab>> = OnceLock::new();
static RESEARCH_PANEL_VISIBLE: AtomicBool = AtomicBool::new(false);
static RESEARCH_PANEL_TAB: OnceLock<Mutex<ResearchTab>> = OnceLock::new();
static RESEARCH_PANEL_STATE: OnceLock<Mutex<ResearchPanelState>> = OnceLock::new();
/// Runtime-discovered research icon paths (e.g. `research/strain2/strain2`) leaked to `&'static str`
/// once each, so they can be used as `TextureCache`/`ButtonState` keys like every other resource path
/// in this file. See `leaked_icon_resource`.
static RESEARCH_ICON_RESOURCES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
static RESEARCH_XPAC_MENU_OPEN: AtomicBool = AtomicBool::new(false);
/// `None` = no filter ("All"); `Some(id)` = only show categories whose `expansion_id()` matches one
/// of `research_expansion_options()`'s ids. Shared across the Research/Conservation tabs since
/// expansions are zoo-wide, not per-branch.
static RESEARCH_EXPANSION_FILTER: OnceLock<Mutex<Option<i32>>> = OnceLock::new();

/// Every registered expansion (vanilla retail ones from `xpac0N.cfg` *and* any OpenZT custom
/// expansions from mod config - see `expansions::get_expansions`), as `(category_filter_id,
/// display_name)` pairs for the `[Xpac]` dropdown. `expansions::Expansion::expansion_id()` is
/// `ZTResearchCategory::expansion_id() + 1` for the matching expansion (`parse_expansion_config`
/// stores the raw `.cfg` id `+1`; see `ZTResearchCategory`'s own doc comment on the same offset), so
/// it's converted back here to match what categories actually store. Registry id 0 (the synthetic
/// "All" bucket `initialise_expansions` always adds) is skipped since our own dropdown already has
/// its own "All" entry that clears the filter. `load_string_by_id` resolves the display name for
/// both vanilla (plain string-table id) and custom (OpenZT-registered, `>= 100_000`) expansions
/// uniformly - it already checks the OpenZT registry first.
fn research_expansion_options() -> Vec<(i32, String)> {
    crate::expansions::get_expansions()
        .into_iter()
        .filter(|expansion| expansion.expansion_id() != 0)
        .filter_map(|expansion| {
            let name = crate::string_registry::load_string_by_id(expansion.name_id())?;
            Some((expansion.expansion_id() as i32 - 1, name))
        })
        .collect()
}

/// "All" + one row per `research_expansion_options()` entry. The popup grows to fit however many
/// expansions are actually registered (vanilla resizes its own dropdown the same way for custom
/// expansions - see `expansions::resize_expansion_dropdown`), rather than assuming a fixed count.
/// Row height matches the width used by `draw_research_xpac_row`'s `row_rect.shrink2(vec2(4.0, 0.0))`.
const RESEARCH_XPAC_TEXT_WRAP_WIDTH: f32 = 123.0 - 2.0 * 4.0;

/// Per-row height for the "All" row plus each `research_expansion_options()` entry, computed once so
/// popup sizing (`research_xpac_popup_rect`) and row drawing (`draw_research_xpac_menu`) can never
/// disagree - see `measure_row_height`.
fn research_xpac_row_heights(ctx: &Context) -> Vec<f32> {
    std::iter::once(measure_row_height(ctx, "All", bold_font(12.0), RESEARCH_XPAC_TEXT_WRAP_WIDTH))
        .chain(
            research_expansion_options()
                .into_iter()
                .map(|(_, name)| measure_row_height(ctx, &name, bold_font(12.0), RESEARCH_XPAC_TEXT_WRAP_WIDTH)),
        )
        .collect()
}

fn research_xpac_popup_rect(ctx: &Context, origin: Pos2) -> Rect {
    let row_heights = research_xpac_row_heights(ctx);
    let height = 2.0 + row_heights.iter().sum::<f32>() + 2.0;
    Rect::from_min_size(origin + vec2(51.0, 33.0), vec2(127.0, height))
}

/// Approximate area of the `[Xpac]` toggle button, used only to exclude it from the "click outside
/// closes the popup" check below - the button's own press/release cycle already handles opening and
/// closing it, so treating a press there as "outside" would race with that and could immediately
/// reopen the popup it just closed (or vice versa).
fn research_xpac_button_rect(origin: Pos2) -> Rect {
    Rect::from_min_size(origin + vec2(34.0, 27.0), vec2(127.0, 20.0))
}

/// Whether the pointer is currently over the (open) expansion popup - used to stop clicks/hover from
/// passing through to whatever the popup visually covers underneath (the category list's top rows,
/// the list's up-scroll-arrow), since those widgets each do their own independent pointer-position
/// check and have no built-in notion of "something modal is drawn on top of me".
fn research_xpac_popup_blocks_pointer(ctx: &Context, origin: Pos2) -> bool {
    RESEARCH_XPAC_MENU_OPEN.load(Ordering::Acquire) && ctx.pointer_hover_pos().is_some_and(|pos| research_xpac_popup_rect(ctx, origin).contains(pos))
}

const BOLD_FONT_FAMILY: &str = "zt-bold";
const BOLD_FONT_NAME: &str = "arial-bold";
const BOLD_FONT_PATH: &str = r"C:\Windows\Fonts\arialbd.ttf";
const GREEN_TEXT: Color32 = Color32::from_rgb(83, 219, 83);
const ZERO_MONEY_TEXT: Color32 = Color32::from_rgb(0xf6, 0xd2, 0x5b);
const NEGATIVE_MONEY_TEXT: Color32 = Color32::from_rgb(0xac, 0x4d, 0x2d);
const OPTIONS_TITLE_TEXT: Color32 = Color32::from_rgb(255, 228, 173);
const OPTIONS_SUBTITLE_TEXT: Color32 = Color32::from_rgb(206, 206, 206);
const OPTIONS_LABEL_TEXT: Color32 = Color32::from_rgb(237, 228, 173);
const OPTIONS_BUTTON_TEXT: Color32 = Color32::from_rgb(255, 186, 16);
const OPTIONS_BUTTON_HOVER_TEXT: Color32 = Color32::from_rgb(255, 223, 41);
/// `resrch1.lyt`'s `ResearchProgram`/`ConservationProgram` and `resrch2.lyt`'s `CategoryName`/list
/// row `forecolor` (156, 205, 183).
const RESEARCH_TEXT_TEAL: Color32 = Color32::from_rgb(156, 205, 183);
/// `resrch2.lyt`'s `ProgramName`/`NoItemText` `forecolor` (253, 218, 88).
const RESEARCH_TEXT_GOLD: Color32 = Color32::from_rgb(253, 218, 88);

#[derive(Clone, Copy, PartialEq, Eq)]
enum OptionsTab {
    Main,
    GraphicsSound,
    Help,
    Advanced,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResearchTab {
    Status,
    Research,
    Conservation,
}

/// Index of the "Research" and "Conservation" branches within `ZTResearchMgr::branches()`, per
/// `research.cfg`'s `[branches]` list order (`branch=research/branres.cfg` then
/// `branch=research/brancon.cfg`). Verify against the live game with the `list_research()` Lua
/// console command if the panel ever shows swapped branch data.
const RESEARCH_BRANCH_INDEX: usize = 0;
const CONSERVATION_BRANCH_INDEX: usize = 1;

/// Row heights used to lay out the category list box; `resrch2.lyt`'s `UIListBox` doesn't specify an
/// explicit row height (the vanilla engine derives it from font metrics + `miniconwidth`), so these
/// are reasonable approximations to verify visually in-game. Which one a given row uses is decided
/// per-row by `measure_row_height`, based on whether its name actually wraps to one or two lines.
/// Single-line height is tall enough for the list's 22x18 checkbox plus a little padding.
const RESEARCH_LIST_ROW_HEIGHT_SINGLE: f32 = 22.0;
/// Tall enough for a two-line wrapped name.
const RESEARCH_LIST_ROW_HEIGHT_DOUBLE: f32 = 32.0;

/// String ids found via the live game's string table (not present in any `.lyt`/decompile in this
/// repo), used as fallback/tooltip text for the research panel's dynamic fields.
const RESEARCH_STRING_NOT_FUNDED: u32 = 23030;
const RESEARCH_STRING_NO_PROGRAM: u32 = 23031;
const RESEARCH_STRING_NO_CATEGORY: u32 = 23032;
const RESEARCH_STRING_DONE_IN_ONE_DAY: u32 = 4026;
const RESEARCH_STRING_DONE_IN_DAYS: u32 = 4028;

/// Per-branch state for the Research/Conservation list tab (`resrch2.lyt`). Both tabs share the same
/// layout but operate on different branches, so each needs independent scroll/press state. Category
/// checked-state is *not* tracked here - it lives on `ZTResearchCategory::is_enabled()` itself (see
/// `set_enabled`), and the displayed category/program name/progress mirror the branch's actual
/// active research (`current_category()`/`current_program()`), same as the Status tab.
#[derive(Default, Clone, Copy)]
struct ResearchListState {
    scroll_offset: f32,
    pressed_row: Option<usize>,
    thumb_dragging: bool,
}

#[derive(Default)]
struct ResearchPanelState {
    research_list: ResearchListState,
    conservation_list: ResearchListState,
}

/// Fixed `resrch1.lyt` y-offsets for one branch-summary block (title/program-name/progress-bar/icon).
/// Research and Conservation blocks are visually identical apart from these offsets and the title text id.
struct ResearchStatusBlockLayout {
    title_text_id: u32,
    title_y: f32,
    program_y: f32,
    progress_bg_y: f32,
    progress_fill_y: f32,
    icon_y: f32,
    border_y: f32,
}

const RESEARCH_STATUS_BLOCK: ResearchStatusBlockLayout = ResearchStatusBlockLayout {
    title_text_id: 4040,
    title_y: 45.0,
    program_y: 65.0,
    progress_bg_y: 203.0,
    progress_fill_y: 205.0,
    icon_y: 116.0,
    border_y: 115.0,
};

const CONSERVATION_STATUS_BLOCK: ResearchStatusBlockLayout = ResearchStatusBlockLayout {
    title_text_id: 4039,
    title_y: 234.0,
    program_y: 255.0,
    progress_bg_y: 393.0,
    progress_fill_y: 395.0,
    icon_y: 310.0,
    border_y: 309.0,
};

pub fn set_money_value(value: f32) {
    super::money_display::set_value(value);
}

pub fn set_date_value(value: nt_time::time::UtcDateTime) {
    super::date_display::set_value(value);
}

pub fn blocks_pointer_at(pos: Pos2, screen_size: Vec2) -> bool {
    if let Ok(regions) = HIT_REGIONS.get_or_init(|| Mutex::new(Vec::new())).lock()
        && !regions.is_empty()
    {
        return regions.iter().rev().any(|region| region.blocks(pos));
    }

    fallback_main_ui_block_rects(screen_size, |rect| rect.contains(pos))
}

struct TextureCache {
    animations: HashMap<TextureKey, CachedTexture>,
    tgas: HashMap<&'static str, CachedTgaTexture>,
}

impl TextureCache {
    fn new() -> Self {
        Self {
            animations: HashMap::new(),
            tgas: HashMap::new(),
        }
    }

    fn animation(&mut self, ctx: &Context, base: &'static str, visual_state: VisualState) -> Option<LoadedTexture> {
        let key = TextureKey { base, visual_state };
        let entry = self.animations.entry(key).or_default();
        entry.texture(ctx, base, visual_state)
    }

    fn tga(&mut self, ctx: &Context, resource: &'static str) -> Option<LoadedTgaTexture> {
        let entry = self.tgas.entry(resource).or_default();
        entry.texture(ctx, resource)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TextureKey {
    base: &'static str,
    visual_state: VisualState,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum VisualState {
    Normal,
    Hover,
    Selected,
    Disabled,
}

impl VisualState {
    fn animation_name(self) -> &'static str {
        match self {
            Self::Normal => "N",
            Self::Hover => "H",
            Self::Selected => "S",
            Self::Disabled => "G",
        }
    }
}

#[derive(Default)]
struct ButtonState {
    selected: HashSet<&'static str>,
    pressed: HashSet<&'static str>,
}

#[derive(Clone, Copy)]
enum ButtonMode {
    Momentary,
    Selected,
}

#[derive(Default)]
struct CachedTexture {
    texture: Option<TextureHandle>,
    size: Vec2,
    offset: Vec2,
    mask: Option<Arc<HitMask>>,
    failed: bool,
    missing_logged: bool,
}

#[derive(Default)]
struct CachedTgaTexture {
    texture: Option<TextureHandle>,
    size: Vec2,
    missing_logged: bool,
}

impl CachedTexture {
    fn texture(&mut self, ctx: &Context, base: &'static str, visual_state: VisualState) -> Option<LoadedTexture> {
        if let Some(texture) = &self.texture {
            return Some(LoadedTexture {
                texture: texture.clone(),
                size: self.size,
                offset: self.offset,
                mask: self.mask.clone()?,
            });
        }

        if self.failed {
            return None;
        }

        let Some(texture) = load_animation_texture(ctx, base, visual_state, &mut self.missing_logged) else {
            return None;
        };

        self.size = texture.size;
        self.offset = texture.offset;
        self.texture = Some(texture.texture.clone());
        self.mask = Some(texture.mask.clone());
        Some(texture)
    }
}

impl CachedTgaTexture {
    fn texture(&mut self, ctx: &Context, resource: &'static str) -> Option<LoadedTgaTexture> {
        if let Some(texture) = &self.texture {
            return Some(LoadedTgaTexture {
                texture: texture.clone(),
                size: self.size,
            });
        }

        let Some(texture) = load_tga_texture(ctx, resource, &mut self.missing_logged) else {
            return None;
        };

        self.size = texture.size;
        self.texture = Some(texture.texture.clone());
        Some(texture)
    }
}

#[derive(Clone)]
struct LoadedTexture {
    texture: TextureHandle,
    size: Vec2,
    offset: Vec2,
    mask: Arc<HitMask>,
}

#[derive(Clone)]
struct LoadedTgaTexture {
    texture: TextureHandle,
    size: Vec2,
}

#[derive(Clone, Copy)]
struct DrawnRect {
    rect: Rect,
    loaded: bool,
}

#[derive(Clone)]
struct HitRegion {
    rect: Rect,
    uv: Rect,
    mask: Arc<HitMask>,
}

impl HitRegion {
    fn blocks(&self, pos: Pos2) -> bool {
        if !self.rect.contains(pos) || self.rect.width() <= 0.0 || self.rect.height() <= 0.0 {
            return false;
        }

        let local_x = (pos.x - self.rect.left()) / self.rect.width();
        let local_y = (pos.y - self.rect.top()) / self.rect.height();
        let u = self.uv.left() + local_x * self.uv.width();
        let v = self.uv.top() + local_y * self.uv.height();
        self.mask.blocks_uv(u, v)
    }
}

#[derive(Clone)]
struct HitMask {
    width: usize,
    height: usize,
    alpha: Vec<bool>,
}

impl HitMask {
    fn from_image(image: &ColorImage) -> Self {
        Self {
            width: image.size[0],
            height: image.size[1],
            alpha: image.pixels.iter().map(|pixel| pixel.a() > 0).collect(),
        }
    }

    fn blocks_uv(&self, u: f32, v: f32) -> bool {
        if self.width == 0 || self.height == 0 || !u.is_finite() || !v.is_finite() {
            return false;
        }

        let x = (u.clamp(0.0, 1.0) * self.width as f32).floor() as usize;
        let y = (v.clamp(0.0, 1.0) * self.height as f32).floor() as usize;
        let x = x.min(self.width - 1);
        let y = y.min(self.height - 1);
        self.alpha.get(y * self.width + x).copied().unwrap_or(false)
    }
}

pub fn show(ctx: &Context, screen_size: Vec2) {
    prepare_bold_font(ctx);

    egui::Area::new("openzt_vanilla_main_ui".into())
        .order(egui::Order::Background)
        .fixed_pos(Pos2::ZERO)
        .interactable(true)
        .show(ctx, |ui| {
            ui.set_min_size(screen_size);
            let painter = ui.painter().clone();
            let cache = TEXTURES.get_or_init(|| Mutex::new(TextureCache::new()));
            let mut cache = match cache.lock() {
                Ok(cache) => cache,
                Err(err) => {
                    warn!("egui overlay: vanilla UI texture cache lock poisoned: {err}");
                    return;
                }
            };

            let buttons = BUTTONS.get_or_init(|| Mutex::new(ButtonState::default()));
            let mut buttons = match buttons.lock() {
                Ok(buttons) => buttons,
                Err(err) => {
                    warn!("egui overlay: vanilla UI button state lock poisoned: {err}");
                    return;
                }
            };

            let mut hit_regions = Vec::new();
            draw_main_ui(ctx, ui, &painter, &mut cache, &mut buttons, &mut hit_regions, screen_size);
            remember_hit_regions(hit_regions);
        });
}

fn draw_main_ui(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    screen_size: Vec2,
) {
    HELP_HOVERED_THIS_FRAME.store(false, Ordering::Release);

    let bg1_size = texture_size(ctx, cache, "ui/main/backgnd1/backgnd1", VisualState::Normal).unwrap_or(vec2(64.0, 248.0));
    let bg2_size = texture_size(ctx, cache, "ui/main/backgnd2/backgnd2", VisualState::Normal).unwrap_or(vec2(170.0, 128.0));
    let bg3_size = texture_size(ctx, cache, "ui/main/backgnd3/backgnd3", VisualState::Normal).unwrap_or(vec2(200.0, 112.0));
    let bg4_size = texture_size(ctx, cache, "ui/main/backgnd4/backgnd4", VisualState::Normal).unwrap_or(vec2(330.0, 38.0));
    let bg5_size = texture_size(ctx, cache, "ui/main/backgnd5/backgnd5", VisualState::Normal).unwrap_or(vec2(256.0, 38.0));

    let bg1_rect = rect_from_pos_size(pos2(0.0, 0.0), bg1_size);
    let bg2_pos = pos2(0.0, (screen_size.y - bg2_size.y).max(0.0));
    let bg3_pos = pos2(0.0, (screen_size.y - bg3_size.y).max(0.0));
    let bg4_pos = pos2(((screen_size.x - bg4_size.x) * 0.5).max(0.0), (screen_size.y - bg4_size.y).max(0.0));
    let bg5_pos = pos2((screen_size.x - bg5_size.x).max(0.0), (screen_size.y - bg5_size.y).max(0.0));

    let bg2 = rect_from_pos_size(bg2_pos, bg2_size);
    let bg3 = rect_from_pos_size(bg3_pos, bg3_size);
    let bg4 = rect_from_pos_size(bg4_pos, bg4_size);
    let bg5 = rect_from_pos_size(bg5_pos, bg5_size);

    if bg2.top() > bg1_rect.bottom() {
        draw_tiled_y(ctx, painter, cache, hit_regions, "ui/main/bg2/bg2", pos2(0.0, bg1_rect.bottom()), bg2.top() - bg1_rect.bottom());
    }
    if bg4.left() > bg3.right() {
        draw_tiled_x_bottom(ctx, painter, cache, hit_regions, "ui/main/bg3/bg3", bg3.right(), screen_size.y, bg4.left() - bg3.right());
    }
    if bg5.left() > bg4.right() {
        draw_tiled_x_bottom(ctx, painter, cache, hit_regions, "ui/main/bg4/bg4", bg4.right(), screen_size.y, bg5.left() - bg4.right());
    }

    draw_options_menu(ctx, ui, painter, cache, buttons, hit_regions);
    draw_research_panel(ctx, painter, cache, buttons, hit_regions);

    draw_anim(ctx, painter, cache, hit_regions, "ui/main/backgnd4/backgnd4", bg4_pos, bg4_size);
    let bg1 = draw_anim(ctx, painter, cache, hit_regions, "ui/main/backgnd1/backgnd1", pos2(0.0, 0.0), bg1_size);
    draw_anim(ctx, painter, cache, hit_regions, "ui/main/backgnd2/backgnd2", bg2_pos, bg2_size);
    draw_anim(ctx, painter, cache, hit_regions, "ui/main/backgnd3/backgnd3", bg3_pos, bg3_size);
    draw_anim(ctx, painter, cache, hit_regions, "ui/main/backgnd5/backgnd5", bg5_pos, bg5_size);

    draw_left_buttons(ctx, ui, painter, cache, buttons, hit_regions, bg1.rect);
    draw_minimap_cluster(ctx, ui, painter, cache, buttons, hit_regions, bg2);
    draw_time_and_money(ctx, ui, painter, cache, buttons, hit_regions, bg3, bg4);
    draw_status_cluster(ctx, ui, painter, cache, buttons, hit_regions, bg4, bg5);

    finish_help_tooltip_frame();
}

fn remember_hit_regions(hit_regions: Vec<HitRegion>) {
    if let Ok(mut stored) = HIT_REGIONS.get_or_init(|| Mutex::new(Vec::new())).lock() {
        *stored = hit_regions;
    }
}

fn fallback_main_ui_block_rects(screen_size: Vec2, mut visit: impl FnMut(Rect) -> bool) -> bool {
    let bg1 = Rect::from_min_size(pos2(0.0, 0.0), vec2(64.0, 248.0));
    let bg2 = Rect::from_min_size(pos2(0.0, (screen_size.y - 128.0).max(0.0)), vec2(170.0, 128.0));
    let bg3 = Rect::from_min_size(pos2(0.0, (screen_size.y - 112.0).max(0.0)), vec2(200.0, 112.0));
    let bg4 = Rect::from_min_size(pos2(((screen_size.x - 330.0) * 0.5).max(0.0), (screen_size.y - 38.0).max(0.0)), vec2(330.0, 38.0));
    let bg5 = Rect::from_min_size(pos2((screen_size.x - 256.0).max(0.0), (screen_size.y - 38.0).max(0.0)), vec2(256.0, 38.0));

    if [bg1, bg2, bg3, bg4, bg5].into_iter().any(&mut visit) {
        return true;
    }
    if bg2.top() > bg1.bottom() {
        if visit(Rect::from_min_max(pos2(0.0, bg1.bottom()), pos2(64.0, bg2.top()))) {
            return true;
        }
    }
    if bg4.left() > bg3.right() {
        if visit(Rect::from_min_max(
            pos2(bg3.right(), (screen_size.y - 112.0).max(0.0)),
            pos2(bg4.left(), screen_size.y),
        )) {
            return true;
        }
    }
    if bg5.left() > bg4.right() {
        if visit(Rect::from_min_max(
            pos2(bg4.right(), (screen_size.y - 38.0).max(0.0)),
            pos2(bg5.left(), screen_size.y),
        )) {
            return true;
        }
    }

    false
}

fn draw_left_buttons(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons_state: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    bg1: Rect,
) {
    let buttons = [
        ("ui/main/habitat/habitat", 4.0, 13.0),
        ("ui/main/buyanim/buyanim", 4.0, 60.0),
        ("ui/main/buyobj/buyobj", 4.0, 108.0),
        ("ui/main/person/person", 4.0, 154.0),
        ("ui/main/undo/undo", 1.0, 258.0),
        ("ui/main/bdoz/bdoz", 1.0, 293.0),
        ("ui/main/msgs/msgs", 1.0, 328.0),
        ("ui/main/resr/resr", 1.0, 363.0),
        ("ui/scenario/scenbut/scenbut", 1.0, 398.0),
        ("ui/main/gameopt/gameopt", 1.0, 433.0),
    ];

    for (resource, x, y) in buttons {
        if resource == "ui/main/gameopt/gameopt" {
            let selected = OPTIONS_MENU_VISIBLE.load(Ordering::Acquire);
            draw_explicit_button(
                ctx,
                painter,
                cache,
                buttons_state,
                hit_regions,
                "main-options-button",
                resource,
                bg1.min + vec2(x, y),
                vec2(40.0, 40.0),
                selected,
                true,
                Some(toggle_options_menu),
                button_help_id(resource),
            );
        } else if resource == "ui/main/resr/resr" {
            let selected = RESEARCH_PANEL_VISIBLE.load(Ordering::Acquire);
            draw_explicit_button(
                ctx,
                painter,
                cache,
                buttons_state,
                hit_regions,
                "main-research-button",
                resource,
                bg1.min + vec2(x, y),
                vec2(40.0, 40.0),
                selected,
                true,
                Some(toggle_research_panel),
                button_help_id(resource),
            );
        } else {
            draw_button(
                ctx,
                ui,
                painter,
                cache,
                buttons_state,
                hit_regions,
                resource,
                bg1.min + vec2(x, y),
                vec2(40.0, 40.0),
                ButtonMode::Selected,
            );
        }
    }
}

fn draw_options_menu(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
) {
    if !OPTIONS_MENU_VISIBLE.load(Ordering::Acquire) {
        return;
    }

    let origin = pos2(17.0, 6.0);
    draw_anim(ctx, painter, cache, hit_regions, "ui/gameopts/optpanbk/optpanbk", origin, vec2(179.0, 440.0));
    draw_text_id(painter, 1540, Rect::from_min_size(origin + vec2(35.0, 5.0), vec2(100.0, 18.0)), 13.0, OPTIONS_TITLE_TEXT, Align2::LEFT_CENTER);

    draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        "options-close",
        "ui/sharedui/close/close",
        origin + vec2(142.0, 3.0),
        vec2(24.0, 24.0),
        false,
        true,
        Some(hide_options_menu),
        Some(3112),
    );

    let active_tab = active_options_tab();
    draw_options_tab_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        origin + vec2(154.0, 80.0),
        "options-tab-main",
        "ui/gameopts/gameopt/gameopt",
        OptionsTab::Main,
        active_tab,
        1541,
    );
    draw_options_tab_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        origin + vec2(154.0, 114.0),
        "options-tab-graphics-sound",
        "ui/gameopts/setting/setting",
        OptionsTab::GraphicsSound,
        active_tab,
        1542,
    );
    draw_options_tab_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        origin + vec2(154.0, 147.0),
        "options-tab-help",
        "ui/gameopts/tooltip/tooltip",
        OptionsTab::Help,
        active_tab,
        1543,
    );
    draw_options_tab_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        origin + vec2(154.0, 180.0),
        "options-tab-advanced",
        "ui/gameopts/advance/advance",
        OptionsTab::Advanced,
        active_tab,
        1530,
    );
    draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        "options-about-button",
        "ui/gameopts/about/about",
        origin + vec2(154.0, 213.0),
        vec2(34.0, 34.0),
        false,
        true,
        Some(show_about_help_popup),
        Some(1544),
    );

    match active_tab {
        OptionsTab::Main => draw_options_main_tab(ctx, painter, cache, buttons, hit_regions, origin),
        OptionsTab::GraphicsSound => draw_options_graphics_sound_tab(ctx, painter, cache, buttons, hit_regions, origin),
        OptionsTab::Help => draw_options_help_tab(ctx, painter, cache, buttons, hit_regions, origin),
        OptionsTab::Advanced => draw_options_advanced_tab(ctx, painter, cache, buttons, hit_regions, origin),
    }

    let _ = ui;
}

fn draw_options_main_tab(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    origin: Pos2,
) {
    draw_options_subtitle(painter, origin, 1541);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-load-game", origin + vec2(32.0, 252.0), 1501, Some(1560), false);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-save-game", origin + vec2(32.0, 283.0), 1502, Some(1561), false);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-start-new", origin + vec2(32.0, 314.0), 1503, Some(1562), false);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-exit-game", origin + vec2(32.0, 345.0), 1504, Some(1563), false);
}

fn draw_options_graphics_sound_tab(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    origin: Pos2,
) {
    draw_options_subtitle(painter, origin, 1542);
    draw_options_label(painter, origin + vec2(38.0, 120.0), vec2(110.0, 16.0), 1510, Align2::LEFT_CENTER);
    draw_options_slider(ctx, painter, cache, buttons, hit_regions, "options-main-volume", origin + vec2(38.0, 135.0), 0.65, Some(1564));
    draw_options_label(painter, origin + vec2(38.0, 165.0), vec2(110.0, 16.0), 1512, Align2::LEFT_CENTER);
    draw_options_slider(ctx, painter, cache, buttons, hit_regions, "options-menu-music", origin + vec2(38.0, 180.0), 0.45, Some(1565));
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-window-mode", origin + vec2(32.0, 210.0), 1587, Some(1566), false);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-resolution-800", origin + vec2(32.0, 245.0), 1517, Some(1568), true);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-resolution-1024", origin + vec2(32.0, 275.0), 1518, Some(1569), false);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-resolution-1280", origin + vec2(32.0, 305.0), 1519, Some(1570), false);

    draw_options_label(painter, origin + vec2(33.0, 334.0), vec2(121.0, 16.0), 1598, Align2::CENTER_CENTER);
    draw_radio_row(ctx, painter, cache, buttons, hit_regions, "options-terrain-none", origin + vec2(36.0, 350.0), 1594, Some(1534), false);
    draw_radio_row(ctx, painter, cache, buttons, hit_regions, "options-terrain-speed", origin + vec2(36.0, 367.0), 1595, Some(1535), false);
    draw_radio_row(ctx, painter, cache, buttons, hit_regions, "options-terrain-balanced", origin + vec2(36.0, 384.0), 1596, Some(1536), true);
    draw_radio_row(ctx, painter, cache, buttons, hit_regions, "options-terrain-quality", origin + vec2(36.0, 401.0), 1597, Some(1537), false);
}

fn draw_options_help_tab(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    origin: Pos2,
) {
    draw_options_subtitle(painter, origin, 1543);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-tooltip-short", origin + vec2(32.0, 217.0), 1520, Some(1571), true);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-tooltip-long", origin + vec2(32.0, 247.0), 1521, Some(1572), false);
    draw_options_label(painter, origin + vec2(38.0, 275.0), vec2(128.0, 16.0), 1522, Align2::LEFT_CENTER);
    draw_options_slider(ctx, painter, cache, buttons, hit_regions, "options-tooltip-delay", origin + vec2(38.0, 290.0), 0.35, Some(1573));
    draw_options_label(painter, origin + vec2(38.0, 320.0), vec2(128.0, 16.0), 1524, Align2::LEFT_CENTER);
    draw_options_slider(ctx, painter, cache, buttons, hit_regions, "options-tooltip-duration", origin + vec2(38.0, 335.0), 0.65, Some(1574));
    draw_options_label(painter, origin + vec2(38.0, 365.0), vec2(128.0, 16.0), 1514, Align2::LEFT_CENTER);
    draw_options_slider(ctx, painter, cache, buttons, hit_regions, "options-main-scroll-speed", origin + vec2(38.0, 380.0), 0.55, Some(1575));
}

fn draw_options_advanced_tab(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    origin: Pos2,
) {
    draw_options_subtitle_id(painter, origin, 1530);
    draw_options_label(painter, origin + vec2(33.0, 130.0), vec2(120.0, 16.0), 1547, Align2::CENTER_CENTER);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-video-high", origin + vec2(32.0, 147.0), 1539, Some(1557), true);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-video-low", origin + vec2(32.0, 177.0), 1546, Some(1558), false);

    draw_options_label(painter, origin + vec2(33.0, 222.0), vec2(120.0, 16.0), 1548, Align2::CENTER_CENTER);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-sound-high", origin + vec2(32.0, 237.0), 1551, Some(1559), true);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-sound-low", origin + vec2(32.0, 267.0), 1552, Some(1578), false);

    draw_options_label(painter, origin + vec2(33.0, 311.0), vec2(120.0, 16.0), 1553, Align2::CENTER_CENTER);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-mouse-high", origin + vec2(32.0, 327.0), 1555, Some(1579), false);
    draw_text_button(ctx, painter, cache, buttons, hit_regions, "options-mouse-low", origin + vec2(32.0, 357.0), 1556, Some(1582), true);
}

fn draw_options_tab_button(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    pos: Pos2,
    key: &'static str,
    resource: &'static str,
    tab: OptionsTab,
    active_tab: OptionsTab,
    help_id: i32,
) {
    let action = match tab {
        OptionsTab::Main => set_options_tab_main,
        OptionsTab::GraphicsSound => set_options_tab_graphics_sound,
        OptionsTab::Help => set_options_tab_help,
        OptionsTab::Advanced => set_options_tab_advanced,
    };
    draw_explicit_button(ctx, painter, cache, buttons, hit_regions, key, resource, pos, vec2(34.0, 34.0), tab == active_tab, true, Some(action), Some(help_id));
}

fn draw_options_subtitle(painter: &Painter, origin: Pos2, text_id: u32) {
    draw_options_subtitle_id(painter, origin, text_id);
}

fn draw_options_subtitle_id(painter: &Painter, origin: Pos2, text_id: u32) {
    draw_text_id(
        painter,
        text_id,
        Rect::from_min_size(origin + vec2(29.0, 29.0), vec2(127.0, 18.0)),
        13.0,
        OPTIONS_SUBTITLE_TEXT,
        Align2::CENTER_CENTER,
    );
}

fn draw_options_label(painter: &Painter, pos: Pos2, size: Vec2, text_id: u32, align: Align2) {
    draw_text_id(painter, text_id, Rect::from_min_size(pos, size), 12.0, OPTIONS_LABEL_TEXT, align);
}

fn draw_text_button(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    key: &'static str,
    pos: Pos2,
    text_id: u32,
    help_id: Option<i32>,
    selected: bool,
) {
    let button = draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        key,
        "ui/gameopts/textbck/textbck",
        pos,
        vec2(124.0, 25.0),
        selected,
        true,
        None,
        help_id,
    );
    let color = if rect_contains_pointer(ctx, button.rect) {
        OPTIONS_BUTTON_HOVER_TEXT
    } else {
        OPTIONS_BUTTON_TEXT
    };
    draw_text_id(painter, text_id, button.rect.shrink2(vec2(8.0, 2.0)), 13.0, color, Align2::CENTER_CENTER);
}

fn draw_options_slider(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    _buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    key: &'static str,
    pos: Pos2,
    value: f32,
    help_id: Option<i32>,
) {
    let track = draw_anim_fixed_size(
        ctx,
        painter,
        cache,
        hit_regions,
        "ui/sharedui/horzslid/horzslid",
        pos,
        vec2(110.0, 26.0),
    );
    if let Some(help_id) = help_id {
        request_help_tooltip_for_rect(ctx, track.rect, help_id);
    }
    let thumb_size = texture_size(ctx, cache, "ui/sharedui/horzthm/horzthm", VisualState::Normal).unwrap_or(vec2(14.0, 26.0));
    let thumb_x = pos.x + (track.rect.width() - thumb_size.x).max(0.0) * value.clamp(0.0, 1.0);
    let thumb_y = pos.y + ((track.rect.height() - thumb_size.y) * 0.5).max(0.0);
    draw_anim(ctx, painter, cache, hit_regions, "ui/sharedui/horzthm/horzthm", pos2(thumb_x, thumb_y), vec2(14.0, 26.0));
    let _ = key;
}

fn draw_radio_row(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    key: &'static str,
    pos: Pos2,
    text_id: u32,
    help_id: Option<i32>,
    selected: bool,
) {
    draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        key,
        "ui/sharedui/radio/radio",
        pos,
        vec2(14.0, 14.0),
        selected,
        true,
        None,
        help_id,
    );
    let label_rect = Rect::from_min_size(pos + vec2(18.0, -1.0), vec2(95.0, 16.0));
    if let Some(help_id) = help_id {
        request_help_tooltip_for_rect(ctx, label_rect, help_id);
    }
    draw_text_id(painter, text_id, label_rect, 12.0, OPTIONS_LABEL_TEXT, Align2::LEFT_CENTER);
}

/// Draws `text` wrapped to fit `rect`'s width, up to `max_rows` lines (eliding the last row with
/// `…` via `TextWrapping::overflow_character` if it still doesn't fit), aligned per `halign` and
/// vertically centered within `rect`. Used for research/program/category names, which vanilla's
/// fixed-size text fields are too narrow to reliably show on one line.
fn draw_wrapped_text(ctx: &Context, painter: &Painter, text: &str, rect: Rect, font: FontId, color: Color32, halign: Align, max_rows: usize) {
    if text.is_empty() {
        return;
    }
    let mut job = LayoutJob::simple(text.to_string(), font, color, rect.width().max(0.0));
    job.halign = halign;
    job.wrap.max_rows = max_rows;
    let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
    // Without `job.justify`, epaint's halign centers/right-aligns each row around x=0 relative to
    // that row's own natural width (not within the wrap box) - so the anchor point must move with
    // `halign` rather than always being the rect's left edge.
    let pos_x = match halign {
        Align::Center => rect.center().x,
        Align::Max => rect.right(),
        Align::Min => rect.left(),
    };
    let pos = pos2(pos_x, rect.center().y - galley.rect.height() / 2.0);
    painter.galley(pos, galley, color);
}

/// Whether `text`, wrapped at `wrap_width` in `font`, needs one line or two - the shared row-height
/// classifier for both the category list and the xpac dropdown, both of which draw their row text
/// through `draw_wrapped_text` with `max_rows = 2` and want each row's own height to match how many
/// lines its name actually took, rather than assuming a uniform height regardless of content.
fn measure_row_height(ctx: &Context, text: &str, font: FontId, wrap_width: f32) -> f32 {
    if text.is_empty() {
        return RESEARCH_LIST_ROW_HEIGHT_SINGLE;
    }
    let mut job = LayoutJob::simple(text.to_string(), font, Color32::WHITE, wrap_width.max(0.0));
    job.wrap.max_rows = 2;
    let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
    if galley.rows.len() <= 1 {
        RESEARCH_LIST_ROW_HEIGHT_SINGLE
    } else {
        RESEARCH_LIST_ROW_HEIGHT_DOUBLE
    }
}

fn draw_text_id(painter: &Painter, text_id: u32, rect: Rect, size: f32, color: Color32, align: Align2) {
    let text = crate::string_registry::load_string_by_id(text_id).unwrap_or_else(|| format!("#{text_id}"));
    let pos = if align == Align2::LEFT_CENTER {
        rect.left_center()
    } else if align == Align2::CENTER_CENTER {
        rect.center()
    } else if align == Align2::RIGHT_CENTER {
        rect.right_center()
    } else if align == Align2::LEFT_TOP {
        rect.left_top()
    } else if align == Align2::CENTER_TOP {
        pos2(rect.center().x, rect.top())
    } else if align == Align2::RIGHT_TOP {
        rect.right_top()
    } else if align == Align2::LEFT_BOTTOM {
        rect.left_bottom()
    } else if align == Align2::CENTER_BOTTOM {
        pos2(rect.center().x, rect.bottom())
    } else {
        rect.right_bottom()
    };
    painter.text(pos, align, text.trim(), bold_font(size), color);
}

fn active_options_tab() -> OptionsTab {
    *OPTIONS_MENU_TAB.get_or_init(|| Mutex::new(OptionsTab::Main)).lock().unwrap()
}

fn set_active_options_tab(tab: OptionsTab) {
    if let Ok(mut active) = OPTIONS_MENU_TAB.get_or_init(|| Mutex::new(OptionsTab::Main)).lock() {
        *active = tab;
    }
}

fn toggle_options_menu() {
    let visible = !OPTIONS_MENU_VISIBLE.load(Ordering::Acquire);
    OPTIONS_MENU_VISIBLE.store(visible, Ordering::Release);
    if visible {
        set_active_options_tab(OptionsTab::Main);
        hide_research_panel();
    }
}

fn hide_options_menu() {
    OPTIONS_MENU_VISIBLE.store(false, Ordering::Release);
}

fn set_options_tab_main() {
    set_active_options_tab(OptionsTab::Main);
}

fn set_options_tab_graphics_sound() {
    set_active_options_tab(OptionsTab::GraphicsSound);
}

fn set_options_tab_help() {
    set_active_options_tab(OptionsTab::Help);
}

fn set_options_tab_advanced() {
    set_active_options_tab(OptionsTab::Advanced);
}

fn show_about_help_popup() {
    super::tooltip::set_overlay_help_tooltip(1544);
}

// ---------------------------------------------------------------------------------------------
// Research panel (research.lyt / resrch1.lyt / resrch2.lyt)
// ---------------------------------------------------------------------------------------------

fn active_research_tab() -> ResearchTab {
    *RESEARCH_PANEL_TAB.get_or_init(|| Mutex::new(ResearchTab::Status)).lock().unwrap()
}

fn set_active_research_tab(tab: ResearchTab) {
    if let Ok(mut active) = RESEARCH_PANEL_TAB.get_or_init(|| Mutex::new(ResearchTab::Status)).lock() {
        *active = tab;
    }
}

fn set_research_tab_status() {
    set_active_research_tab(ResearchTab::Status);
}

fn set_research_tab_research() {
    set_active_research_tab(ResearchTab::Research);
}

fn set_research_tab_conservation() {
    set_active_research_tab(ResearchTab::Conservation);
}

fn toggle_research_panel() {
    let visible = !RESEARCH_PANEL_VISIBLE.load(Ordering::Acquire);
    RESEARCH_PANEL_VISIBLE.store(visible, Ordering::Release);
    if visible {
        set_active_research_tab(ResearchTab::Status);
        hide_options_menu();
    }
}

fn hide_research_panel() {
    RESEARCH_PANEL_VISIBLE.store(false, Ordering::Release);
}

fn toggle_research_xpac_menu() {
    let open = !RESEARCH_XPAC_MENU_OPEN.load(Ordering::Acquire);
    RESEARCH_XPAC_MENU_OPEN.store(open, Ordering::Release);
}

fn close_research_xpac_menu() {
    RESEARCH_XPAC_MENU_OPEN.store(false, Ordering::Release);
}

fn research_expansion_filter() -> Option<i32> {
    *RESEARCH_EXPANSION_FILTER.get_or_init(|| Mutex::new(None)).lock().unwrap()
}

fn clear_research_expansion_filter() {
    if let Ok(mut filter) = RESEARCH_EXPANSION_FILTER.get_or_init(|| Mutex::new(None)).lock() {
        *filter = None;
    }
    close_research_xpac_menu();
}

fn set_research_expansion_filter(id: i32) {
    if let Ok(mut filter) = RESEARCH_EXPANSION_FILTER.get_or_init(|| Mutex::new(None)).lock() {
        *filter = Some(id);
    }
    close_research_xpac_menu();
}

fn research_panel_state() -> &'static Mutex<ResearchPanelState> {
    RESEARCH_PANEL_STATE.get_or_init(|| Mutex::new(ResearchPanelState::default()))
}

fn research_list_state_mut<R>(branch_index: usize, f: impl FnOnce(&mut ResearchListState) -> R) -> R {
    let mut state = research_panel_state().lock().unwrap_or_else(|err| err.into_inner());
    let list_state = if branch_index == CONSERVATION_BRANCH_INDEX {
        &mut state.conservation_list
    } else {
        &mut state.research_list
    };
    f(list_state)
}

fn research_list_state(branch_index: usize) -> ResearchListState {
    research_list_state_mut(branch_index, |state| *state)
}

fn increase_research_branch_funding() {
    let mgr = crate::globals::globals().ztresearchmgr();
    if mgr.branch_count() > RESEARCH_BRANCH_INDEX {
        mgr.branch_mut(RESEARCH_BRANCH_INDEX).increase_funding();
    }
}

fn decrease_research_branch_funding() {
    let mgr = crate::globals::globals().ztresearchmgr();
    if mgr.branch_count() > RESEARCH_BRANCH_INDEX {
        mgr.branch_mut(RESEARCH_BRANCH_INDEX).decrease_funding();
    }
}

fn increase_conservation_branch_funding() {
    let mgr = crate::globals::globals().ztresearchmgr();
    if mgr.branch_count() > CONSERVATION_BRANCH_INDEX {
        mgr.branch_mut(CONSERVATION_BRANCH_INDEX).increase_funding();
    }
}

fn decrease_conservation_branch_funding() {
    let mgr = crate::globals::globals().ztresearchmgr();
    if mgr.branch_count() > CONSERVATION_BRANCH_INDEX {
        mgr.branch_mut(CONSERVATION_BRANCH_INDEX).decrease_funding();
    }
}

fn scroll_research_list_up() {
    research_list_state_mut(RESEARCH_BRANCH_INDEX, |state| state.scroll_offset = (state.scroll_offset - RESEARCH_LIST_ROW_HEIGHT_SINGLE).max(0.0));
}

fn scroll_research_list_down() {
    research_list_state_mut(RESEARCH_BRANCH_INDEX, |state| state.scroll_offset += RESEARCH_LIST_ROW_HEIGHT_SINGLE);
}

fn scroll_conservation_list_up() {
    research_list_state_mut(CONSERVATION_BRANCH_INDEX, |state| state.scroll_offset = (state.scroll_offset - RESEARCH_LIST_ROW_HEIGHT_SINGLE).max(0.0));
}

fn scroll_conservation_list_down() {
    research_list_state_mut(CONSERVATION_BRANCH_INDEX, |state| state.scroll_offset += RESEARCH_LIST_ROW_HEIGHT_SINGLE);
}

/// Like `draw_status_meter` (the zoo/animal/guest rating meters) but always fills with
/// `ui/sharedui/statg.tga` regardless of `pct` - research progress bars are a plain green fill at
/// any completion level, not a red/yellow/green health-style meter.
fn draw_research_progress_fill(ctx: &Context, painter: &Painter, cache: &mut TextureCache, meter: Rect, pct: f32) {
    let pct = pct.clamp(0.0, 100.0);
    let fill_width = (meter.width() * pct / 100.0).round();
    if fill_width <= 0.0 {
        return;
    }

    let fill = Rect::from_min_size(meter.min, vec2(fill_width, meter.height()));
    if let Some(texture) = cache.tga(ctx, "ui/sharedui/statg.tga") {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2((fill_width / texture.size.x).min(1.0), (meter.height() / texture.size.y).min(1.0)));
        painter.image(texture.texture.id(), fill, uv, Color32::WHITE);
    } else {
        painter.rect_filled(fill, 0.0, Color32::from_rgb(66, 196, 59));
    }
}

fn program_progress_pct(program: &crate::ztresearch::ZTResearchProgram) -> f32 {
    let target = program.target_cost();
    if target <= 0.0 {
        0.0
    } else {
        (program.current_progress() / target * 100.0).clamp(0.0, 100.0)
    }
}

/// Progress-bar hover tooltip: "Done in %s day(s)" when the branch's current program has active
/// funding, or "Not Funded" when it doesn't (`days_remaining_on_program()` returns `None` for both
/// "no program selected" and "program selected but funding rate is 0" - only show anything for the
/// latter, since there's nothing useful to say about an empty bar).
fn request_research_progress_tooltip(has_program: bool, days_remaining: Option<f32>) {
    if !has_program {
        return;
    }
    let text = match days_remaining {
        Some(days) => {
            let whole_days = days.round().max(0.0) as i64;
            let template_id = if whole_days == 1 { RESEARCH_STRING_DONE_IN_ONE_DAY } else { RESEARCH_STRING_DONE_IN_DAYS };
            crate::string_registry::load_string_by_id(template_id).map(|template| template.replace("%s", &whole_days.to_string()))
        }
        None => crate::string_registry::load_string_by_id(RESEARCH_STRING_NOT_FUNDED),
    };
    if let Some(text) = text {
        request_help_tooltip(text);
    }
}

/// Icon paths (e.g. `research/strain2/strain2`) come from `ZTResearchProgram`/`ZTResearchCategory`/
/// `ZTResearchBranch` accessors as runtime `String`s, but `TextureCache`/`ButtonState` are keyed by
/// `&'static str` everywhere in this file. The set of distinct research icons is small and fixed for
/// the life of the process (~125 topics), so leaking each newly-seen path once and caching the
/// leaked pointer is simpler than threading owned/`Cow<'static, str>` keys through those APIs.
fn leaked_icon_resource(path: &str) -> &'static str {
    let cache = RESEARCH_ICON_RESOURCES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache.lock().unwrap_or_else(|err| err.into_inner());
    if let Some(existing) = cache.get(path) {
        return existing;
    }
    let leaked: &'static str = Box::leak(path.to_string().into_boxed_str());
    cache.insert(path.to_string(), leaked);
    leaked
}

/// Manual press/release-in-rect tracking for one row of the category list, mirroring
/// `draw_button_impl`'s press-then-release-while-hovered logic but scoped to a local
/// `ResearchListState` instead of the shared `ButtonState`, since list rows are dynamic in count and
/// carry a runtime category id rather than a `&'static str` key.
fn list_row_clicked(ctx: &Context, rect: Rect, row_index: usize, pressed_row: &mut Option<usize>) -> bool {
    let hovered = rect_contains_pointer(ctx, rect);
    let pressed = ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary));
    let down = ctx.input(|input| input.pointer.button_down(PointerButton::Primary));
    let released = ctx.input(|input| input.pointer.button_released(PointerButton::Primary));

    if hovered && pressed {
        *pressed_row = Some(row_index);
    }

    // Only the row that actually captured the press may read/clear `pressed_row` this frame -
    // every other row must leave it alone. Without this guard, whichever row happens to iterate
    // first each frame (row 0) would unconditionally clear the real presser's state before that
    // row's own turn came up, so only row 0 could ever complete a click.
    if *pressed_row != Some(row_index) {
        return false;
    }

    if released {
        *pressed_row = None;
        return hovered;
    }
    if !down {
        *pressed_row = None;
    }

    false
}

fn draw_research_panel(ctx: &Context, painter: &Painter, cache: &mut TextureCache, buttons: &mut ButtonState, hit_regions: &mut Vec<HitRegion>) {
    if !RESEARCH_PANEL_VISIBLE.load(Ordering::Acquire) {
        return;
    }

    let origin = pos2(17.0, 6.0);
    let active_tab = active_research_tab();
    let background = if matches!(active_tab, ResearchTab::Status) {
        "ui/research/respan/respan"
    } else {
        "ui/research/resconpn/resconpn"
    };
    draw_anim(ctx, painter, cache, hit_regions, background, origin, vec2(179.0, 440.0));

    // research.lyt's PanelTitle is statically bound to textid=4001 ("Program Status"), but that's
    // only correct for the Status tab; switching tabs should retitle the panel to the active
    // branch's name, reusing the same text ids resrch1.lyt's block headers use for "Research"/
    // "Conservation" (4040/4039 - also what the tab buttons' own helpid= already points at).
    let title_text_id = match active_tab {
        ResearchTab::Status => 4001,
        ResearchTab::Research => 4040,
        ResearchTab::Conservation => 4039,
    };
    draw_text_id(
        painter,
        title_text_id,
        Rect::from_min_size(origin + vec2(35.0, 5.0), vec2(120.0, 18.0)),
        13.0,
        OPTIONS_TITLE_TEXT,
        Align2::LEFT_CENTER,
    );

    draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        "research-close",
        "ui/sharedui/close/close",
        origin + vec2(142.0, 3.0),
        vec2(24.0, 24.0),
        false,
        true,
        Some(hide_research_panel),
        Some(3112),
    );

    draw_research_tab_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        origin + vec2(154.0, 80.0),
        "research-tab-status",
        "ui/guest/tbinfo/tbinfo",
        ResearchTab::Status,
        active_tab,
        4001,
    );
    draw_research_tab_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        origin + vec2(154.0, 114.0),
        "research-tab-research",
        "ui/research/tbreser/tbreser",
        ResearchTab::Research,
        active_tab,
        4040,
    );
    draw_research_tab_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        origin + vec2(154.0, 147.0),
        "research-tab-conservation",
        "ui/research/fund/fund",
        ResearchTab::Conservation,
        active_tab,
        4039,
    );

    match active_tab {
        ResearchTab::Status => draw_research_status_tab(ctx, painter, cache, hit_regions, origin),
        ResearchTab::Research => draw_research_list_tab(ctx, painter, cache, buttons, hit_regions, origin, RESEARCH_BRANCH_INDEX),
        ResearchTab::Conservation => draw_research_list_tab(ctx, painter, cache, buttons, hit_regions, origin, CONSERVATION_BRANCH_INDEX),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_research_tab_button(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    pos: Pos2,
    key: &'static str,
    resource: &'static str,
    tab: ResearchTab,
    active_tab: ResearchTab,
    help_id: i32,
) {
    let action = match tab {
        ResearchTab::Status => set_research_tab_status,
        ResearchTab::Research => set_research_tab_research,
        ResearchTab::Conservation => set_research_tab_conservation,
    };
    draw_explicit_button(ctx, painter, cache, buttons, hit_regions, key, resource, pos, vec2(34.0, 34.0), tab == active_tab, true, Some(action), Some(help_id));
}

fn draw_research_status_tab(ctx: &Context, painter: &Painter, cache: &mut TextureCache, hit_regions: &mut Vec<HitRegion>, origin: Pos2) {
    let mgr = crate::globals::globals().ztresearchmgr();
    if mgr.branch_count() > RESEARCH_BRANCH_INDEX {
        draw_research_branch_status_block(ctx, painter, cache, hit_regions, origin, mgr.branch(RESEARCH_BRANCH_INDEX), &RESEARCH_STATUS_BLOCK);
    }
    if mgr.branch_count() > CONSERVATION_BRANCH_INDEX {
        draw_research_branch_status_block(ctx, painter, cache, hit_regions, origin, mgr.branch(CONSERVATION_BRANCH_INDEX), &CONSERVATION_STATUS_BLOCK);
    }
}

fn draw_research_branch_status_block(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    hit_regions: &mut Vec<HitRegion>,
    origin: Pos2,
    branch: &crate::ztresearch::ZTResearchBranch,
    layout: &ResearchStatusBlockLayout,
) {
    draw_text_id(
        painter,
        layout.title_text_id,
        Rect::from_min_size(origin + vec2(38.0, layout.title_y), vec2(110.0, 16.0)),
        13.0,
        OPTIONS_SUBTITLE_TEXT,
        Align2::CENTER_CENTER,
    );

    let program_rect = Rect::from_min_size(origin + vec2(38.0, layout.program_y), vec2(110.0, 50.0));
    let current_program = branch.current_program();
    let program_name = current_program
        .as_ref()
        .and_then(|program| program.name())
        .or_else(|| crate::string_registry::load_string_by_id(RESEARCH_STRING_NO_PROGRAM))
        .unwrap_or_default();
    draw_wrapped_text(ctx, painter, &program_name, program_rect, bold_font(13.0), RESEARCH_TEXT_TEAL, Align::Center, 2);

    let progress_bg_rect = Rect::from_min_size(origin + vec2(35.0, layout.progress_bg_y), vec2(119.0, 18.0));
    draw_anim(ctx, painter, cache, hit_regions, "ui/research/progback/progback", progress_bg_rect.min, progress_bg_rect.size());
    if let Some(program) = current_program {
        let fill_rect = Rect::from_min_size(origin + vec2(40.0, layout.progress_fill_y), vec2(110.0, 12.0));
        draw_research_progress_fill(ctx, painter, cache, fill_rect, program_progress_pct(program));
    }
    if rect_contains_pointer(ctx, progress_bg_rect) {
        request_research_progress_tooltip(current_program.is_some(), branch.days_remaining_on_program());
    }

    let icon_resource = current_program
        .as_ref()
        .and_then(|program| program.icon())
        .or_else(|| branch.noprogicon())
        .map(|icon| leaked_icon_resource(&icon))
        .unwrap_or("ui/research/noprog/noprog");
    let icon_rect = Rect::from_min_size(origin + vec2(72.0, layout.icon_y), vec2(44.0, 32.0));
    draw_anim(ctx, painter, cache, hit_regions, icon_resource, icon_rect.min, icon_rect.size());
    draw_anim(
        ctx,
        painter,
        cache,
        hit_regions,
        "ui/sharedui/border/border",
        origin + vec2(71.0, layout.border_y),
        vec2(46.0, 34.0),
    );
    if rect_contains_pointer(ctx, icon_rect)
        && let Some(program) = current_program
    {
        request_help_tooltip_for_id(program.help_id());
    }
}

fn draw_research_list_tab(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    origin: Pos2,
    branch_index: usize,
) {
    let mgr = crate::globals::globals().ztresearchmgr();
    if mgr.branch_count() <= branch_index {
        return;
    }
    let branch = mgr.branch(branch_index);
    let is_conservation = branch_index == CONSERVATION_BRANCH_INDEX;

    draw_research_xpac_button(ctx, painter, cache, buttons, hit_regions, origin);

    let (minus_key, plus_key, decrease_fn, increase_fn): (&'static str, &'static str, fn(), fn()) = if is_conservation {
        (
            "conservation-funding-minus",
            "conservation-funding-plus",
            decrease_conservation_branch_funding as fn(),
            increase_conservation_branch_funding as fn(),
        )
    } else {
        (
            "research-funding-minus",
            "research-funding-plus",
            decrease_research_branch_funding as fn(),
            increase_research_branch_funding as fn(),
        )
    };

    let funding_level_count = branch.funding_levels().len() as i32;
    let current_funding_level = branch.current_funding_level();
    let can_decrease = current_funding_level > 0;
    let can_increase = current_funding_level + 1 < funding_level_count;

    draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        minus_key,
        "ui/research/minus/minus",
        origin + vec2(37.0, 259.0),
        vec2(30.0, 30.0),
        false,
        can_decrease,
        Some(decrease_fn),
        Some(4003),
    );
    draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        plus_key,
        "ui/research/plus/plus",
        origin + vec2(94.0, 259.0),
        vec2(30.0, 30.0),
        false,
        can_increase,
        Some(increase_fn),
        Some(4004),
    );

    draw_text_id(
        painter,
        4005,
        Rect::from_min_size(origin + vec2(38.0, 200.0), vec2(110.0, 30.0)),
        12.0,
        OPTIONS_SUBTITLE_TEXT,
        Align2::CENTER_CENTER,
    );

    // getFundingText's "%s" is filled with the money-formatted cost of the *current* funding level
    // (per private/resources/decompiles/ZTResearchBranch_getFundingText.c: builds the name_id template using
    // a `bfinternat::getMoneyText`-formatted string, not the branch name), so use the already-correct
    // vanilla implementation directly rather than hand-rolling the substitution.
    let funding_level_rect = Rect::from_min_size(origin + vec2(38.0, 233.0), vec2(110.0, 30.0));
    draw_wrapped_text(ctx, painter, &branch.funding_text(), funding_level_rect, bold_font(13.0), GREEN_TEXT, Align::Center, 2);
    request_help_tooltip_for_rect(ctx, funding_level_rect, 4006);

    // CategoryName/ProgramName/progress/icon mirror the branch's actual active research (same data
    // as the Status tab), not a UI-local list selection - the list below is a multi-select checklist
    // of which categories are eligible for `pick_random_program`, tracked on the categories
    // themselves via `is_enabled()`/`set_enabled`, not "which one is being viewed".
    let current_category = branch.current_category();
    let current_program = branch.current_program();

    let category_name_rect = Rect::from_min_size(origin + vec2(33.0, 303.0), vec2(122.0, 30.0));
    let category_name = current_category
        .and_then(|category| category.name())
        .or_else(|| crate::string_registry::load_string_by_id(RESEARCH_STRING_NO_CATEGORY))
        .unwrap_or_default();
    draw_wrapped_text(ctx, painter, &category_name, category_name_rect, bold_font(13.0), RESEARCH_TEXT_TEAL, Align::Center, 2);
    if rect_contains_pointer(ctx, category_name_rect)
        && let Some(category) = current_category
    {
        request_help_tooltip_for_id(category.help_id());
    }

    let program_name_rect = Rect::from_min_size(origin + vec2(33.0, 330.0), vec2(122.0, 30.0));
    let program_name = current_program
        .and_then(|program| program.name())
        .or_else(|| crate::string_registry::load_string_by_id(RESEARCH_STRING_NO_PROGRAM))
        .unwrap_or_default();
    draw_wrapped_text(ctx, painter, &program_name, program_name_rect, bold_font(13.0), RESEARCH_TEXT_GOLD, Align::Center, 2);

    let progress_bg_rect = Rect::from_min_size(origin + vec2(34.0, 400.0), vec2(119.0, 18.0));
    draw_anim(ctx, painter, cache, hit_regions, "ui/research/progback/progback", progress_bg_rect.min, progress_bg_rect.size());
    if let Some(program) = current_program {
        let fill_rect = Rect::from_min_size(origin + vec2(40.0, 402.0), vec2(110.0, 12.0));
        draw_research_progress_fill(ctx, painter, cache, fill_rect, program_progress_pct(program));
    }
    if rect_contains_pointer(ctx, progress_bg_rect) {
        request_research_progress_tooltip(current_program.is_some(), branch.days_remaining_on_program());
    }

    let icon_resource = current_program
        .and_then(|program| program.icon())
        .or_else(|| branch.noprogicon())
        .map(|icon| leaked_icon_resource(&icon))
        .unwrap_or("ui/research/noprog/noprog");
    let icon_rect = Rect::from_min_size(origin + vec2(71.0, 363.0), vec2(44.0, 32.0));
    draw_anim(ctx, painter, cache, hit_regions, icon_resource, icon_rect.min, icon_rect.size());
    draw_anim(ctx, painter, cache, hit_regions, "ui/sharedui/border/border", origin + vec2(70.0, 361.0), vec2(46.0, 34.0));
    if rect_contains_pointer(ctx, icon_rect)
        && let Some(program) = current_program
    {
        request_help_tooltip_for_id(program.help_id());
    }

    let expansion_filter = research_expansion_filter();
    let categories: Vec<_> = branch
        .categories()
        .filter(|category| expansion_filter.map_or(true, |filter| category.expansion_id() == filter))
        .collect();
    draw_research_category_list(ctx, painter, cache, buttons, hit_regions, origin, branch_index, &categories, is_conservation);

    // Drawn last so the popup renders on top of the list/funding controls it overlaps.
    if RESEARCH_XPAC_MENU_OPEN.load(Ordering::Acquire) {
        draw_research_xpac_menu(ctx, painter, cache, hit_regions, origin);
    }
}

/// `resrch2.lyt`'s `[Xpac]` button - opens `ui/xpac.lyt`'s expansion filter dropdown. Its label shows
/// the currently selected expansion (or "All").
fn draw_research_xpac_button(ctx: &Context, painter: &Painter, cache: &mut TextureCache, buttons: &mut ButtonState, hit_regions: &mut Vec<HitRegion>, origin: Pos2) {
    let button = draw_explicit_button(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        "research-xpac-button",
        "ui/sharedui/list/list",
        origin + vec2(34.0, 27.0),
        vec2(127.0, 20.0),
        RESEARCH_XPAC_MENU_OPEN.load(Ordering::Acquire),
        true,
        Some(toggle_research_xpac_menu),
        Some(22999),
    );

    let label = research_expansion_filter()
        .and_then(|id| research_expansion_options().into_iter().find(|(expansion_id, _)| *expansion_id == id))
        .map(|(_, name)| name)
        .unwrap_or_else(|| "All".to_string());
    let label_color = if rect_contains_pointer(ctx, button.rect) { OPTIONS_BUTTON_HOVER_TEXT } else { OPTIONS_BUTTON_TEXT };
    draw_wrapped_text(ctx, painter, &label, button.rect.shrink2(vec2(6.0, 2.0)), bold_font(12.0), label_color, Align::Center, 1);
}

/// `ui/xpac.lyt`'s popup: a background (`ui/sharedui/listbk/listbk`) plus an "All" row and one row
/// per `research_expansion_options()` entry. Usually a handful of rows at most, so plain
/// click-on-press detection (no shared `ButtonState`/press-tracking machinery) is enough - unlike
/// the potentially-long dynamic category list.
fn draw_research_xpac_menu(ctx: &Context, painter: &Painter, cache: &mut TextureCache, hit_regions: &mut Vec<HitRegion>, origin: Pos2) {
    let popup_rect = research_xpac_popup_rect(ctx, origin);

    // A click outside the popup (and outside the toggle button itself - its own press/release cycle
    // already opens/closes the menu, so excluding it avoids a same-press race) dismisses it without
    // applying any selection. (Escape-to-close was tried and removed: the overlay has no way to stop
    // the vanilla game from also processing the same Escape keypress - e.g. it would cancel an active
    // build tool - without a WM_CHAR-level fix in wndproc.rs. Revisit that properly if this is wanted
    // again; don't reintroduce the GetAsyncKeyState poll alone.)
    let clicked_outside = ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary))
        && ctx
            .pointer_hover_pos()
            .is_some_and(|pos| !popup_rect.contains(pos) && !research_xpac_button_rect(origin).contains(pos));
    if clicked_outside {
        close_research_xpac_menu();
        return;
    }

    // draw_anim uses the texture's *native* decoded size once loaded (fallback_size only applies if
    // the texture fails to load) - listbk's native size is a fixed 127x68, generally shorter than
    // the popup needs, so draw_anim_fixed_size (which always stretches to the given size) is
    // required here instead.
    draw_anim_fixed_size(ctx, painter, cache, hit_regions, "ui/sharedui/listbk/listbk", popup_rect.min, popup_rect.size());

    let current_filter = research_expansion_filter();
    let list_origin = popup_rect.min + vec2(2.0, 2.0);

    let row_heights = research_xpac_row_heights(ctx);
    let mut row_offsets = Vec::with_capacity(row_heights.len());
    let mut acc = 0.0;
    for height in &row_heights {
        row_offsets.push(acc);
        acc += height;
    }

    draw_research_xpac_row(ctx, painter, list_origin, row_offsets[0], row_heights[0], "All", current_filter.is_none(), clear_research_expansion_filter);

    for (i, (expansion_id, name)) in research_expansion_options().into_iter().enumerate() {
        let idx = i + 1;
        draw_research_xpac_row(ctx, painter, list_origin, row_offsets[idx], row_heights[idx], &name, current_filter == Some(expansion_id), move || {
            set_research_expansion_filter(expansion_id)
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_research_xpac_row(
    ctx: &Context,
    painter: &Painter,
    list_origin: Pos2,
    row_top_offset: f32,
    row_height: f32,
    name: &str,
    selected: bool,
    action: impl FnOnce(),
) {
    let row_rect = Rect::from_min_size(list_origin + vec2(0.0, row_top_offset), vec2(123.0, row_height));
    let hovered = rect_contains_pointer(ctx, row_rect);
    let bg_color = if selected {
        Color32::from_rgba_unmultiplied(255, 255, 255, 40)
    } else if hovered {
        Color32::from_rgba_unmultiplied(156, 205, 183, 25)
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(row_rect, 0.0, bg_color);
    draw_wrapped_text(ctx, painter, name, row_rect.shrink2(vec2(4.0, 0.0)), bold_font(12.0), RESEARCH_TEXT_TEAL, Align::LEFT, 2);

    if hovered && ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary)) {
        action();
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_research_category_list(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    origin: Pos2,
    branch_index: usize,
    categories: &[&'static crate::ztresearch::ZTResearchCategory],
    is_conservation: bool,
) {
    let list_rect = Rect::from_min_size(origin + vec2(38.0, 51.0), vec2(102.0, 148.0));
    // The expansion popup overlaps only the top portion of this list (and the up-scroll-arrow) when
    // open; gate hover/click on whatever's actually under the pointer right now rather than a blanket
    // "list is open" flag, since rows below the popup's bottom edge remain perfectly clickable.
    let popup_blocks_pointer = research_xpac_popup_blocks_pointer(ctx, origin);

    if categories.is_empty() {
        draw_anim_fixed_size(ctx, painter, cache, hit_regions, "ui/sharedui/scrolbck/scrolbck", origin + vec2(140.0, 51.0), vec2(12.0, 148.0));
        // draw_text_id paints via `Painter::text`, which never wraps - fine for the short fixed
        // button/title labels it's used for elsewhere, but this "no categories" message needs to wrap
        // within the list box's width, so it goes through `draw_wrapped_text` instead.
        if let Some(text) = crate::string_registry::load_string_by_id(11003) {
            draw_wrapped_text(
                ctx,
                painter,
                &text,
                Rect::from_min_size(origin + vec2(40.0, 51.0), vec2(110.0, 204.0)),
                bold_font(12.0),
                RESEARCH_TEXT_GOLD,
                Align::Center,
                8,
            );
        }
        return;
    }

    // Confirmed via private/resources/decompiles/_fillList.c (OOAnalyzer::UIListBox::addString's icon
    // arguments): vanilla uses two distinct single-state icons, ui/sharedui/checkbx1/checkbx1
    // (unchecked) and ui/sharedui/checkbx2/checkbx2 (checked), not a single N/H/S chrome. Both are
    // 22x18, matching resrch2.lyt's `miniconwidth = 22` - confirming the list has no separate
    // per-row topic icon, the reserved "mini icon" column *is* the checkbox.
    const CHECKBOX_SIZE: Vec2 = Vec2 { x: 22.0, y: 18.0 };
    const TEXT_LEFT_PAD: f32 = CHECKBOX_SIZE.x + 3.0;
    const TEXT_RIGHT_PAD: f32 = 4.0;

    // Each row's height depends on whether its name wraps to one or two lines (see
    // `measure_row_height`), so heights/offsets are precomputed together in one pass rather than
    // assumed uniform - this is also what fixes the scrollbar's `max_scroll` overestimating how far
    // there is to scroll.
    let text_wrap_width = (list_rect.width() - TEXT_LEFT_PAD - TEXT_RIGHT_PAD).max(0.0);
    let rows: Vec<(String, f32)> = categories
        .iter()
        .map(|category| {
            let name = category.name().unwrap_or_default();
            let height = measure_row_height(ctx, &name, bold_font(12.0), text_wrap_width);
            (name, height)
        })
        .collect();
    let mut row_offsets: Vec<f32> = Vec::with_capacity(rows.len());
    let mut acc = 0.0;
    for (_, height) in &rows {
        row_offsets.push(acc);
        acc += height;
    }
    let total_height = acc;
    let max_scroll = (total_height - list_rect.height()).max(0.0);
    research_list_state_mut(branch_index, |state| state.scroll_offset = state.scroll_offset.clamp(0.0, max_scroll));
    let scroll_offset = research_list_state(branch_index).scroll_offset;

    // Clip all row drawing to the list box so long category names can't bleed past its bounds.
    let list_painter = painter.with_clip_rect(list_rect);

    for (row_index, category) in categories.iter().enumerate() {
        let (name, row_height) = &rows[row_index];
        let row_top = list_rect.top() + row_offsets[row_index] - scroll_offset;
        if row_top + row_height < list_rect.top() || row_top > list_rect.bottom() {
            continue;
        }
        let row_rect = Rect::from_min_size(pos2(list_rect.left(), row_top), vec2(list_rect.width(), *row_height));

        // Clicking a row toggles that category's own `enabled` flag - this is a multi-select
        // checklist (any number of categories can be checked at once), persisted on the category
        // itself via `ZTResearchCategory::set_enabled`, not UI-local "which row is selected" state.
        let clicked = !popup_blocks_pointer
            && research_list_state_mut(branch_index, |state| list_row_clicked(ctx, row_rect, row_index, &mut state.pressed_row));
        if clicked {
            let currently_enabled = category.is_enabled();
            let mgr = crate::globals::globals().ztresearchmgr();
            if mgr.branch_count() > branch_index {
                mgr.branch_mut(branch_index).category_mut(row_index).set_enabled(!currently_enabled);
            }
        }

        let enabled = category.is_enabled();
        let hovered = !popup_blocks_pointer && rect_contains_pointer(ctx, row_rect);
        if hovered {
            request_help_tooltip_for_id(category.help_id());
        }

        let checkbox_pos = pos2(row_rect.left(), row_rect.center().y - CHECKBOX_SIZE.y / 2.0);
        let checkbox_resource = if enabled { "ui/sharedui/checkbx2/checkbx2" } else { "ui/sharedui/checkbx1/checkbx1" };
        draw_anim(ctx, &list_painter, cache, hit_regions, checkbox_resource, checkbox_pos, CHECKBOX_SIZE);

        let text_rect = Rect::from_min_size(
            pos2(row_rect.left() + TEXT_LEFT_PAD, row_rect.top()),
            vec2((row_rect.width() - TEXT_LEFT_PAD - TEXT_RIGHT_PAD).max(0.0), row_rect.height()),
        );
        draw_wrapped_text(ctx, &list_painter, name, text_rect, bold_font(12.0), RESEARCH_TEXT_TEAL, Align::LEFT, 2);

        if hovered {
            // Matches resrch2.lyt's `highlightbordercolor` (255, 217, 90) - vanilla's list rows don't
            // change background or text color on hover/select, only the row border. Drawn last so it
            // sits in front of the checkbox/text rather than being covered by them.
            list_painter.rect_stroke(row_rect, 0.0, Stroke::new(1.0, Color32::from_rgb(255, 217, 90)), StrokeKind::Inside);
        }
    }

    // draw_anim uses the texture's *native* decoded size once loaded (the requested size only
    // applies as a fallback if the texture fails to load) - scrolbck's native size doesn't match the
    // list box's actual height, so draw_anim_fixed_size (which always stretches to the given size) is
    // required here instead, or the background visibly overshoots the list box.
    draw_anim_fixed_size(ctx, painter, cache, hit_regions, "ui/sharedui/scrolbck/scrolbck", origin + vec2(140.0, 51.0), vec2(12.0, 148.0));

    let (up_key, down_key, up_fn, down_fn): (&'static str, &'static str, fn(), fn()) = if is_conservation {
        ("conservation-list-up", "conservation-list-down", scroll_conservation_list_up as fn(), scroll_conservation_list_down as fn())
    } else {
        ("research-list-up", "research-list-down", scroll_research_list_up as fn(), scroll_research_list_down as fn())
    };
    // Fixed-size (not `draw_explicit_button`, which renders at the texture's native decoded size):
    // these arrows are anchored to line up exactly with the list box's top/bottom edges and the
    // scrollbar track, so a native-size mismatch would make them overshoot past those edges.
    draw_explicit_button_fixed_size(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        up_key,
        "ui/sharedui/arowup/arowup",
        origin + vec2(140.0, 51.0),
        vec2(12.0, 12.0),
        false,
        scroll_offset > 0.0 && !popup_blocks_pointer,
        Some(up_fn),
        None,
    );
    draw_explicit_button_fixed_size(
        ctx,
        painter,
        cache,
        buttons,
        hit_regions,
        down_key,
        "ui/sharedui/arowdn/arowdn",
        origin + vec2(140.0, 187.0),
        vec2(12.0, 12.0),
        false,
        scroll_offset < max_scroll,
        Some(down_fn),
        None,
    );

    if max_scroll > 0.0 {
        let track_height = (list_rect.height() - 24.0).max(0.0);
        let thumb_height = 20.0_f32.min(track_height.max(1.0));
        let thumb_travel = (track_height - thumb_height).max(0.0);
        let thumb_y = list_rect.top() + 12.0 + (scroll_offset / max_scroll) * thumb_travel;
        // draw_anim_fixed_size (not draw_anim) so the thumb actually renders at the computed
        // `thumb_height` instead of the texture's native decoded size, which could otherwise spill
        // past the down arrow regardless of how far there is left to scroll.
        let thumb_drawn = draw_anim_fixed_size(ctx, painter, cache, hit_regions, "ui/sharedui/thumb/thumb", pos2(origin.x + 140.0, thumb_y), vec2(12.0, thumb_height));

        // The thumb is otherwise purely decorative (`draw_anim_fixed_size` only paints it and
        // registers a `HitRegion` for tooltip/pass-through purposes) - drag it directly to scroll,
        // proportional to how far it's moved along its travel range. No drag-start anchor is needed: `pointer.delta()`
        // already gives this frame's movement, so it accumulates onto `scroll_offset` naturally.
        let thumb_hovered = !popup_blocks_pointer && rect_contains_pointer(ctx, thumb_drawn.rect);
        let pressed = ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary));
        let down = ctx.input(|input| input.pointer.button_down(PointerButton::Primary));

        research_list_state_mut(branch_index, |state| {
            if thumb_hovered && pressed {
                state.thumb_dragging = true;
            }
            if !down {
                state.thumb_dragging = false;
            }
            if state.thumb_dragging && thumb_travel > 0.0 {
                let delta_y = ctx.input(|input| input.pointer.delta().y);
                if delta_y != 0.0 {
                    state.scroll_offset = (state.scroll_offset + delta_y / thumb_travel * max_scroll).clamp(0.0, max_scroll);
                }
            }
        });
    }
}

fn draw_minimap_cluster(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    bg2: Rect,
) {
    let zoom_level = crate::globals::globals().ztworldmgr().zoom_level();

    draw_action_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/sharedui/snap/snap",
        bg2.min + vec2(5.0, 86.0),
        vec2(34.0, 34.0),
        ButtonMode::Momentary,
        crate::ztui::click_snapshot,
    );
    draw_enabled_action_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/zoomin/zoomin",
        bg2.min + vec2(14.0, 17.0),
        vec2(28.0, 28.0),
        ButtonMode::Momentary,
        zoom_level < 2,
        crate::ztui::click_zoom_in,
    );
    draw_enabled_action_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/zoomout/zoomout",
        bg2.min + vec2(5.0, 24.0),
        vec2(28.0, 28.0),
        ButtonMode::Momentary,
        zoom_level > -2,
        crate::ztui::click_zoom_out,
    );
    draw_action_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/rotr/rotr",
        bg2.min + vec2(6.0, 40.0),
        vec2(28.0, 28.0),
        ButtonMode::Momentary,
        crate::ztui::click_rotate_cw,
    );
    draw_action_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/rotl/rotl",
        bg2.min + vec2(26.0, 27.0),
        vec2(28.0, 28.0),
        ButtonMode::Momentary,
        crate::ztui::click_rotate_ccw,
    );
    draw_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/trees/trees",
        bg2.min + vec2(147.0, 81.0),
        vec2(28.0, 28.0),
        ButtonMode::Selected,
    );
    draw_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/guests/guests",
        bg2.min + vec2(127.0, 90.0),
        vec2(28.0, 28.0),
        ButtonMode::Selected,
    );
    draw_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/builds/builds",
        bg2.min + vec2(106.0, 100.0),
        vec2(28.0, 28.0),
        ButtonMode::Selected,
    );

    let _minimap = Rect::from_min_size(bg2.min + vec2(10.0, 44.0), vec2(139.0, 69.0));
    request_help_tooltip_for_rect(ctx, Rect::from_min_size(bg2.min + vec2(10.0, 44.0), vec2(139.0, 69.0)), 1026);
}

fn draw_time_and_money(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    bg3: Rect,
    bg4: Rect,
) {
    let pause_resource = if super::pause::is_paused() {
        "ui/main/play/play"
    } else {
        "ui/main/pause/pause"
    };
    let pause = draw_action_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        pause_resource,
        bg3.min + vec2(170.0, 80.0),
        vec2(34.0, 34.0),
        ButtonMode::Momentary,
        super::pause::click_toggle_pause,
    );
    let date_rect = Rect::from_min_size(pause.rect.min + vec2(25.0, 7.0), vec2(108.0, 18.0));
    painter.text(date_rect.center(), Align2::CENTER_CENTER, super::date_display::current(), bold_font(14.0), GREEN_TEXT);
    if rect_contains_pointer(ctx, date_rect)
        && let Some(text) = super::date_display::tooltip_text()
    {
        request_help_tooltip(text);
    }

    let money_rect = Rect::from_min_size(bg4.min + vec2(90.0, 9.0), vec2(125.0, 18.0));
    let money = super::money_display::current();
    let money_color = if money.negative {
        NEGATIVE_MONEY_TEXT
    } else if money.zero {
        ZERO_MONEY_TEXT
    } else {
        GREEN_TEXT
    };
    painter.text(money_rect.center(), Align2::CENTER_CENTER, money.text, bold_font(14.0), money_color);
    request_help_tooltip_for_rect(ctx, money_rect, 1016);
}

fn draw_status_cluster(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    bg4: Rect,
    bg5: Rect,
) {
    draw_status(ctx, ui, painter, cache, buttons, hit_regions, "ui/main/zstat/zstat", bg4.min + vec2(231.0, 3.0), true);
    draw_status(ctx, ui, painter, cache, buttons, hit_regions, "ui/main/astat/astat", bg5.min + vec2(0.0, 3.0), true);
    draw_status(ctx, ui, painter, cache, buttons, hit_regions, "ui/main/gstat/gstat", bg5.min + vec2(85.0, 3.0), true);
    draw_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/hstat/hstat",
        bg5.min + vec2(170.0, 3.0),
        vec2(34.0, 34.0),
        ButtonMode::Selected,
    );
    draw_button(
        ctx,
        ui,
        painter,
        cache,
        buttons,
        hit_regions,
        "ui/main/staff/staff",
        bg5.min + vec2(206.0, 3.0),
        vec2(34.0, 34.0),
        ButtonMode::Selected,
    );
}

fn draw_status(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    button: &'static str,
    pos: Pos2,
    with_meter: bool,
) {
    if with_meter {
        draw_anim(ctx, painter, cache, hit_regions, "ui/main/progbck/progbck", pos + vec2(26.0, 5.0), vec2(56.0, 22.0));
        let meter = Rect::from_min_size(pos + vec2(32.0, 9.0), vec2(45.0, 13.0));
        draw_status_meter(ctx, painter, cache, meter, status_meter_value(button));
        if rect_contains_pointer(ctx, meter)
            && let Some(text) = status_meter_tooltip(button)
        {
            request_help_tooltip(text);
        }
    }
    draw_button(ctx, ui, painter, cache, buttons, hit_regions, button, pos, vec2(34.0, 34.0), ButtonMode::Selected);
}

fn draw_status_meter(ctx: &Context, painter: &Painter, cache: &mut TextureCache, meter: Rect, value: u8) {
    let value = value.min(100);
    let fill_width = (meter.width() * value as f32 / 100.0).round();
    if fill_width <= 0.0 {
        return;
    }

    let resource = status_meter_texture(value);
    let fill = Rect::from_min_size(meter.min, vec2(fill_width, meter.height()));
    if let Some(texture) = cache.tga(ctx, resource) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2((fill_width / texture.size.x).min(1.0), (meter.height() / texture.size.y).min(1.0)));
        painter.image(texture.texture.id(), fill, uv, Color32::WHITE);
    } else {
        painter.rect_filled(fill, 0.0, status_meter_fallback_color(value));
    }
}

fn status_meter_texture(value: u8) -> &'static str {
    match value {
        0..=24 => "ui/sharedui/statr.tga",
        25..=49 => "ui/sharedui/staty.tga",
        _ => "ui/sharedui/statg.tga",
    }
}

fn status_meter_fallback_color(value: u8) -> Color32 {
    match value {
        0..=24 => Color32::from_rgb(0xac, 0x4d, 0x2d),
        25..=49 => Color32::from_rgb(0xf6, 0xd2, 0x5b),
        _ => Color32::from_rgb(66, 196, 59),
    }
}

fn status_meter_value(resource: &str) -> u8 {
    match resource {
        "ui/main/zstat/zstat" => super::status_display::zoo_rating(),
        "ui/main/astat/astat" => super::status_display::animal_rating(),
        "ui/main/gstat/gstat" => super::status_display::guest_rating(),
        _ => 75,
    }
}

fn draw_button(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    fallback_size: Vec2,
    mode: ButtonMode,
) -> DrawnRect {
    draw_button_impl(ctx, ui, painter, cache, buttons, hit_regions, resource, resource, pos, fallback_size, mode, true, None)
}

fn draw_action_button(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    fallback_size: Vec2,
    mode: ButtonMode,
    action: fn(),
) -> DrawnRect {
    draw_button_impl(ctx, ui, painter, cache, buttons, hit_regions, resource, resource, pos, fallback_size, mode, true, Some(action))
}

fn draw_enabled_action_button(
    ctx: &Context,
    ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    fallback_size: Vec2,
    mode: ButtonMode,
    enabled: bool,
    action: fn(),
) -> DrawnRect {
    draw_button_impl(ctx, ui, painter, cache, buttons, hit_regions, resource, resource, pos, fallback_size, mode, enabled, Some(action))
}

fn draw_button_impl(
    ctx: &Context,
    _ui: &mut Ui,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    key: &'static str,
    resource: &'static str,
    pos: Pos2,
    fallback_size: Vec2,
    mode: ButtonMode,
    enabled: bool,
    action: Option<fn()>,
) -> DrawnRect {
    let normal_texture = cache.animation(ctx, resource, VisualState::Normal);
    let size = normal_texture.as_ref().map(|texture| texture.size).unwrap_or(fallback_size);
    let rect = rect_from_pos_size(pos, size);
    let hovered = enabled && button_contains_pointer(ctx, rect, normal_texture.as_ref());
    if hovered
        && let Some(help_id) = button_help_id(resource)
    {
        request_help_tooltip_for_id(help_id);
    }
    let primary_pressed = ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary));
    let primary_down = ctx.input(|input| input.pointer.button_down(PointerButton::Primary));
    let primary_released = ctx.input(|input| input.pointer.button_released(PointerButton::Primary));

    if enabled && hovered && primary_pressed {
        buttons.pressed.insert(key);
    }

    if !primary_down && !primary_released {
        buttons.pressed.remove(key);
    }

    let pressed_on_button = buttons.pressed.contains(key);
    if primary_released {
        if enabled && hovered && pressed_on_button {
            if let Some(action) = action {
                action();
            }
            if matches!(mode, ButtonMode::Selected) {
                buttons.selected.insert(key);
            }
        }
        buttons.pressed.remove(key);
    }

    let selected = buttons.selected.contains(key);
    let visual_state = if !enabled {
        VisualState::Disabled
    } else if matches!(mode, ButtonMode::Momentary) && pressed_on_button && primary_down {
        VisualState::Selected
    } else if selected {
        VisualState::Selected
    } else if hovered {
        VisualState::Hover
    } else {
        VisualState::Normal
    };

    draw_anim_state(ctx, painter, cache, hit_regions, resource, pos, size, visual_state)
}

fn draw_explicit_button(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    key: &'static str,
    resource: &'static str,
    pos: Pos2,
    fallback_size: Vec2,
    selected: bool,
    enabled: bool,
    action: Option<fn()>,
    help_id: Option<i32>,
) -> DrawnRect {
    let normal_texture = cache.animation(ctx, resource, VisualState::Normal);
    let size = normal_texture.as_ref().map(|texture| texture.size).unwrap_or(fallback_size);
    let rect = rect_from_pos_size(pos, size);
    let hovered = enabled && button_contains_pointer(ctx, rect, normal_texture.as_ref());
    if hovered
        && let Some(help_id) = help_id
    {
        request_help_tooltip_for_id(help_id);
    }

    let primary_pressed = ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary));
    let primary_down = ctx.input(|input| input.pointer.button_down(PointerButton::Primary));
    let primary_released = ctx.input(|input| input.pointer.button_released(PointerButton::Primary));

    if enabled && hovered && primary_pressed {
        buttons.pressed.insert(key);
    }

    if !primary_down && !primary_released {
        buttons.pressed.remove(key);
    }

    let pressed_on_button = buttons.pressed.contains(key);
    if primary_released {
        if enabled && hovered && pressed_on_button
            && let Some(action) = action
        {
            action();
        }
        buttons.pressed.remove(key);
    }

    let visual_state = if !enabled {
        VisualState::Disabled
    } else if pressed_on_button && primary_down {
        VisualState::Selected
    } else if selected {
        VisualState::Selected
    } else if hovered {
        VisualState::Hover
    } else {
        VisualState::Normal
    };

    draw_anim_state(ctx, painter, cache, hit_regions, resource, pos, size, visual_state)
}

/// Same as `draw_explicit_button`, but always draws/hit-tests at the given `size` instead of falling
/// back to it only when the texture fails to load - `draw_explicit_button` otherwise renders at the
/// loaded texture's *native* decoded size regardless of what's requested, which the scroll arrow
/// buttons can't tolerate: they're anchored to line up exactly with the list box's edges (and, for
/// the down arrow, with the bottom of the scrollbar track), so any native-size mismatch makes them
/// visibly overshoot past that edge.
#[allow(clippy::too_many_arguments)]
fn draw_explicit_button_fixed_size(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    buttons: &mut ButtonState,
    hit_regions: &mut Vec<HitRegion>,
    key: &'static str,
    resource: &'static str,
    pos: Pos2,
    size: Vec2,
    selected: bool,
    enabled: bool,
    action: Option<fn()>,
    help_id: Option<i32>,
) -> DrawnRect {
    let normal_texture = cache.animation(ctx, resource, VisualState::Normal);
    let rect = rect_from_pos_size(pos, size);
    let hovered = enabled && button_contains_pointer(ctx, rect, normal_texture.as_ref());
    if hovered
        && let Some(help_id) = help_id
    {
        request_help_tooltip_for_id(help_id);
    }

    let primary_pressed = ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary));
    let primary_down = ctx.input(|input| input.pointer.button_down(PointerButton::Primary));
    let primary_released = ctx.input(|input| input.pointer.button_released(PointerButton::Primary));

    if enabled && hovered && primary_pressed {
        buttons.pressed.insert(key);
    }

    if !primary_down && !primary_released {
        buttons.pressed.remove(key);
    }

    let pressed_on_button = buttons.pressed.contains(key);
    if primary_released {
        if enabled && hovered && pressed_on_button
            && let Some(action) = action
        {
            action();
        }
        buttons.pressed.remove(key);
    }

    let visual_state = if !enabled {
        VisualState::Disabled
    } else if pressed_on_button && primary_down {
        VisualState::Selected
    } else if selected {
        VisualState::Selected
    } else if hovered {
        VisualState::Hover
    } else {
        VisualState::Normal
    };

    draw_anim_state_fixed_size(ctx, painter, cache, hit_regions, resource, pos, size, visual_state)
}

fn button_help_id(resource: &str) -> Option<i32> {
    match resource {
        "ui/main/buyanim/buyanim" => Some(1000),
        "ui/main/habitat/habitat" => Some(1002),
        "ui/main/buyobj/buyobj" => Some(1001),
        "ui/main/person/person" => Some(1005),
        "ui/main/undo/undo" => Some(1075),
        "ui/main/bdoz/bdoz" => Some(1025),
        "ui/main/msgs/msgs" => Some(1006),
        "ui/main/resr/resr" => Some(1019),
        "ui/scenario/scenbut/scenbut" => Some(4107),
        "ui/main/gameopt/gameopt" => Some(1004),
        "ui/main/pause/pause" => Some(1071),
        "ui/main/play/play" => Some(1072),
        "ui/main/zoomin/zoomin" => Some(1007),
        "ui/main/zoomout/zoomout" => Some(1023),
        "ui/main/trees/trees" => Some(1066),
        "ui/main/guests/guests" => Some(1068),
        "ui/main/builds/builds" => Some(1067),
        "ui/main/rotr/rotr" => Some(1008),
        "ui/main/rotl/rotl" => Some(1009),
        "ui/main/zstat/zstat" => Some(1014),
        "ui/main/astat/astat" => Some(1010),
        "ui/main/gstat/gstat" => Some(1012),
        "ui/main/hstat/hstat" => Some(1050),
        "ui/main/staff/staff" => Some(1051),
        _ => None,
    }
}

fn status_meter_help_id(resource: &str) -> Option<i32> {
    match resource {
        "ui/main/zstat/zstat" => Some(1015),
        "ui/main/astat/astat" => Some(1011),
        "ui/main/gstat/gstat" => Some(1013),
        _ => None,
    }
}

fn status_meter_tooltip(resource: &str) -> Option<String> {
    let help_id = status_meter_help_id(resource)?;
    let value = status_meter_tooltip_value(resource)?;
    let template = super::tooltip::tooltip_text_from_id(help_id, 1)?;
    Some(format_numeric_help_text(&template, value))
}

fn status_meter_tooltip_value(resource: &str) -> Option<u32> {
    let ztgamemgr = crate::globals::globals().ztgamemgr();
    match resource {
        "ui/main/zstat/zstat" => Some(ztgamemgr.zoo_rating()),
        "ui/main/astat/astat" => Some(ztgamemgr.animal_rating_percent()),
        "ui/main/gstat/gstat" => Some(ztgamemgr.guest_rating_percent()),
        _ => None,
    }
}

fn format_numeric_help_text(template: &str, value: u32) -> String {
    if template.contains("%d") {
        template.replace("%d", &value.to_string())
    } else {
        format!("{}{}", template, value)
    }
}

fn request_help_tooltip_for_rect(ctx: &Context, rect: Rect, help_id: i32) {
    if rect_contains_pointer(ctx, rect) {
        request_help_tooltip_for_id(help_id);
    }
}

fn request_help_tooltip_for_id(help_id: i32) {
    HELP_HOVERED_THIS_FRAME.store(true, Ordering::Release);
    super::tooltip::set_overlay_help_tooltip(help_id);
}

fn request_help_tooltip(text: String) {
    HELP_HOVERED_THIS_FRAME.store(true, Ordering::Release);
    super::tooltip::set_overlay_tooltip(text);
}

fn finish_help_tooltip_frame() {
    let hovered = HELP_HOVERED_THIS_FRAME.load(Ordering::Acquire);
    let was_hovered = HELP_HOVERED_LAST_FRAME.swap(hovered, Ordering::AcqRel);
    if was_hovered && !hovered {
        super::tooltip::clear_tooltip();
    }
}

fn rect_contains_pointer(ctx: &Context, rect: Rect) -> bool {
    let Some(pos) = ctx.pointer_hover_pos().or_else(crate::ui::current_pointer_pos).or_else(crate::ui::last_pointer_pos) else {
        return false;
    };

    rect.contains(pos)
}

fn button_contains_pointer(ctx: &Context, rect: Rect, texture: Option<&LoadedTexture>) -> bool {
    let Some(pos) = ctx.pointer_hover_pos().or_else(crate::ui::current_pointer_pos).or_else(crate::ui::last_pointer_pos) else {
        return false;
    };

    if let Some(texture) = texture {
        return HitRegion {
            rect,
            uv: unit_uv(),
            mask: texture.mask.clone(),
        }
        .blocks(pos);
    }

    rect.contains(pos)
}

fn draw_anim(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    fallback_size: Vec2,
) -> DrawnRect {
    draw_anim_state(ctx, painter, cache, hit_regions, resource, pos, fallback_size, VisualState::Normal)
}

fn draw_anim_state(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    fallback_size: Vec2,
    visual_state: VisualState,
) -> DrawnRect {
    let loaded = cache
        .animation(ctx, resource, visual_state)
        .map(|texture| (texture, visual_state))
        .or_else(|| cache.animation(ctx, resource, VisualState::Normal).map(|texture| (texture, VisualState::Normal)));
    let size = loaded.as_ref().map(|(texture, _)| texture.size).unwrap_or(fallback_size);
    let normal_offset = cache.animation(ctx, resource, VisualState::Normal).map(|texture| texture.offset).unwrap_or(Vec2::ZERO);
    let state_offset = loaded.as_ref().map(|(texture, _)| texture.offset).unwrap_or(normal_offset);
    let offset_delta = normal_offset - state_offset;
    let rect = rect_from_pos_size(pos + offset_delta, size);
    let loaded_image = loaded.is_some();
    if let Some((texture, loaded_visual_state)) = loaded {
        let uv = unit_uv();
        let tint = if matches!(visual_state, VisualState::Disabled) && !matches!(loaded_visual_state, VisualState::Disabled) {
            Color32::from_white_alpha(128)
        } else {
            Color32::WHITE
        };
        painter.image(texture.texture.id(), rect, uv, tint);
        hit_regions.push(HitRegion {
            rect,
            uv,
            mask: texture.mask.clone(),
        });
    }

    DrawnRect { rect, loaded: loaded_image }
}

/// Like `draw_anim_state`, but always draws/hit-tests at the given `size` rather than the loaded
/// texture's native decoded size - the state-aware (hover/selected/disabled) counterpart to
/// `draw_anim_fixed_size`, which only covers the `Normal` state.
fn draw_anim_state_fixed_size(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    size: Vec2,
    visual_state: VisualState,
) -> DrawnRect {
    let loaded = cache
        .animation(ctx, resource, visual_state)
        .map(|texture| (texture, visual_state))
        .or_else(|| cache.animation(ctx, resource, VisualState::Normal).map(|texture| (texture, VisualState::Normal)));
    let normal_offset = cache.animation(ctx, resource, VisualState::Normal).map(|texture| texture.offset).unwrap_or(Vec2::ZERO);
    let state_offset = loaded.as_ref().map(|(texture, _)| texture.offset).unwrap_or(normal_offset);
    let offset_delta = normal_offset - state_offset;
    let rect = rect_from_pos_size(pos + offset_delta, size);
    let loaded_image = loaded.is_some();
    if let Some((texture, loaded_visual_state)) = loaded {
        let uv = unit_uv();
        let tint = if matches!(visual_state, VisualState::Disabled) && !matches!(loaded_visual_state, VisualState::Disabled) {
            Color32::from_white_alpha(128)
        } else {
            Color32::WHITE
        };
        painter.image(texture.texture.id(), rect, uv, tint);
        hit_regions.push(HitRegion {
            rect,
            uv,
            mask: texture.mask.clone(),
        });
    }

    DrawnRect { rect, loaded: loaded_image }
}

fn draw_anim_fixed_size(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    size: Vec2,
) -> DrawnRect {
    let Some(texture) = cache.animation(ctx, resource, VisualState::Normal) else {
        return DrawnRect {
            rect: rect_from_pos_size(pos, size),
            loaded: false,
        };
    };

    let rect = rect_from_pos_size(pos, size);
    let uv = unit_uv();
    painter.image(texture.texture.id(), rect, uv, Color32::WHITE);
    hit_regions.push(HitRegion {
        rect,
        uv,
        mask: texture.mask.clone(),
    });

    DrawnRect { rect, loaded: true }
}

fn draw_tiled_y(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    pos: Pos2,
    height: f32,
) {
    if height <= 0.0 {
        return;
    }

    let Some(texture) = cache.animation(ctx, resource, VisualState::Normal) else {
        return;
    };
    let tile_size = texture.size;
    if tile_size.x <= 0.0 || tile_size.y <= 0.0 {
        return;
    }

    let bottom = pos.y + height;
    let mut y = pos.y;
    while y < bottom {
        let tile_height = tile_size.y.min(bottom - y);
        let dest = Rect::from_min_size(pos2(pos.x, y), vec2(tile_size.x, tile_height));
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, tile_height / tile_size.y));
        painter.image(texture.texture.id(), dest, uv, Color32::WHITE);
        hit_regions.push(HitRegion {
            rect: dest,
            uv,
            mask: texture.mask.clone(),
        });
        y += tile_size.y;
    }
}

fn draw_tiled_x_bottom(
    ctx: &Context,
    painter: &Painter,
    cache: &mut TextureCache,
    hit_regions: &mut Vec<HitRegion>,
    resource: &'static str,
    left: f32,
    bottom: f32,
    width: f32,
) {
    if width <= 0.0 {
        return;
    }

    let Some(texture) = cache.animation(ctx, resource, VisualState::Normal) else {
        return;
    };
    let tile_size = texture.size;
    if tile_size.x <= 0.0 || tile_size.y <= 0.0 {
        return;
    }

    let top = bottom - tile_size.y;
    let right = left + width;
    let mut x = left;
    while x < right {
        let tile_width = tile_size.x.min(right - x);
        let dest = Rect::from_min_size(pos2(x, top), vec2(tile_width, tile_size.y));
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(tile_width / tile_size.x, 1.0));
        painter.image(texture.texture.id(), dest, uv, Color32::WHITE);
        hit_regions.push(HitRegion {
            rect: dest,
            uv,
            mask: texture.mask.clone(),
        });
        x += tile_size.x;
    }
}

fn texture_size(ctx: &Context, cache: &mut TextureCache, resource: &'static str, visual_state: VisualState) -> Option<Vec2> {
    cache.animation(ctx, resource, visual_state).map(|texture| texture.size)
}

fn load_animation_texture(ctx: &Context, base: &'static str, visual_state: VisualState, missing_logged: &mut bool) -> Option<LoadedTexture> {
    let descriptor_name = format!("{base}.ani");
    let Some((descriptor_source, descriptor_data)) = crate::resource_manager::lazyresourcemap::get_file(&descriptor_name) else {
        log_missing(missing_logged, &descriptor_name);
        return None;
    };

    let descriptor = String::from_utf8_lossy(&descriptor_data).into_owned();
    let mut ini = Ini::new_cs();
    if let Err(err) = ini.read(descriptor) {
        warn!("egui overlay: failed to parse animation descriptor {descriptor_name}: {err}");
        return None;
    }

    let resource_name = match animation_resource_name(&ini, visual_state) {
        Some(resource_name) => resource_name,
        None => {
            warn!("egui overlay: animation descriptor {descriptor_name} has no animation entries");
            return None;
        }
    };
    let Some((animation_source, animation_data)) = crate::resource_manager::lazyresourcemap::get_file(&resource_name) else {
        log_missing(missing_logged, &resource_name);
        return None;
    };
    let Some((palette_source, palette_data)) = animation_palette_data(base, &animation_data) else {
        log_missing(missing_logged, &format!("{base}.pal or embedded palette for {resource_name}"));
        return None;
    };

    match zt_image::decode_animation_frames(&animation_data, &palette_data) {
        Ok((animation, frames)) => {
            let Some(frame) = animation.frames.first() else {
                warn!("egui overlay: animation {resource_name} decoded with no frame metadata");
                return None;
            };
            let Some(image) = frames.into_iter().next() else {
                warn!("egui overlay: animation {resource_name} decoded with no frames");
                return None;
            };
            let size = vec2(image.size[0] as f32, image.size[1] as f32);
            let offset = vec2(frame_offset(frame.horizontal_offset_x), frame_offset(frame.vertical_offset_y));
            let mask = Arc::new(HitMask::from_image(&image));
            let texture = ctx.load_texture(format!("vanilla-main:{base}:{}", visual_state.animation_name()), image, egui::TextureOptions::NEAREST);
            info!(
                "egui overlay: loaded vanilla UI asset {base} using {descriptor_source}, {animation_source}, {palette_source} as {}x{} offset {},{}",
                size.x, size.y, offset.x, offset.y
            );
            Some(LoadedTexture { texture, size, offset, mask })
        }
        Err(err) => {
            warn!("egui overlay: failed to decode vanilla UI asset {base}: {err}");
            None
        }
    }
}

fn load_tga_texture(ctx: &Context, resource: &'static str, missing_logged: &mut bool) -> Option<LoadedTgaTexture> {
    let Some((source, data)) = crate::resource_manager::lazyresourcemap::get_file(resource) else {
        log_missing(missing_logged, resource);
        return None;
    };

    match tga::decode_tga(&data) {
        Ok(image) => {
            let size = vec2(image.size[0] as f32, image.size[1] as f32);
            let texture = ctx.load_texture(format!("vanilla-main:{resource}"), image, egui::TextureOptions::NEAREST);
            info!("egui overlay: loaded vanilla UI TGA {resource} using {source} as {}x{}", size.x, size.y);
            Some(LoadedTgaTexture { texture, size })
        }
        Err(err) => {
            warn!("egui overlay: failed to decode vanilla UI TGA {resource}: {err}");
            None
        }
    }
}

fn animation_palette_data(base: &str, animation_data: &[u8]) -> Option<(String, Box<[u8]>)> {
    let embedded_palette_name = match Animation::parse(animation_data) {
        Ok(animation) => animation,
        Err(err) => {
            warn!("egui overlay: failed to parse animation palette metadata for {base}: {err}");
            return crate::resource_manager::lazyresourcemap::get_file(&format!("{base}.pal"));
        }
    };
    let embedded_palette_name = normalize_resource_name(&embedded_palette_name.palette_filename);
    if !embedded_palette_name.is_empty()
        && let Some(palette) = crate::resource_manager::lazyresourcemap::get_file(&embedded_palette_name)
    {
        return Some(palette);
    }

    crate::resource_manager::lazyresourcemap::get_file(&format!("{base}.pal"))
}

fn normalize_resource_name(name: &str) -> String {
    name.trim_matches(char::from(0)).replace('\\', "/").to_ascii_lowercase()
}

fn frame_offset(value: u16) -> f32 {
    (value as i16) as f32
}

fn animation_resource_name(ini: &Ini, visual_state: VisualState) -> Option<String> {
    let mut dirs = Vec::new();
    for index in 0.. {
        let Some(dir) = ini.get("animation", &format!("dir{index}")) else {
            break;
        };
        dirs.push(dir);
    }

    let animations = ini.get_vec("animation", "animation")?;
    let animation = animations
        .iter()
        .find(|animation| animation.eq_ignore_ascii_case(visual_state.animation_name()))
        .or_else(|| animations.iter().find(|animation| animation.eq_ignore_ascii_case("N")))
        .or_else(|| animations.first())?;

    dirs.push(animation.clone());
    Some(dirs.join("/"))
}

fn log_missing(missing_logged: &mut bool, resource: &str) {
    if !*missing_logged {
        info!("egui overlay: vanilla UI resource not available yet: {resource}");
        *missing_logged = true;
    }
}

fn rect_from_pos_size(pos: Pos2, size: Vec2) -> Rect {
    Rect::from_min_size(pos, vec2(size.x.max(0.0), size.y.max(0.0)))
}

fn unit_uv() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
}

fn bold_font(size: f32) -> FontId {
    if BOLD_FONT_ACTIVE.load(Ordering::Acquire) {
        FontId::new(size, FontFamily::Name(BOLD_FONT_FAMILY.into()))
    } else {
        FontId::proportional(size)
    }
}

fn prepare_bold_font(ctx: &Context) {
    if BOLD_FONT_ACTIVE.load(Ordering::Acquire) {
        return;
    }

    if BOLD_FONT_REGISTERED.load(Ordering::Acquire) {
        BOLD_FONT_ACTIVE.store(true, Ordering::Release);
        return;
    }

    if register_bold_font(ctx) {
        BOLD_FONT_REGISTERED.store(true, Ordering::Release);
        ctx.request_repaint();
    }
}

fn register_bold_font(ctx: &Context) -> bool {
    let font_bytes = match std::fs::read(BOLD_FONT_PATH) {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!("egui overlay: failed to read bold UI font {BOLD_FONT_PATH}: {err}");
            return false;
        }
    };

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(BOLD_FONT_NAME.to_string(), Arc::new(FontData::from_owned(font_bytes)));
    fonts.families.insert(FontFamily::Name(BOLD_FONT_FAMILY.into()), vec![BOLD_FONT_NAME.to_string()]);

    ctx.set_fonts(fonts);
    info!("egui overlay: registered bold UI font from {BOLD_FONT_PATH}");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: usize, height: usize, pixels: Vec<Color32>) -> ColorImage {
        ColorImage::new([width, height], pixels)
    }

    #[test]
    fn hit_mask_blocks_opaque_pixels() {
        let mask = HitMask::from_image(&image(2, 1, vec![Color32::TRANSPARENT, Color32::WHITE]));

        assert!(!mask.blocks_uv(0.25, 0.5));
        assert!(mask.blocks_uv(0.75, 0.5));
    }

    #[test]
    fn hit_region_rejects_out_of_bounds_positions() {
        let region = HitRegion {
            rect: Rect::from_min_size(pos2(10.0, 20.0), vec2(5.0, 5.0)),
            uv: unit_uv(),
            mask: Arc::new(HitMask::from_image(&image(1, 1, vec![Color32::WHITE]))),
        };

        assert!(!region.blocks(pos2(9.0, 22.0)));
        assert!(!region.blocks(pos2(12.0, 26.0)));
    }

    #[test]
    fn hit_region_maps_cropped_uv_to_source_pixels() {
        let region = HitRegion {
            rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(20.0, 10.0)),
            uv: Rect::from_min_max(pos2(0.5, 0.0), pos2(1.0, 1.0)),
            mask: Arc::new(HitMask::from_image(&image(
                4,
                1,
                vec![Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::WHITE, Color32::TRANSPARENT],
            ))),
        };

        assert!(region.blocks(pos2(1.0, 5.0)));
        assert!(!region.blocks(pos2(19.0, 5.0)));
    }
}
