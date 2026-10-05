//! Rule HW-09: where a country's fiscal device (e.g. an EFD/SDC tax box) would plug in.
//! Not built in v1.0, so the trait has no methods and nothing uses it yet.

/// A fiscal device that signs or reports each sale to the tax office. Extension point only.
pub trait FiscalDevice: Send + Sync {}
