//! Dev-only commands for the fake scale panel (rule HW-10). Registered only when
//! `dev_tools::ON` is true.

use std::sync::Arc;
use std::time::Duration;

use hardware::Reading;
use pos_core::AppError;
use tauri::State;

use super::on_worker;
use crate::dev_tools::{self, FakeScaleChange, FakeScaleView};
use crate::devices::Devices;

/// How long "Read scale" waits for a stable weight, like checkout will.
const READ_LIMIT: Duration = Duration::from_secs(2);

/// What the fake scale holds now.
#[tauri::command]
pub fn dev_fake_scale_get(devices: State<'_, Arc<Devices>>) -> FakeScaleView {
    dev_tools::fake_scale_view(&devices)
}

/// Changes the fake scale's weight, stable switch and plug.
#[tauri::command]
pub fn dev_fake_scale_set(
    devices: State<'_, Arc<Devices>>,
    change: FakeScaleChange,
) -> Result<FakeScaleView, AppError> {
    dev_tools::fake_scale_set(&devices, &change)
}

/// Reads the fake scale through the same path as a real scale.
#[tauri::command]
pub async fn dev_fake_scale_read(devices: State<'_, Arc<Devices>>) -> Result<Reading, AppError> {
    let devices = Arc::clone(&devices);
    on_worker(move || dev_tools::fake_scale_read(&devices, READ_LIMIT)).await
}
