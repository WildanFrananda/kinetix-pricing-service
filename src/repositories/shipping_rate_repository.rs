use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::error::AppError;
use crate::models::ShippingRate;
use crate::repositories::traits::ShippingRateRepositoryPort;
use crate::schema::shipping_rates::dsl::*;
use crate::DbPool;

pub struct ShippingRateRepository;

#[async_trait]
impl ShippingRateRepositoryPort for ShippingRateRepository {
    async fn find_by_tier(
        &self,
        pool: &DbPool,
        tier: &str,
    ) -> Result<Option<ShippingRate>, AppError> {
        let mut conn = pool.get().await?;

        let record = shipping_rates
            .filter(service_tier.eq(tier))
            .filter(active.eq(true))
            .select(ShippingRate::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        return Ok(record);
    }
}
