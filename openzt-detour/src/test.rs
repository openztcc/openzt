use std::fs;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static TEST_CONFIG: Lazy<Mutex<TestConfig>> = Lazy::new(|| {
    Mutex::new(TestConfig::load())
});

#[derive(Default)]
struct TestConfig {
    enabled_detour: Option<String>,
    success_signaled: bool,
}

impl TestConfig {
    fn load() -> Self {
        let config_path = option_env!("DETOUR_TEST_CONFIG_PATH");
        let enabled_detour = config_path
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|content| content.trim().to_string());
        
        TestConfig {
            enabled_detour,
            success_signaled: false,
        }
    }
}

pub fn is_detour_enabled(detour_name: &str) -> bool {
    let config = TEST_CONFIG.lock().unwrap();
    match &config.enabled_detour {
        Some(enabled) => enabled == detour_name,
        None => false,
    }
}

pub fn signal_detour_success(detour_name: &str) {
    let mut config = TEST_CONFIG.lock().unwrap();
    
    if config.success_signaled {
        return;
    }
    
    if let Some(success_path) = option_env!("DETOUR_TEST_SUCCESS_PATH") {
        if config.enabled_detour.as_deref() == Some(detour_name) {
            let _ = fs::write(success_path, detour_name);
            config.success_signaled = true;
        }
    }
}

#[macro_export]
macro_rules! test_detour {
    ($name:expr, $original_call:expr) => {{
        #[cfg(feature = "detour-testing")]
        {
            if $crate::test::is_detour_enabled($name) {
                $crate::test::signal_detour_success($name);
            }
        }
        $original_call
    }}
}

#[macro_export]
macro_rules! test_detour_void {
    ($name:expr, $original_call:expr) => {{
        #[cfg(feature = "detour-testing")]
        {
            if $crate::test::is_detour_enabled($name) {
                $crate::test::signal_detour_success($name);
            }
        }
        $original_call;
    }}
}