use tonic::Code;

pub const KNOWN_GRPC_METHODS: &[&str] = &[
    "pricing.v1.PricingService/CalculatePrice",
    "pricing.v1.PricingService/RedeemVoucher",
    "pricing.v1.PricingService/ReleaseVoucherRedemption",
    "pricing.v1.PricingService/AllocateFlashSaleStock",
    "pricing.v1.PricingService/ReleaseFlashSaleAllocation",
    "grpc.reflection.v1alpha.ServerReflection/ServerReflectionInfo",
];

pub const UNKNOWN_GRPC_METHOD: &str = "unknown";

pub const NO_GRPC_STATUS: &str = "NO_STATUS";

pub fn grpc_method_label(path: &str) -> &'static str {
    let dialled = path.trim_start_matches('/');
    for known in KNOWN_GRPC_METHODS {
        if *known == dialled {
            return known;
        }
    }
    return UNKNOWN_GRPC_METHOD;
}

pub fn grpc_code_label(code: Code) -> &'static str {
    return match code {
        Code::Ok => "OK",
        Code::Cancelled => "CANCELLED",
        Code::Unknown => "UNKNOWN",
        Code::InvalidArgument => "INVALID_ARGUMENT",
        Code::DeadlineExceeded => "DEADLINE_EXCEEDED",
        Code::NotFound => "NOT_FOUND",
        Code::AlreadyExists => "ALREADY_EXISTS",
        Code::PermissionDenied => "PERMISSION_DENIED",
        Code::ResourceExhausted => "RESOURCE_EXHAUSTED",
        Code::FailedPrecondition => "FAILED_PRECONDITION",
        Code::Aborted => "ABORTED",
        Code::OutOfRange => "OUT_OF_RANGE",
        Code::Unimplemented => "UNIMPLEMENTED",
        Code::Internal => "INTERNAL",
        Code::Unavailable => "UNAVAILABLE",
        Code::DataLoss => "DATA_LOSS",
        Code::Unauthenticated => "UNAUTHENTICATED",
    };
}
