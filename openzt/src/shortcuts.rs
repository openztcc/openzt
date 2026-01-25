//! Keyboard shortcut registration system.
//!
//! Modules can independently register shortcuts with key+modifier combinations.
//! The HANDLE_KEY_DOWN detour checks matches before calling the original function.
//!
//! # Typestate Pattern
//!
//! Shortcuts use a typestate pattern for type-safe modifier construction:
//!
//! ```rust
//! use openzt::shortcuts::{Key, Ctrl, Shift, Alt, VkKey};
//! use windows::Win32::UI::Input::KeyboardAndMouse::*;
//!
//! // Method 1: Using VK constants directly
//! let ctrl_a = Key::<{ VK_A.0 as i32 }>::new() + Ctrl;
//!
//! // Method 2: Using VkKey::code() in a const
//! const R_KEY: i32 = VkKey::R.code();
//! let ctrl_r = Key::<R_KEY>::new() + Ctrl;
//!
//! // Key with multiple modifiers
//! let ctrl_shift_a = Key::<{ VK_A.0 as i32 }>::new() + Ctrl + Shift;
//!
//! // Register the shortcut
//! register_shortcut("module", "description", ctrl_shift_a, false, || {});
//! ```

use std::marker::PhantomData;
use std::ops::Add;
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

// ============================================================================
// Typestate Pattern for Shortcut Construction
// ============================================================================

/// Marker type for Ctrl modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ctrl;

/// Marker type for Alt modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alt;

/// Marker type for Shift modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shift;

/// Convenience constants for nicer syntax
pub const CTRL: Ctrl = Ctrl;
pub const ALT: Alt = Alt;
pub const SHIFT: Shift = Shift;

/// A key with a specific virtual key code.
///
/// This is the starting point for building shortcuts with modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key<const CODE: i32>;

impl<const CODE: i32> Key<CODE> {
    /// Create a new key with the given virtual key code.
    pub const fn new() -> Self {
        Self
    }
}

impl VkKey {
    /// Get the virtual key code as a const expression for use with `Key`.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Using VK constants directly
    /// let s1 = Key::<{ VK_A.0 as i32 }>::new() + Ctrl;
    ///
    /// // Using the convenience code() method
    /// const A_KEY: i32 = VkKey::A.code();
    /// let s2 = Key::<A_KEY>::new() + Ctrl;
    /// ```
    pub const fn code(self) -> i32 {
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

/// A complete shortcut with type-level modifier states.
///
/// The const generics encode the modifier states at compile time, preventing
/// invalid combinations and enabling efficient matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut<const CTRL: bool, const ALT: bool, const SHIFT: bool, const KEY: i32> {
    _private: PhantomData<()>,
}

impl<const CTRL: bool, const ALT: bool, const SHIFT: bool, const KEY: i32>
    Shortcut<CTRL, ALT, SHIFT, KEY>
{
    const fn new() -> Self {
        Self { _private: PhantomData }
    }

    /// Check if this shortcut matches the given key+modifier state.
    pub fn matches(&self, ctrl: bool, alt: bool, shift: bool, key: i32) -> bool {
        CTRL == ctrl && ALT == alt && SHIFT == shift && KEY == key
    }

    /// Get the key code for this shortcut.
    pub const fn key_code(&self) -> i32 {
        KEY
    }

    /// Get the Ctrl modifier state.
    pub const fn ctrl(&self) -> bool {
        CTRL
    }

    /// Get the Alt modifier state.
    pub const fn alt(&self) -> bool {
        ALT
    }

    /// Get the Shift modifier state.
    pub const fn shift(&self) -> bool {
        SHIFT
    }
}

// ----------------------------------------------------------------------------
// Add implementations for building shortcuts
// ----------------------------------------------------------------------------

// Key + Modifier -> Shortcut
impl<const CODE: i32> Add<Ctrl> for Key<CODE> {
    type Output = Shortcut<true, false, false, CODE>;

    fn add(self, _: Ctrl) -> Self::Output {
        Shortcut::new()
    }
}

impl<const CODE: i32> Add<Alt> for Key<CODE> {
    type Output = Shortcut<false, true, false, CODE>;

