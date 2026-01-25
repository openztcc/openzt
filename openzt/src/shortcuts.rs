//! Keyboard shortcut registration system.
//!
//! Modules can independently register shortcuts with key+modifier combinations.
//! The HANDLE_KEY_DOWN detour checks matches before calling the original function.

use std::sync::LazyLock;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

/// Common virtual key codes for shortcuts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VkKey {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Numbers
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Special keys
    Space, Enter, Escape, Tab, Backspace,
    Insert, Delete, Home, End,
    PageUp, PageDown,
    Up, Down, Left, Right,
}

impl VkKey {
    fn to_i32(self) -> i32 {
        match self {
            Self::A => VK_A.0 as i32,
            Self::B => VK_B.0 as i32,
            Self::C => VK_C.0 as i32,
            Self::D => VK_D.0 as i32,
            Self::E => VK_E.0 as i32,
            Self::F => VK_F.0 as i32,
            Self::G => VK_G.0 as i32,
            Self::H => VK_H.0 as i32,
            Self::I => VK_I.0 as i32,
            Self::J => VK_J.0 as i32,
            Self::K => VK_K.0 as i32,
            Self::L => VK_L.0 as i32,
            Self::M => VK_M.0 as i32,
            Self::N => VK_N.0 as i32,
            Self::O => VK_O.0 as i32,
            Self::P => VK_P.0 as i32,
            Self::Q => VK_Q.0 as i32,
            Self::R => VK_R.0 as i32,
            Self::S => VK_S.0 as i32,
            Self::T => VK_T.0 as i32,
            Self::U => VK_U.0 as i32,
            Self::V => VK_V.0 as i32,
            Self::W => VK_W.0 as i32,
            Self::X => VK_X.0 as i32,
            Self::Y => VK_Y.0 as i32,
            Self::Z => VK_Z.0 as i32,
            Self::Num0 => VK_0.0 as i32,
            Self::Num1 => VK_1.0 as i32,
            Self::Num2 => VK_2.0 as i32,
            Self::Num3 => VK_3.0 as i32,
            Self::Num4 => VK_4.0 as i32,
            Self::Num5 => VK_5.0 as i32,
            Self::Num6 => VK_6.0 as i32,
            Self::Num7 => VK_7.0 as i32,
            Self::Num8 => VK_8.0 as i32,
            Self::Num9 => VK_9.0 as i32,
            Self::F1 => VK_F1.0 as i32,
            Self::F2 => VK_F2.0 as i32,
            Self::F3 => VK_F3.0 as i32,
            Self::F4 => VK_F4.0 as i32,
            Self::F5 => VK_F5.0 as i32,
            Self::F6 => VK_F6.0 as i32,
            Self::F7 => VK_F7.0 as i32,
            Self::F8 => VK_F8.0 as i32,
            Self::F9 => VK_F9.0 as i32,
            Self::F10 => VK_F10.0 as i32,
            Self::F11 => VK_F11.0 as i32,
            Self::F12 => VK_F12.0 as i32,
            Self::Space => VK_SPACE.0 as i32,
            Self::Enter => VK_RETURN.0 as i32,
            Self::Escape => VK_ESCAPE.0 as i32,
            Self::Tab => VK_TAB.0 as i32,
            Self::Backspace => VK_BACK.0 as i32,
            Self::Insert => VK_INSERT.0 as i32,
            Self::Delete => VK_DELETE.0 as i32,
            Self::Home => VK_HOME.0 as i32,
            Self::End => VK_END.0 as i32,
            Self::PageUp => VK_PRIOR.0 as i32,
            Self::PageDown => VK_NEXT.0 as i32,
            Self::Up => VK_UP.0 as i32,
            Self::Down => VK_DOWN.0 as i32,
            Self::Left => VK_LEFT.0 as i32,
            Self::Right => VK_RIGHT.0 as i32,
        }
    }
}

struct Shortcut {
    module: String,
    description: String,
    key_code: i32,
    ctrl: bool,
    shift: bool,
    alt: bool,
    callback: fn(),
}

static SHORTCUT_REGISTRY: LazyLock<std::sync::Mutex<Vec<Shortcut>>> =
    LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Register a keyboard shortcut.
