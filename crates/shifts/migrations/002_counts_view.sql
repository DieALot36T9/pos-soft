-- Publish the cash counts by denomination, so the Z report can show them (SHF-08).
-- Page: docs/05 Modules/Module - shifts.md
CREATE VIEW shifts_v_counts AS SELECT shift_id, stage, denomination_minor, count FROM shifts_counts;
