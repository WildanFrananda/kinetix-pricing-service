use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;

use crate::DbPool;

pub fn create_pool(database_url: &str) -> DbPool {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    return Pool::builder(config)
        .max_size(10)
        .build()
        .expect("Failed to create diesel-async deadpool connection pool");
}
