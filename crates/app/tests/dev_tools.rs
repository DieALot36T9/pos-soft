//! The dev-only fake scale panel (rule HW-10). These tools exist in debug builds
//! and in builds made with the `dev-tools` feature (the Mac demo build).

#![cfg(any(debug_assertions, feature = "dev-tools"))]

use std::time::Duration;

use app::dev_tools::{self, FakeScaleChange};
use app::devices::Devices;
use hardware::WeightUnit;
use pos_core::AppError;

fn change(weight: &str, stable: bool, connected: bool) -> FakeScaleChange {
    FakeScaleChange {
        weight: weight.into(),
        unit: WeightUnit::Kg,
        stable,
        connected,
    }
}

#[test]
fn hw_10_the_fake_scale_starts_empty_and_stable() {
    let devices = Devices::new(None);
    let view = dev_tools::fake_scale_view(&devices);
    assert_eq!(view.weight, "0.000");
    assert_eq!(view.unit, WeightUnit::Kg);
    assert!(view.stable);
    assert!(view.connected);
}

#[test]
fn hw_10_a_weight_set_on_the_panel_is_read_from_the_scale() {
    let devices = Devices::new(None);
    let view = dev_tools::fake_scale_set(&devices, &change("1.234", true, true)).expect("saved");
    assert_eq!(view.weight, "1.234");

    let reading =
        dev_tools::fake_scale_read(&devices, Duration::from_millis(500)).expect("a weight");

    assert_eq!(reading.value_milli, 1234);
    assert_eq!(reading.unit, WeightUnit::Kg);
    assert!(reading.stable);
}

#[test]
fn hw_10_a_moving_weight_is_read_as_not_stable() {
    let devices = Devices::new(None);
    dev_tools::fake_scale_set(&devices, &change("0.5", false, true)).expect("saved");

    let reading =
        dev_tools::fake_scale_read(&devices, Duration::from_millis(250)).expect("a weight");

    assert_eq!(reading.value_milli, 500);
    assert!(!reading.stable);
}

#[test]
fn hw_10_an_unplugged_fake_scale_times_out() {
    let devices = Devices::new(None);
    dev_tools::fake_scale_set(&devices, &change("1", true, false)).expect("saved");

    let result = dev_tools::fake_scale_read(&devices, Duration::from_millis(200));

    match result {
        Err(AppError::Hardware { device, code, .. }) => {
            assert_eq!(device, "scale");
            assert_eq!(code, "timeout");
        }
        other => panic!("expected a timeout, got {other:?}"),
    }
}

#[test]
fn hw_10_a_weight_that_is_not_a_number_is_refused() {
    let devices = Devices::new(None);

    let result = dev_tools::fake_scale_set(&devices, &change("1,2x", true, true));

    match result {
        Err(AppError::Validation { field, .. }) => assert_eq!(field.as_deref(), Some("weight")),
        other => panic!("expected a validation error, got {other:?}"),
    }
    // Nothing changed.
    assert_eq!(dev_tools::fake_scale_view(&devices).weight, "0.000");
}

#[test]
fn hw_10_a_negative_weight_shows_with_its_sign() {
    let devices = Devices::new(None);
    let view = dev_tools::fake_scale_set(&devices, &change("-0.05", true, true)).expect("saved");
    assert_eq!(view.weight, "-0.050");
}
