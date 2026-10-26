## 2026-10-26 - Rebuilding search with batched insertion
**Learning:** We replaced an N+1 query pattern where search indices were rebuilt by iterating through each product and running 3 queries (select, insert, insert). It's much faster to construct the search views via a single batched SQL statement for insertion!
**Action:** When a rebuild/refresh is performed over a table with tens of thousands of rows, check if SQLite's INSERT INTO ... SELECT pattern can be used instead of iterative logic.
