---
tags: [guide, testing]
---
# TDD Workflow

## The loop
1. **Pick one acceptance criterion** (e.g. `CHK-08` weight label).
2. **Red:** write the smallest test that expresses it, using the requirement's own numbers. Run it and watch it fail **for the right reason**.
3. **Green:** write the simplest code that passes.
4. **Refactor:** clean names and duplication with all tests green. Check [[Coding Standards]].
5. Repeat for the next criterion. Commit at each green step.

## Choosing the test level
| You're building… | Start with |
|---|---|
| A calculation (tax, pricing, rounding, barcode) | A unit test of the pure function |
| A module function that touches its tables | A module test with `test_db()` |
| Something that crosses modules | A workflow test in `app/tests/` with `TestShop` |
| A screen | A Vitest component test, then a Playwright flow against the mock bridge |
| A printout | A golden-image test (the new image needs product-owner approval) |

## Example (Rust)
```rust
#[test]
fn chk_08_weight_label_adds_weighed_line() {
    let shop = TestShop::new()
        .weighed("Beef").unit("kg").price("750.00").scale_code("00123")
        .open_shift_default().build();

    let cart = shop.scan("2100123012343").unwrap();

    assert_eq!(cart.lines[0].qty, qty("1.234"));
    assert_eq!(cart.lines[0].amount, money("925.50"));
}
```

## Bugs
Every bug fix starts with a test that reproduces the bug (red), then the fix (green). The test name includes the issue number.
