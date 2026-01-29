use crate::error::FeatherGateError;
use crate::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 主配置结构
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub model_list: Vec<ModelConfig>,
}

/// 模型配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    pub model_name: String,
    pub litellm_params: LitellmParams,
}

/// Litellm 参数（兼容 litellm 格式）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LitellmParams {
    pub model: String, // 格式: provider/model-id
    pub api_key: String,
    #[serde(default = "default_api_base")]
    pub api_base: String,
}

fn default_api_base() -> String {
    String::new()
}

impl Config {
    /// 从 YAML 文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let content = Self::replace_env_vars(&content)?;
        let config: Config = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// 替换配置中的环境变量 ${VAR}
    fn replace_env_vars(content: &str) -> Result<String> {
        let re = Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();
        let mut result = content.to_string();

        for cap in re.captures_iter(content) {
            let var_name = &cap[1];
            let var_value = std::env::var(var_name).map_err(|_| {
                FeatherGateError::config(format!("环境变量未找到: {}", var_name))
            })?;
            result = result.replace(&cap[0], &var_value);
        }

        Ok(result)
    }

    /// 验证配置
    fn validate(&self) -> Result<()> {
        if self.model_list.is_empty() {
            return Err(FeatherGateError::config("model_list 不能为空"));
        }

        for model in &self.model_list {
            if model.model_name.is_empty() {
                return Err(FeatherGateError::config("model_name 不能为空"));
            }
            if model.litellm_params.model.is_empty() {
                return Err(FeatherGateError::config("model 参数不能为空"));
            }
            if model.litellm_params.api_key.is_empty() {
                return Err(FeatherGateError::config("api_key 不能为空"));
            }
        }

        Ok(())
    }

    /// 根据 model_name 查找配置
    pub fn find_model(&self, model_name: &str) -> Option<&ModelConfig> {
        self.model_list
            .iter()
            .find(|m| m.model_name == model_name)
    }
}

/// 解析模型字符串 (provider/model-id)
pub fn parse_model_string(model: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = model.split('/').collect();
    if parts.len() != 2 {
        return Err(FeatherGateError::InvalidModelString(format!(
            "期望格式 'provider/model-id'，得到: {}",
            model
        )));
    }

    let provider = parts[0].to_string();
    let model_id = parts[1].to_string();

    if provider.is_empty() || model_id.is_empty() {
        return Err(FeatherGateError::InvalidModelString(format!(
            "提供商和模型 ID 不能为空: {}",
            model
        )));
    }

    Ok((provider, model_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_model_string_valid() {
        let (provider, model_id) = parse_model_string("openai/gpt-4").unwrap();
        assert_eq!(provider, "openai");
        assert_eq!(model_id, "gpt-4");

        let (provider, model_id) = parse_model_string("anthropic/claude-opus-4-5").unwrap();
        assert_eq!(provider, "anthropic");
        assert_eq!(model_id, "claude-opus-4-5");

        let (provider, model_id) = parse_model_string("gemini/gemini-pro").unwrap();
        assert_eq!(provider, "gemini");
        assert_eq!(model_id, "gemini-pro");
    }

    #[test]
    fn test_parse_model_string_invalid() {
        assert!(parse_model_string("invalid").is_err());
        assert!(parse_model_string("too/many/parts").is_err());
        assert!(parse_model_string("/empty-provider").is_err());
        assert!(parse_model_string("empty-model/").is_err());
    }

    #[test]
    fn test_config_from_valid_yaml() {
        let yaml = r#"
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: sk-test-key
      api_base: https://api.openai.com/v1
  - model_name: claude
    litellm_params:
      model: anthropic/claude-opus-4-5
      api_key: sk-ant-test
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::from_file(file.path()).unwrap();
        assert_eq!(config.model_list.len(), 2);
        assert_eq!(config.model_list[0].model_name, "gpt-4");
        assert_eq!(config.model_list[0].litellm_params.model, "openai/gpt-4");
        assert_eq!(
            config.model_list[0].litellm_params.api_base,
            "https://api.openai.com/v1"
        );
        assert_eq!(config.model_list[1].litellm_params.api_base, ""); // 默认值
    }

    #[test]
    fn test_config_with_env_vars() {
        env::set_var("TEST_API_KEY", "sk-from-env");

        let yaml = r#"
model_list:
  - model_name: test
    litellm_params:
      model: openai/gpt-4
      api_key: ${TEST_API_KEY}
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::from_file(file.path()).unwrap();
        assert_eq!(config.model_list[0].litellm_params.api_key, "sk-from-env");

        env::remove_var("TEST_API_KEY");
    }

    #[test]
    fn test_config_missing_env_var() {
        let yaml = r#"
model_list:
  - model_name: test
    litellm_params:
      model: openai/gpt-4
      api_key: ${MISSING_VAR}
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = Config::from_file(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("MISSING_VAR"));
    }

    #[test]
    fn test_config_validation_empty_model_list() {
        let yaml = r#"
model_list: []
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = Config::from_file(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_find_model() {
        let yaml = r#"
model_list:
  - model_name: gpt-4
    litellm_params:
      model: openai/gpt-4
      api_key: sk-test
  - model_name: claude
    litellm_params:
      model: anthropic/claude-opus-4-5
      api_key: sk-ant-test
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::from_file(file.path()).unwrap();

        let model = config.find_model("gpt-4");
        assert!(model.is_some());
        assert_eq!(model.unwrap().litellm_params.model, "openai/gpt-4");

        let model = config.find_model("non-existent");
        assert!(model.is_none());
    }
}
