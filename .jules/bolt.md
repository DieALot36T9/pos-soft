## 2024-05-13 - [SQLite Search Rebuild Performance Bottleneck]
**Learning:** Rebuilding search index rows individually using `refresh_search_row` inside a loop leads to severe N+1 query performance bottlenecks.
**Action:** Use batched queries, e.g., `INSERT INTO ... SELECT` with `rusqlite::Transaction::execute_batch` to perform large-scale data copying or search index rebuilds instead of iterating in Rust.