    fn add(self, _: Alt) -> Self::Output {
        Shortcut::new()
    }
}

impl<const CODE: i32> Add<Shift> for Key<CODE> {
    type Output = Shortcut<false, false, true, CODE>;

    fn add(self, _: Shift) -> Self::Output {
        Shortcut::new()
    }
}

// Shortcut + Modifier (adding modifiers to existing shortcuts)
impl<const ALT: bool, const SHIFT: bool, const KEY: i32> Add<Ctrl>
    for Shortcut<false, ALT, SHIFT, KEY>
{
    type Output = Shortcut<true, ALT, SHIFT, KEY>;

    fn add(self, _: Ctrl) -> Self::Output {
        Shortcut::new()
    }
}

impl<const CTRL: bool, const SHIFT: bool, const KEY: i32> Add<Alt>
    for Shortcut<CTRL, false, SHIFT, KEY>
{
    type Output = Shortcut<CTRL, true, SHIFT, KEY>;

    fn add(self, _: Alt) -> Self::Output {
        Shortcut::new()
    }
}

impl<const CTRL: bool, const ALT: bool, const KEY: i32> Add<Shift>
    for Shortcut<CTRL, ALT, false, KEY>
{
    type Output = Shortcut<CTRL, ALT, true, KEY>;

    fn add(self, _: Shift) -> Self::Output {
        Shortcut::new()
    }
}

// ============================================================================
// Shortcut Registry (Runtime storage)
// ============================================================================

/// Internal representation of a registered shortcut.
struct RegisteredShortcut {
    module: String,
    description: String,
    key_code: i32,
    ctrl: bool,
    shift: bool,
    alt: bool,
    callback: fn(),
}

/// Global registry of all registered shortcuts.
static SHORTCUT_REGISTRY: LazyLock<std::sync::Mutex<Vec<RegisteredShortcut>>> =
    LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

/// Sealed trait to restrict shortcut registration to typestate shortcuts only.
mod private {
    pub trait SealedShortcut {}
}

impl<const CTRL: bool, const ALT: bool, const SHIFT: bool, const KEY: i32>
    private::SealedShortcut for Shortcut<CTRL, ALT, SHIFT, KEY>
{
}

/// Trait for types that can be registered as shortcuts.
///
/// This is sealed to only allow `Shortcut` instances to be registered,
/// preventing runtime errors from invalid combinations.
pub trait Registerable: private::SealedShortcut {
    /// Get the key code.
    fn key_code(&self) -> i32;

    /// Get the Ctrl modifier state.
    fn ctrl(&self) -> bool;

    /// Get the Alt modifier state.
    fn alt(&self) -> bool;

