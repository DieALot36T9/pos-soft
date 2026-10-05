//! Reading and writing CSV and Excel files, and reading cell values.
//! It knows nothing about products or customers: `app` maps rows to them.
//!
//! Page: `docs/05 Modules/Module - io.md`.

mod error;
mod guard;
mod mapping;
mod parse;
mod read;
mod report;
mod table;
mod write;

pub use error::{IoError, Result};
pub use mapping::{CellErr, FieldSpec, auto_map};
pub use parse::{parse_bool, parse_date, parse_money, parse_qty};
pub use read::{read_file, sheet_names};
pub use report::report_to_xlsx;
pub use table::{ExportHeader, ExportLabels, Table, XlsxCell, XlsxSheet};
pub use write::{write_csv, write_xlsx};
