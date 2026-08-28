#![recursion_limit = "256"]

pub mod config;
pub mod db;
pub mod error;
pub mod grpc;
pub mod guards;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod schema;
pub mod services;
pub mod traits;

pub type DbPool = diesel_async::pooled_connection::deadpool::Pool<diesel_async::AsyncPgConnection>;
