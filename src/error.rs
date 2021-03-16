pub trait IntoPlatformError {
    type TargetType: std::fmt::Debug;

    fn into_platform_error(self) -> Self::TargetType;

    fn ok() -> Self::TargetType;
}
