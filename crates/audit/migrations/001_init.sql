-- The audit log (USR-08). Rows are only ever added: the triggers below refuse
-- every UPDATE and DELETE, so nobody can change or remove an event.
CREATE TABLE audit_events (
  id             TEXT PRIMARY KEY,
  at             INTEGER NOT NULL,
  business_date  TEXT NOT NULL,
  till_id        TEXT NOT NULL,
  user_id        TEXT,                 -- NULL for system events (backup timer)
  approved_by    TEXT,                 -- override approver
  action         TEXT NOT NULL,        -- 'sale.void', 'price.change', 'drawer.no_sale', ...
  entity_type    TEXT,
  entity_id      TEXT,
  details_json   TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX audit_events_at ON audit_events(at);
CREATE INDEX audit_events_action ON audit_events(action, at);
CREATE INDEX audit_events_user ON audit_events(user_id, at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT, 'audit is append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT, 'audit is append-only'); END;
CREATE VIEW audit_v_events AS SELECT * FROM audit_events;
