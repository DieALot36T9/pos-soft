-- Settings: one row per typed setting, stored as JSON (Module - settings).
CREATE TABLE settings_values (
  key         TEXT PRIMARY KEY,          -- e.g. 'shop_profile', 'currency', 'rounding'
  value_json  TEXT NOT NULL CHECK (json_valid(value_json)),
  updated_at  INTEGER NOT NULL,
  updated_by  TEXT                       -- user id (NULL during setup)
);

-- First-run setup: one row per wizard step that has been finished (rule SET-02).
CREATE TABLE settings_setup_steps (
  step         TEXT PRIMARY KEY CHECK (step IN
                 ('shop', 'currency', 'tax', 'admin', 'till', 'hardware', 'backup', 'products')),
  completed_at INTEGER NOT NULL
);