    /// Get the Shift modifier state.
    fn shift(&self) -> bool;
}

impl<const CTRL: bool, const ALT: bool, const SHIFT: bool, const KEY: i32> Registerable
    for Shortcut<CTRL, ALT, SHIFT, KEY>
{
    fn key_code(&self) -> i32 {
        KEY
    }

    fn ctrl(&self) -> bool {
        CTRL
    }

    fn alt(&self) -> bool {
        ALT
    }

    fn shift(&self) -> bool {
        SHIFT
    }
}

/// Register a keyboard shortcut.
///
/// # Arguments
///
/// * `module` - Name of the module registering the shortcut (for logging/debugging)
/// * `description` - Human-readable description of what the shortcut does
/// * `shortcut` - The shortcut definition (built using typestate pattern)
/// * `override_existing` - If true, replace existing shortcuts with same key+modifiers
/// * `callback` - Function to call when shortcut is triggered (runs on game thread)
///
/// # Returns
///
/// * `Ok(())` - Successfully registered
/// * `Err(String)` - Shortcut already registered and override was false
///
/// # Example
///
/// ```rust
/// use openzt::shortcuts::{Key, Ctrl, Shift};
///
/// register_shortcut(
///     "mymodule",
///     "Do something cool",
///     Key::<{ VK_A.0 as i32 }>::new() + Ctrl + Shift,
///     false,
///     || {
///         // Handler code
///     }
/// )?;
/// ```
pub fn register_shortcut<S: Registerable>(
    module: &str,
    description: &str,
    shortcut: S,
    override_existing: bool,
    callback: fn(),
) -> Result<(), String> {
    let key_code = shortcut.key_code();
    let ctrl = shortcut.ctrl();
    let alt = shortcut.alt();
    let shift = shortcut.shift();

    let mut registry = SHORTCUT_REGISTRY.lock().unwrap();

    // Check for existing shortcut with same key+modifiers
    let existing_idx = registry
        .iter()
        .position(|s| s.key_code == key_code && s.ctrl == ctrl && s.shift == shift && s.alt == alt);

    match (existing_idx, override_existing) {
        (Some(idx), true) => {
            tracing::warn!(
                "Shortcut override: {} replacing {} for key combo",
                module,
                registry[idx].module
            );
            registry[idx] = RegisteredShortcut {
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
        (Some(_idx), false) => Err("Shortcut already registered (use override=true to replace)".to_string()),
        (None, _) => {
            registry.push(RegisteredShortcut {
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
            "  ({}{}{}) {} - {}\n",
            mod_str,
            key_name,
            if s.ctrl || s.shift || s.alt { ")" } else { "" },
            s.module,
            s.description
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
///     Key::<{ VK_F6.0 as i32 }>::new() + Ctrl,
///     false,  // override
///     || {
///         tracing::info!("Cash added!");
///     }
/// );
/// ```
#[macro_export]
macro_rules! shortcut {
    ($module:expr, $description:expr, $shortcut:expr, $override:expr, || $body:block) => {
        $crate::shortcuts::register_shortcut(
            $module,
            $description,
            $shortcut,
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
    const R_KEY: i32 = VkKey::R.code();
    shortcut!(
        "shortcuts",
        "Example: Print test message",
        Key::<R_KEY>::new() + Ctrl,
        false,  // override
        || {
            tracing::info!("Ctrl+R shortcut triggered! This is an example shortcut.");
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bare_key() {
        let k = Key::<0x41>::new();
        let s: Shortcut<true, false, false, 0x41> = k + Ctrl;
        assert!(s.matches(true, false, false, 0x41));
    }

    #[test]
    fn test_key_plus_ctrl() {
        let k = Key::<0x41>::new();
        let s = k + Ctrl;
        assert!(s.matches(true, false, false, 0x41));
        assert!(!s.matches(false, false, false, 0x41));
    }

    #[test]
    fn test_key_plus_alt() {
        let k = Key::<0x41>::new();
        let s = k + Alt;
        assert!(s.matches(false, true, false, 0x41));
    }

    #[test]
    fn test_key_plus_shift() {
        let k = Key::<0x41>::new();
        let s = k + Shift;
        assert!(s.matches(false, false, true, 0x41));
    }

    #[test]
    fn test_key_plus_ctrl_plus_shift() {
        let k = Key::<0x41>::new();
        let s = k + Ctrl + Shift;
        assert!(s.matches(true, false, true, 0x41));
    }

    #[test]
    fn test_key_plus_all_modifiers() {
        let k = Key::<0x41>::new();
        let s = k + Ctrl + Alt + Shift;
        assert!(s.matches(true, true, true, 0x41));
    }

    #[test]
    fn test_shortcut_accessors() {
        let k = Key::<0x41>::new();
        let s = k + Ctrl + Shift;
        assert_eq!(s.key_code(), 0x41);
        assert!(s.ctrl());
        assert!(!s.alt());
        assert!(s.shift());
    }

    #[test]
    fn test_vkkey_code() {
        const A_KEY: i32 = VkKey::A.code();
        let s = Key::<A_KEY>::new() + Ctrl;
        assert_eq!(s.key_code(), VK_A.0 as i32);
        assert!(s.ctrl());
    }

    #[test]
    fn test_registerable_trait() {
        let s: Shortcut<true, false, false, 0x41> = Key::<0x41>::new() + Ctrl;
        assert!(s.ctrl());
        assert!(!s.alt());
        assert!(!s.shift());
        assert_eq!(s.key_code(), 0x41);
    }
}
