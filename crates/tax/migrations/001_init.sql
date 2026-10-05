-- Tax rates and their display components (Module - tax, TAX-02, TAX-03, TAX-10).
-- Released migrations are never edited. Changes go in a new file.

CREATE TABLE tax_rates (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  rate_bps    INTEGER NOT NULL CHECK (rate_bps BETWEEN 0 AND 10000),
  active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  sort        INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

-- "Components add up to the rate" is checked in Rust when saving.
CREATE TABLE tax_rate_components (
  id           TEXT PRIMARY KEY,
  tax_rate_id  TEXT NOT NULL REFERENCES tax_rates(id),
  name         TEXT NOT NULL,
  rate_bps     INTEGER NOT NULL,
  sort         INTEGER NOT NULL
);
CREATE INDEX tax_rate_components_by_rate ON tax_rate_components (tax_rate_id, sort);

-- Rule TAX-10: a rate is never deleted, only deactivated. Old sales still point to it.
CREATE TRIGGER tax_rates_never_deleted BEFORE DELETE ON tax_rates
BEGIN
  SELECT RAISE(ABORT, 'tax rates are never deleted, only deactivated');
END;

CREATE VIEW tax_v_rates AS SELECT id, name, rate_bps, active FROM tax_rates;
