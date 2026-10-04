---
tags: [requirements]
modules: [auth, audit]
---
# Requirements: Users and Security (USR)

Modules: [[Module - auth]], [[Module - audit]]. Permissions: [[Roles and Permissions]].

### USR-01 Login
- The login screen shows active users as tiles. Choosing a tile asks for a PIN (cashiers) or password (admins, or anyone who chose a password)
- Given a correct PIN, then the user is logged in and goes to Checkout (or Back office if they have no open shift and choose so)

### USR-02 Lockout after failed attempts
- Given 5 wrong PIN/password attempts in a row for one user
- Then that user is locked for 60 seconds. The lock is shown with a countdown and audited
- Other users can still log in

### USR-03 Lock screen and switching users
- F12 or the idle timeout (setting) locks the till. The cart is kept
- The same user unlocks with their PIN. An admin may unlock to help
- A different cashier can't take over an open cart. They must log in after the cart is held, completed or cancelled

### USR-04 Roles
- System roles Admin and Cashier, plus custom roles with permission checkboxes and a maximum discount %

### USR-05 Admin override
- Given a cashier presses "Void sale" (needs `SaleVoid`)
- When the override dialog asks for an admin and they enter a correct password
- Then that void runs once. The audit event records `user = cashier`, `approved_by = admin`
- Then a second void needs a new override

### USR-06 Managing users
- Create a user (username, display name, role, PIN and/or password)
- Disable instead of delete (users have history)
- Reset another user's PIN/password (admin)
- Given only one active admin, when disabling them or changing their role, then it's refused

### USR-07 Change own password/PIN
- Any user can change their own PIN/password by entering the current one first

### USR-08 Audit log
- Events: login/logout/lockout, overrides, voids, returns, cancelled carts with lines, discounts over the role limit, price overrides, price/cost changes, stock adjustments, count postings, drawer no-sale, cash in/out, shift force close, settings changes, user/role changes, imports, backups, restores, logging switched on/off
- Each event: time, business date, till, user, approved-by, action, entity, details (JSON)
- Read-only, filterable by date/user/action, exportable to Excel. **Nobody can edit or delete events** (enforced by database triggers)

### USR-09 Recovery code
- Given the admin forgot their password
- When someone enters the recovery code on the login screen ("Forgot admin password")
- Then they can set a new password for one admin. A **new** recovery code is generated and shown once, and the event is audited
