-- auth: roles, permissions, users and the recovery code (Module - auth).

CREATE TABLE auth_roles (
  id                TEXT PRIMARY KEY,
  name              TEXT NOT NULL UNIQUE,
  is_system         INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0,1)),  -- Admin, Cashier
  max_discount_bps  INTEGER NOT NULL DEFAULT 0,                             -- Admin: 10000
  created_at        INTEGER NOT NULL
);

CREATE TABLE auth_role_permissions (
  role_id     TEXT NOT NULL REFERENCES auth_roles(id),
  permission  TEXT NOT NULL,             -- Permission enum, snake_case
  PRIMARY KEY (role_id, permission)
);

CREATE TABLE auth_users (
  id               TEXT PRIMARY KEY,
  username         TEXT NOT NULL UNIQUE COLLATE NOCASE,
  display_name     TEXT NOT NULL,
  role_id          TEXT NOT NULL REFERENCES auth_roles(id),
  password_hash    TEXT,                 -- argon2id PHC string
  pin_hash         TEXT,
  active           INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  failed_attempts  INTEGER NOT NULL DEFAULT 0,
  locked_until     INTEGER,
  created_at       INTEGER NOT NULL,
  updated_at       INTEGER NOT NULL,
  CHECK (password_hash IS NOT NULL OR pin_hash IS NOT NULL)
);

CREATE TABLE auth_recovery (
  id          INTEGER PRIMARY KEY CHECK (id = 1),
  code_hash   TEXT NOT NULL,
  created_at  INTEGER NOT NULL
);

-- The two system roles. Their ids are fixed so the code can find them
-- (ADMIN_ROLE_ID and CASHIER_ROLE_ID in src/roles.rs).
-- The Admin role's permissions are not stored: it always has all of them.
INSERT INTO auth_roles (id, name, is_system, max_discount_bps, created_at)
VALUES ('00000000000000000000000001', 'Admin', 1, 10000, unixepoch() * 1000),
       ('00000000000000000000000002', 'Cashier', 1, 0, unixepoch() * 1000);

-- The Cashier role's default permissions: every "C" row in Roles and Permissions.
INSERT INTO auth_role_permissions (role_id, permission)
VALUES ('00000000000000000000000002', 'checkout_sell'),
       ('00000000000000000000000002', 'checkout_cancel_cart'),
       ('00000000000000000000000002', 'checkout_hold_resume'),
       ('00000000000000000000000002', 'checkout_manual_weight'),
       ('00000000000000000000000002', 'discount_apply'),
       ('00000000000000000000000002', 'sell_on_credit'),
       ('00000000000000000000000002', 'receipt_reprint_last'),
       ('00000000000000000000000002', 'shift_open_close_own'),
       ('00000000000000000000000002', 'customer_view_create'),
       ('00000000000000000000000002', 'customer_receive_payment'),
       ('00000000000000000000000002', 'stock_view'),
       ('00000000000000000000000002', 'stock_count_enter'),
       ('00000000000000000000000002', 'labels_print'),
       ('00000000000000000000000002', 'backup_run');
