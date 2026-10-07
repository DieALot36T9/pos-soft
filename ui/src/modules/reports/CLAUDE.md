# Module: reports (screens)

You are working on the **Reports** screen in the back office: every report in
`docs/02 Requirements/Req - Reports.md`, shown with one generic table, printed on
receipt paper or A4 (or saved as PDF from the print dialog), and exported to Excel.
The Rust side is the `reports` crate; `app` connects it (`workflows/report_runs.rs`).

1. Read `docs/05 Modules/Module - reports.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/reports/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Every number comes from Rust in a `ReportTable`: never add up money or stock here.
4. Test on its own: `pnpm test src/modules/reports` and `pnpm e2e`.
