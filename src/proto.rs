pub mod common {
    pub mod v1 {
        tonic::include_proto!("common.v1");
    }
}

pub mod pricing {
    pub mod v1 {
        tonic::include_proto!("pricing.v1");
    }
}

pub mod shipping {
    pub mod v1 {
        tonic::include_proto!("shipping.v1");
    }
}