///
/// # Arguments
///
/// * `module` - Name of the module registering the shortcut (for logging/debugging)
/// * `description` - Human-readable description of what the shortcut does
/// * `key` - The virtual key to bind to
/// * `ctrl` - Whether Ctrl modifier is required
/// * `shift` - Whether Shift modifier is required
/// * `alt` - Whether Alt modifier is required
/// * `override_existing` - If true, replace existing shortcuts with same key+modifiers
/// * `callback` - Function to call when shortcut is triggered (runs on game thread)
///
/// # Returns
///
/// * `Ok(())` - Successfully registered
/// * `Err(String)` - Shortcut already registered and override was false
pub fn register_shortcut(
    module: &str,
    description: &str,
    key: VkKey,
    ctrl: bool,
    shift: bool,
    alt: bool,
    override_existing: bool,
    callback: fn(),
) -> Result<(), String> {
    let key_code = key.to_i32();
    let mut registry = SHORTCUT_REGISTRY.lock().unwrap();

    // Check for existing shortcut with same key+modifiers
    let existing_idx = registry.iter().position(|s| {
        s.key_code == key_code && s.ctrl == ctrl && s.shift == shift && s.alt == alt
    });

    match (existing_idx, override_existing) {
        (Some(idx), true) => {
            tracing::warn!(
                "Shortcut override: {} replacing {} for key combo",
                module,
                registry[idx].module
            );
            registry[idx] = Shortcut {
                module: module.to_string(),
                description: description.to_string(),
                key_code,
                ctrl,
                shift,
                alt,
                callback,
            };
            Ok(())
        }
        (Some(idx), false) => Err(format!(
            "Shortcut already registered by module '{}' (use override=true to replace)",
            registry[idx].module
        )),
        (None, _) => {
            registry.push(Shortcut {
                module: module.to_string(),
                description: description.to_string(),
                key_code,
                ctrl,
                shift,
                alt,
                callback,
            });
            tracing::debug!(
                "Shortcut registered: {} - {} (Ctrl+Shift+Alt: {}{}{} key: {:#x})",
                module,
                description,
                ctrl,
                shift,
                alt,
                key_code
            );
            Ok(())
        }
    }
}

/// Check if a key press matches any registered shortcuts.
///
/// Returns the callback function if a match is found, None otherwise.
pub fn check_shortcuts(key_code: i32) -> Option<fn()> {
    // GetAsyncKeyState returns i16, check high bit (bit 15) for key state
    let ctrl_pressed = unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 } & 0x8000 != 0;
    let shift_pressed = unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 } & 0x8000 != 0;
    let alt_pressed = unsafe { GetAsyncKeyState(VK_MENU.0 as i32) as u16 } & 0x8000 != 0;

    let registry = SHORTCUT_REGISTRY.lock().unwrap();
    registry
        .iter()
        .find(|s| {
            s.key_code == key_code
                && s.ctrl == ctrl_pressed
                && s.shift == shift_pressed
                && s.alt == alt_pressed
        })
        .map(|s| {
            tracing::debug!("Shortcut triggered: {} - {}", s.module, s.description);
            s.callback
        })
}

/// List all registered shortcuts as a formatted string.
pub fn list_shortcuts() -> String {
    let registry = SHORTCUT_REGISTRY.lock().unwrap();
    if registry.is_empty() {
        return "No shortcuts registered.".to_string();
    }

    let mut result = String::from("Registered shortcuts:\n");
    for s in registry.iter() {
        let modifiers: Vec<&str> = vec![
            if s.ctrl { Some("Ctrl") } else { None },
            if s.shift { Some("Shift") } else { None },
            if s.alt { Some("Alt") } else { None },
        ]
        .into_iter()
        .flatten()
        .collect();

        let mod_str = if modifiers.is_empty() {
            String::new()
        } else {
            format!("{}+", modifiers.join("+"))
        };

        let key_name = format!("{:#x}", s.key_code);
        result.push_str(&format!(
            "  [{}{}{}] {} - {}\n",
            mod_str, key_name, if s.ctrl || s.shift || s.alt { ")" } else { "" }, s.module, s.description
        ));
    }
    result
}

/// Macro for registering keyboard shortcuts with ergonomic syntax.
///
/// # Example
///
/// ```rust
/// shortcut!(
///     "ztgamemgr",
///     "Add $10,000 to budget",
///     VkKey::F6,
///     true,   // Ctrl
///     false,  // Shift
///     false,  // Alt
///     false,  // override
///     || {
///         tracing::info!("Cash added!");
///     }
/// );
/// ```
#[macro_export]
macro_rules! shortcut {
    ($module:expr, $description:expr, $key:expr, $ctrl:expr, $shift:expr, $alt:expr, $override:expr, || $body:block) => {
        $crate::shortcuts::register_shortcut(
            $module,
            $description,
            $key,
            $ctrl,
            $shift,
            $alt,
            $override,
            || $body,
        )
        .unwrap_or_else(|e| tracing::error!("Failed to register shortcut: {}", e))
    };
}

use openzt_detour_macro::detour_mod;
use openzt_detour::gen::ztapp::HANDLE_KEY_DOWN;
use tracing::info;

#[detour_mod]
pub mod zoo_shortcuts {
    use super::*;

    #[detour(HANDLE_KEY_DOWN)]
    unsafe extern "cdecl" fn handle_key_down(param_1: i32) -> u32 {
        if let Some(callback) = super::check_shortcuts(param_1) {
            callback();
            return 0; // Don't call original function
        }
        unsafe { HANDLE_KEY_DOWN_DETOUR.call(param_1) }
    }
}

pub fn init() {
    info!("Initializing keyboard shortcut system");
    if let Err(e) = unsafe { zoo_shortcuts::init_detours() } {
        tracing::error!("Error initializing shortcut detours: {}", e);
    }

    // Example shortcut: Ctrl+R prints a test message
    shortcut!(
        "shortcuts",
        "Example: Print test message",
        VkKey::R,
        true,   // Ctrl
        false,  // Shift
        false,  // Alt
        false,  // override
        || {
            tracing::info!("Ctrl+R shortcut triggered! This is an example shortcut.");
        }
    );
}
