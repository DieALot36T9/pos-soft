-- Shifts, the money movements of each shift, and cash counts.
-- Page: docs/05 Modules/Module - shifts.md

CREATE TABLE shifts_shifts (
  id TEXT PRIMARY KEY,
  till_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  business_date TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','closed')),
  opened_at INTEGER NOT NULL,
  opening_float_minor INTEGER NOT NULL,
  closed_at INTEGER,
  closed_by TEXT,
  forced INTEGER NOT NULL DEFAULT 0 CHECK (forced IN (0,1)),
  expected_cash_minor INTEGER,
  counted_cash_minor INTEGER,
  difference_minor INTEGER,
  close_note TEXT
);

-- Rule SHF-02: only one open shift per till.
CREATE UNIQUE INDEX shifts_one_open_per_till ON shifts_shifts(till_id) WHERE status = 'open';

-- A ledger: rows are only ever added, never changed or removed (ADR-010).
CREATE TABLE shifts_movements (
  id TEXT PRIMARY KEY,
  shift_id TEXT NOT NULL REFERENCES shifts_shifts(id),
  kind TEXT NOT NULL CHECK (kind IN ('float','sale_payment','refund','cash_in','cash_out','cash_drop','customer_payment','supplier_payment','void_reversal')),
  method_id TEXT,
  method_kind TEXT NOT NULL,
  amount_minor INTEGER NOT NULL,
  ref_type TEXT,
  ref_id TEXT,
  reason TEXT,
  at INTEGER NOT NULL,
  user_id TEXT NOT NULL
);
CREATE INDEX shifts_movements_shift ON shifts_movements(shift_id);
CREATE INDEX shifts_movements_ref ON shifts_movements(shift_id, ref_type, ref_id);

CREATE TRIGGER shifts_movements_no_update BEFORE UPDATE ON shifts_movements
BEGIN
  SELECT RAISE(ABORT, 'shifts_movements is append-only');
END;
CREATE TRIGGER shifts_movements_no_delete BEFORE DELETE ON shifts_movements
BEGIN
  SELECT RAISE(ABORT, 'shifts_movements is append-only');
END;

CREATE TABLE shifts_counts (
  shift_id TEXT NOT NULL REFERENCES shifts_shifts(id),
  stage TEXT NOT NULL CHECK (stage IN ('open','close')),
  denomination_minor INTEGER NOT NULL,
  count INTEGER NOT NULL,
  PRIMARY KEY (shift_id, stage, denomination_minor)
);

CREATE VIEW shifts_v_shifts AS SELECT * FROM shifts_shifts;
CREATE VIEW shifts_v_movements AS
  SELECT m.*, s.business_date, s.till_id
  FROM shifts_movements m JOIN shifts_shifts s ON s.id = m.shift_id;
