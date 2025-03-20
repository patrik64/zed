mod agent_profile;

use std::sync::Arc;

use ::open_ai::Model as OpenAiModel;
use anthropic::Model as AnthropicModel;
use collections::HashMap;
use deepseek::Model as DeepseekModel;
use feature_flags::FeatureFlagAppExt;
use gpui::{App, Pixels};
use language_model::{CloudModel, LanguageModel};
use lmstudio::Model as LmStudioModel;
use ollama::Model as OllamaModel;
use schemars::{schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};
use settings::{Settings, SettingsSources};

pub use crate::agent_profile::*;

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssistantDockPosition {
    Left,
    #[default]
    Right,
    Bottom,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum AssistantProviderContentV1 {
    #[serde(rename = "zed.dev")]
    ZedDotDev { default_model: Option<CloudModel> },
    #[serde(rename = "openai")]
    OpenAi {
        default_model: Option<OpenAiModel>,
        api_url: Option<String>,
        available_models: Option<Vec<OpenAiModel>>,
    },
    #[serde(rename = "anthropic")]
    Anthropic {
        default_model: Option<AnthropicModel>,
        api_url: Option<String>,
    },
    #[serde(rename = "ollama")]
    Ollama {
        default_model: Option<OllamaModel>,
        api_url: Option<String>,
    },
    #[serde(rename = "lmstudio")]
    LmStudio {
        default_model: Option<LmStudioModel>,
        api_url: Option<String>,
    },
    #[serde(rename = "deepseek")]
    DeepSeek {
        default_model: Option<DeepseekModel>,
        api_url: Option<String>,
    },
}

#[derive(Debug, Default)]
pub struct AssistantSettings {
    pub enabled: bool,
    pub button: bool,
    pub dock: AssistantDockPosition,
    pub default_width: Pixels,
    pub default_height: Pixels,
    pub default_model: LanguageModelSelection,
    pub editor_model: LanguageModelSelection,
    pub inline_alternatives: Vec<LanguageModelSelection>,
    pub using_outdated_settings_version: bool,
    pub enable_experimental_live_diffs: bool,
    pub profiles: HashMap<Arc<str>, AgentProfile>,
}

impl AssistantSettings {
    pub fn are_live_diffs_enabled(&self, cx: &App) -> bool {
        cx.is_staff() || self.enable_experimental_live_diffs
    }
}

/// Assistant panel settings
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum AssistantSettingsContent {
    Versioned(VersionedAssistantSettingsContent),
    Legacy(LegacyAssistantSettingsContent),
}

impl JsonSchema for AssistantSettingsContent {
    fn schema_name() -> String {
        VersionedAssistantSettingsContent::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        VersionedAssistantSettingsContent::json_schema(gen)
    }

    fn is_referenceable() -> bool {
        VersionedAssistantSettingsContent::is_referenceable()
    }
}

impl Default for AssistantSettingsContent {
    fn default() -> Self {
        Self::Versioned(VersionedAssistantSettingsContent::default())
    }
}

impl AssistantSettingsContent {
    pub fn is_version_outdated(&self) -> bool {
        match self {
            AssistantSettingsContent::Versioned(settings) => match settings {
                VersionedAssistantSettingsContent::V1(_) => true,
                VersionedAssistantSettingsContent::V2(_) => false,
            },
            AssistantSettingsContent::Legacy(_) => true,
        }
    }

    fn upgrade(&self) -> AssistantSettingsContentV2 {
        match self {
            AssistantSettingsContent::Versioned(settings) => match settings {
                VersionedAssistantSettingsContent::V1(settings) => AssistantSettingsContentV2 {
                    enabled: settings.enabled,
                    button: settings.button,
                    dock: settings.dock,
                    default_width: settings.default_width,
                    default_height: settings.default_width,
                    default_model: settings
                        .provider
                        .clone()
                        .and_then(|provider| match provider {
                            AssistantProviderContentV1::ZedDotDev { default_model } => {
                                default_model.map(|model| LanguageModelSelection {
                                    provider: "zed.dev".to_string(),
                                    model: model.id().to_string(),
                                })
                            }
                            AssistantProviderContentV1::OpenAi { default_model, .. } => {
                                default_model.map(|model| LanguageModelSelection {
                                    provider: "openai".to_string(),
                                    model: model.id().to_string(),
                                })
                            }
                            AssistantProviderContentV1::Anthropic { default_model, .. } => {
                                default_model.map(|model| LanguageModelSelection {
                                    provider: "anthropic".to_string(),
                                    model: model.id().to_string(),
                                })
                            }
                            AssistantProviderContentV1::Ollama { default_model, .. } => {
                                default_model.map(|model| LanguageModelSelection {
                                    provider: "ollama".to_string(),
                                    model: model.id().to_string(),
                                })
                            }
                            AssistantProviderContentV1::LmStudio { default_model, .. } => {
                                default_model.map(|model| LanguageModelSelection {
                                    provider: "lmstudio".to_string(),
                                    model: model.id().to_string(),
                                })
                            }
                            AssistantProviderContentV1::DeepSeek { default_model, .. } => {
                                default_model.map(|model| LanguageModelSelection {
                                    provider: "deepseek".to_string(),
                                    model: model.id().to_string(),
                                })
                            }
                        }),
                    editor_model: None,
                    inline_alternatives: None,
                    enable_experimental_live_diffs: None,
                    profiles: None,
                },
                VersionedAssistantSettingsContent::V2(settings) => settings.clone(),
            },
            AssistantSettingsContent::Legacy(settings) => AssistantSettingsContentV2 {
                enabled: None,
                button: settings.button,
                dock: settings.dock,
                default_width: settings.default_width,
                default_height: settings.default_height,
                default_model: Some(LanguageModelSelection {
                    provider: "openai".to_string(),
                    model: settings
                        .default_open_ai_model
                        .clone()
                        .unwrap_or_default()
                        .id()
                        .to_string(),
                }),
                editor_model: None,
                inline_alternatives: None,
                enable_experimental_live_diffs: None,
                profiles: None,
            },
        }
    }

    pub fn set_dock(&mut self, dock: AssistantDockPosition) {
        match self {
            AssistantSettingsContent::Versioned(settings) => match settings {
                VersionedAssistantSettingsContent::V1(settings) => {
                    settings.dock = Some(dock);
                }
                VersionedAssistantSettingsContent::V2(settings) => {
                    settings.dock = Some(dock);
                }
            },
            AssistantSettingsContent::Legacy(settings) => {
                settings.dock = Some(dock);
            }
        }
    }

    pub fn set_model(&mut self, language_model: Arc<dyn LanguageModel>) {
        let model = language_model.id().0.to_string();
        let provider = language_model.provider_id().0.to_string();

        match self {
            AssistantSettingsContent::Versioned(settings) => match settings {
                VersionedAssistantSettingsContent::V1(settings) => match provider.as_ref() {
                    "zed.dev" => {
                        log::warn!("attempted to set zed.dev model on outdated settings");
                    }
                    "anthropic" => {
                        let api_url = match &settings.provider {
                            Some(AssistantProviderContentV1::Anthropic { api_url, .. }) => {
                                api_url.clone()
                            }
                            _ => None,
                        };
                        settings.provider = Some(AssistantProviderContentV1::Anthropic {
                            default_model: AnthropicModel::from_id(&model).ok(),
                            api_url,
                        });
                    }
                    "ollama" => {
                        let api_url = match &settings.provider {
                            Some(AssistantProviderContentV1::Ollama { api_url, .. }) => {
                                api_url.clone()
                            }
                            _ => None,
                        };
                        settings.provider = Some(AssistantProviderContentV1::Ollama {
                            default_model: Some(ollama::Model::new(&model, None, None)),
                            api_url,
                        });
                    }
                    "lmstudio" => {
                        let api_url = match &settings.provider {
                            Some(AssistantProviderContentV1::LmStudio { api_url, .. }) => {
                                api_url.clone()
                            }
                            _ => None,
                        };
                        settings.provider = Some(AssistantProviderContentV1::LmStudio {
                            default_model: Some(lmstudio::Model::new(&model, None, None)),
                            api_url,
                        });
                    }
                    "openai" => {
                        let (api_url, available_models) = match &settings.provider {
                            Some(AssistantProviderContentV1::OpenAi {
                                api_url,
                                available_models,
                                ..
                            }) => (api_url.clone(), available_models.clone()),
                            _ => (None, None),
                        };
                        settings.provider = Some(AssistantProviderContentV1::OpenAi {
                            default_model: OpenAiModel::from_id(&model).ok(),
                            api_url,
                            available_models,
                        });
                    }
                    "deepseek" => {
                        let api_url = match &settings.provider {
                            Some(AssistantProviderContentV1::DeepSeek { api_url, .. }) => {
                                api_url.clone()
                            }
                            _ => None,
                        };
                        settings.provider = Some(AssistantProviderContentV1::DeepSeek {
                            default_model: DeepseekModel::from_id(&model).ok(),
                            api_url,
                        });
                    }
                    _ => {}
                },
                VersionedAssistantSettingsContent::V2(settings) => {
                    settings.default_model = Some(LanguageModelSelection { provider, model });
                }
            },
            AssistantSettingsContent::Legacy(settings) => {
                if let Ok(model) = OpenAiModel::from_id(&language_model.id().0) {
                    settings.default_open_ai_model = Some(model);
                }
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
#[serde(tag = "version")]
pub enum VersionedAssistantSettingsContent {
    #[serde(rename = "1")]
    V1(AssistantSettingsContentV1),
    #[serde(rename = "2")]
    V2(AssistantSettingsContentV2),
}

impl Default for VersionedAssistantSettingsContent {
    fn default() -> Self {
        Self::V2(AssistantSettingsContentV2 {
            enabled: None,
            button: None,
            dock: None,
            default_width: None,
            default_height: None,
            default_model: None,
            editor_model: None,
            inline_alternatives: None,
            enable_experimental_live_diffs: None,
            profiles: None,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct AssistantSettingsContentV2 {
    /// Whether the Assistant is enabled.
    ///
    /// Default: true
    enabled: Option<bool>,
    /// Whether to show the assistant panel button in the status bar.
    ///
    /// Default: true
    button: Option<bool>,
    /// Where to dock the assistant.
    ///
    /// Default: right
    dock: Option<AssistantDockPosition>,
    /// Default width in pixels when the assistant is docked to the left or right.
    ///
    /// Default: 640
    default_width: Option<f32>,
    /// Default height in pixels when the assistant is docked to the bottom.
    ///
    /// Default: 320
    default_height: Option<f32>,
    /// The default model to use when creating new chats.
    default_model: Option<LanguageModelSelection>,
    /// The model to use when applying edits from the assistant.
    editor_model: Option<LanguageModelSelection>,
    /// Additional models with which to generate alternatives when performing inline assists.
    inline_alternatives: Option<Vec<LanguageModelSelection>>,
    /// Enable experimental live diffs in the assistant panel.
    ///
    /// Default: false
    enable_experimental_live_diffs: Option<bool>,
    #[schemars(skip)]
    profiles: Option<HashMap<Arc<str>, AgentProfileContent>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LanguageModelSelection {
    #[schemars(schema_with = "providers_schema")]
    pub provider: String,
    pub model: String,
}

fn providers_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        enum_values: Some(vec![
            "anthropic".into(),
            "bedrock".into(),
            "google".into(),
            "lmstudio".into(),
            "ollama".into(),
            "openai".into(),
            "zed.dev".into(),
            "copilot_chat".into(),
            "deepseek".into(),
        ]),
        ..Default::default()
    }
    .into()
}

impl Default for LanguageModelSelection {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentProfileContent {
    pub name: Arc<str>,
    pub tools: HashMap<Arc<str>, bool>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct AssistantSettingsContentV1 {
    /// Whether the Assistant is enabled.
    ///
    /// Default: true
    enabled: Option<bool>,
    /// Whether to show the assistant panel button in the status bar.
    ///
    /// Default: true
    button: Option<bool>,
    /// Where to dock the assistant.
    ///
    /// Default: right
    dock: Option<AssistantDockPosition>,
    /// Default width in pixels when the assistant is docked to the left or right.
    ///
    /// Default: 640
    default_width: Option<f32>,
    /// Default height in pixels when the assistant is docked to the bottom.
    ///
    /// Default: 320
    default_height: Option<f32>,
    /// The provider of the assistant service.
    ///
    /// This can be "openai", "anthropic", "ollama", "lmstudio", "deepseek", "zed.dev"
    /// each with their respective default models and configurations.
    provider: Option<AssistantProviderContentV1>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct LegacyAssistantSettingsContent {
    /// Whether to show the assistant panel button in the status bar.
    ///
    /// Default: true
    pub button: Option<bool>,
    /// Where to dock the assistant.
    ///
    /// Default: right
    pub dock: Option<AssistantDockPosition>,
    /// Default width in pixels when the assistant is docked to the left or right.
    ///
    /// Default: 640
    pub default_width: Option<f32>,
    /// Default height in pixels when the assistant is docked to the bottom.
    ///
    /// Default: 320
    pub default_height: Option<f32>,
    /// The default OpenAI model to use when creating new chats.
    ///
    /// Default: gpt-4-1106-preview
    pub default_open_ai_model: Option<OpenAiModel>,
    /// OpenAI API base URL to use when creating new chats.
    ///
    /// Default: <https://api.openai.com/v1>
    pub openai_api_url: Option<String>,
}

impl Settings for AssistantSettings {
    const KEY: Option<&'static str> = Some("assistant");

    const PRESERVED_KEYS: Option<&'static [&'static str]> = Some(&["version"]);

    type FileContent = AssistantSettingsContent;

    fn load(
        sources: SettingsSources<Self::FileContent>,
        _: &mut gpui::App,
    ) -> anyhow::Result<Self> {
        let mut settings = AssistantSettings::default();

        for value in sources.defaults_and_customizations() {
            if value.is_version_outdated() {
                settings.using_outdated_settings_version = true;
            }

            let value = value.upgrade();
            merge(&mut settings.enabled, value.enabled);
            merge(&mut settings.button, value.button);
            merge(&mut settings.dock, value.dock);
            merge(
                &mut settings.default_width,
                value.default_width.map(Into::into),
            );
            merge(
                &mut settings.default_height,
                value.default_height.map(Into::into),
            );
            merge(&mut settings.default_model, value.default_model);
            merge(&mut settings.editor_model, value.editor_model);
            merge(&mut settings.inline_alternatives, value.inline_alternatives);
            merge(
                &mut settings.enable_experimental_live_diffs,
                value.enable_experimental_live_diffs,
            );
            merge(
                &mut settings.profiles,
                value.profiles.map(|profiles| {
                    profiles
                        .into_iter()
                        .map(|(id, profile)| {
                            (
                                id,
                                AgentProfile {
                                    name: profile.name.into(),
                                    tools: profile.tools,
                                    context_servers: HashMap::default(),
                                },
                            )
                        })
                        .collect()
                }),
            );
        }

        Ok(settings)
    }
}

fn merge<T>(target: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *target = value;
    }
}

#[cfg(test)]
mod tests {
    use fs::Fs;
    use gpui::{ReadGlobal, TestAppContext};

    use super::*;

    #[gpui::test]
    async fn test_deserialize_assistant_settings_with_version(cx: &mut TestAppContext) {
        let fs = fs::FakeFs::new(cx.executor().clone());
        fs.create_dir(paths::settings_file().parent().unwrap())
            .await
            .unwrap();

        cx.update(|cx| {
            let test_settings = settings::SettingsStore::test(cx);
            cx.set_global(test_settings);
            AssistantSettings::register(cx);
        });

        cx.update(|cx| {
            assert!(!AssistantSettings::get_global(cx).using_outdated_settings_version);
            assert_eq!(
                AssistantSettings::get_global(cx).default_model,
                LanguageModelSelection {
                    provider: "zed.dev".into(),
                    model: "claude-3-5-sonnet-latest".into(),
                }
            );
        });

        cx.update(|cx| {
            settings::SettingsStore::global(cx).update_settings_file::<AssistantSettings>(
                fs.clone(),
                |settings, _| {
                    *settings = AssistantSettingsContent::Versioned(
                        VersionedAssistantSettingsContent::V2(AssistantSettingsContentV2 {
                            default_model: Some(LanguageModelSelection {
                                provider: "test-provider".into(),
                                model: "gpt-99".into(),
                            }),
                            editor_model: Some(LanguageModelSelection {
                                provider: "test-provider".into(),
                                model: "gpt-99".into(),
                            }),
                            inline_alternatives: None,
                            enabled: None,
                            button: None,
                            dock: None,
                            default_width: None,
                            default_height: None,
                            enable_experimental_live_diffs: None,
                            profiles: None,
                        }),
                    )
                },
            );
        });

        cx.run_until_parked();

        let raw_settings_value = fs.load(paths::settings_file()).await.unwrap();
        assert!(raw_settings_value.contains(r#""version": "2""#));

        #[derive(Debug, Deserialize)]
        struct AssistantSettingsTest {
            assistant: AssistantSettingsContent,
        }

        let assistant_settings: AssistantSettingsTest =
            serde_json_lenient::from_str(&raw_settings_value).unwrap();

        assert!(!assistant_settings.assistant.is_version_outdated());
    }
}
