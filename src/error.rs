use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeatherGateError {
    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML 解析错误: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("JSON 解析错误: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP 请求错误: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("模型未找到: {0}")]
    ModelNotFound(String),

    #[error("提供商不支持: {0}")]
    UnsupportedProvider(String),

    #[error("无效的模型字符串: {0}")]
    InvalidModelString(String),

    #[error("上游 API 错误: {status} - {message}")]
    UpstreamError { status: u16, message: String },

    #[error("内部错误: {0}")]
    InternalError(String),
}

impl FeatherGateError {
    pub fn config(msg: impl Into<String>) -> Self {
        FeatherGateError::ConfigError(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        FeatherGateError::InternalError(msg.into())
    }

    pub fn upstream(status: u16, message: impl Into<String>) -> Self {
        FeatherGateError::UpstreamError {
            status,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display() {
        let err = FeatherGateError::ConfigError("测试错误".to_string());
        assert_eq!(err.to_string(), "配置错误: 测试错误");

        let err = FeatherGateError::ModelNotFound("gpt-4".to_string());
        assert_eq!(err.to_string(), "模型未找到: gpt-4");

        let err = FeatherGateError::upstream(404, "Not Found");
        assert_eq!(err.to_string(), "上游 API 错误: 404 - Not Found");
    }

    #[test]
    fn test_error_conversion_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "文件未找到");
        let err: FeatherGateError = io_err.into();
        assert!(matches!(err, FeatherGateError::IoError(_)));
    }

    #[test]
    fn test_error_conversion_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err: FeatherGateError = json_err.into();
        assert!(matches!(err, FeatherGateError::JsonError(_)));
    }

    #[test]
    fn test_convenience_constructors() {
        let err = FeatherGateError::config("配置无效");
        assert!(matches!(err, FeatherGateError::ConfigError(_)));
        assert_eq!(err.to_string(), "配置错误: 配置无效");

        let err = FeatherGateError::internal("内部错误");
        assert!(matches!(err, FeatherGateError::InternalError(_)));
    }
}
