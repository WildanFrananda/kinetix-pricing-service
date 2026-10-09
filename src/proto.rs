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

pub mod catalog {
    pub mod v1 {
        tonic::include_proto!("catalog.v1");
    }
}

pub mod identity {
    pub mod v1 {
        tonic::include_proto!("identity.v1");
    }
}
