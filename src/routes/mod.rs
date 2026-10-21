pub mod allowed_methods;
pub mod catchers;
pub mod created_response;
pub mod discount_routes;
pub mod flash_sale_routes;
pub mod health_routes;
pub mod method_not_allowed;
pub mod metrics_routes;
pub mod voucher_routes;

pub use created_response::CreatedResponse;
pub use method_not_allowed::MethodNotAllowed;
