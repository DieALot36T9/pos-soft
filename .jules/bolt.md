## 2024-05-14 - [SQLite Batch Inserts]
**Learning:** For batched inserts using SQLite, we can dynamically build the SQL with multiple placeholders `(?, ?, ?)` and use `rusqlite::params_from_iter` for optimal performance. SQLite historically limits parameters to 999 so chunk size should be kept safe (e.g. 300 chunks x 3 params = 900 params). This reduces the overhead of executing queries iteratively in a `for` loop.
**Action:** Use batch inserts for lists where significant items are inserted, like saving price lists, counting postings or imports.
