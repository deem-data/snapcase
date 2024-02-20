use duckdb::Connection;
use duckdb::Row;


pub fn from_query<F>(query: &str, mut consumer: F)
    where
        F: FnMut(&Row<'_>) -> ()
{
    let duckdb = Connection::open_in_memory().unwrap();
    let mut stmt = duckdb.prepare(query).unwrap();
    let mut rows = stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        consumer(row);
    }
}