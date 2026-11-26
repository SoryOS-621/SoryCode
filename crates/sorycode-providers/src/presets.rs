use crate::{GeminiProvider, OpenAiCompatProvider, Provider, ProviderManager};

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub base_url: &'static str,
    pub env_key: &'static str,
    pub default_model: &'static str,
    pub free_only: bool,
}

pub const PROVIDER_PRESETS: &[ProviderInfo] = &[
    ProviderInfo {
        name: "openai",
        description: "OpenAI",
        base_url: "https://api.openai.com/v1",
        env_key: "OPENAI_API_KEY",
        default_model: "gpt-4o-mini",
        free_only: false,
    },
    ProviderInfo {
        name: "gemini",
        description: "Google Gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta",
        env_key: "GEMINI_API_KEY",
        default_model: "gemini-2.0-flash",
        free_only: false,
    },
    ProviderInfo {
        name: "mistral",
        description: "Mistral AI",
        base_url: "https://api.mistral.ai/v1",
        env_key: "MISTRAL_API_KEY",
        default_model: "mistral-small-latest",
        free_only: false,
    },
    ProviderInfo {
        name: "groq",
        description: "Groq",
        base_url: "https://api.groq.com/openai/v1",
        env_key: "GROQ_API_KEY",
        default_model: "llama-3.3-70b-versatile",
        free_only: false,
    },
    ProviderInfo {
        name: "nvidia",
        description: "NVIDIA NIM",
        base_url: "https://integrate.api.nvidia.com/v1",
        env_key: "NVIDIA_API_KEY",
        default_model: "nvidia/nemotron-3-super-120b-a12b",
        free_only: false,
    },
    ProviderInfo {
        name: "openrouter",
        description: "OpenRouter (all models)",
        base_url: "https://openrouter.ai/api/v1",
        env_key: "OPENROUTER_API_KEY",
        default_model: "openai/gpt-4o-mini",
        free_only: false,
    },
    ProviderInfo {
        name: "openrouter-free",
        description: "OpenRouter Free models",
        base_url: "https://openrouter.ai/api/v1",
        env_key: "OPENROUTER_API_KEY",
        default_model: "openrouter/free",
        free_only: true,
    },
    ProviderInfo {
        name: "opencode-zen",
        description: "OpenCode Zen (free models)",
        base_url: "https://opencode.ai/zen/v1",
        env_key: "OPENCODE_ZEN_API_KEY",
        default_model: "big-pickle",
        free_only: true,
    },
    ProviderInfo {
        name: "github",
        description: "GitHub Models",
        base_url: "https://models.github.ai/inference",
        env_key: "GITHUB_TOKEN",
        default_model: "openai/gpt-4o-mini",
        free_only: false,
    },
];

pub fn build_provider_manager() -> (ProviderManager, Vec<&'static str>) {
    let mut pm = ProviderManager::new();
    let mut available = Vec::new();
    for p in PROVIDER_PRESETS {
        let key = std::env::var(p.env_key).unwrap_or_default();
        // For free providers, allow empty key (anonymous) but still register if possible
        let register = if p.free_only { true } else { !key.is_empty() };
        if !register {
            continue;
        }
        let api_key = if key.is_empty() {
            "public".to_string()
        } else {
            key.clone()
        };
        let provider: Box<dyn Provider> = if p.name == "gemini" {
            Box::new(GeminiProvider::new(api_key))
        } else {
            let mut prov = OpenAiCompatProvider::new(p.name, p.base_url, api_key);
            // OpenRouter recommended headers
            if p.name.starts_with("openrouter") {
                prov = prov.with_headers(vec![
                    (
                        "HTTP-Referer".to_string(),
                        "https://sorycode.local".to_string(),
                    ),
                    ("X-Title".to_string(), "SoryCode".to_string()),
                ]);
            }
            // OpenCode Zen requires x-opencode-session header
            if p.name == "opencode-zen" {
                prov = prov.with_headers(vec![
                    (
                        "x-opencode-session".to_string(),
                        "sorycode-session".to_string(),
                    ),
                    ("User-Agent".to_string(), "opencode/1.0.0".to_string()),
                ]);
            }
            Box::new(prov)
        };
        pm.register(p.name, provider);
        available.push(p.name);
    }
    (pm, available)
}

pub fn find_preset(name: &str) -> Option<&'static ProviderInfo> {
    PROVIDER_PRESETS.iter().find(|p| p.name == name)
}

/// Build a provider instance from a preset name and an API key.
///
/// This is the single construction path shared by every client (CLI, desktop,
/// future SDKs). It keeps provider-specific headers in one place.
pub fn create_provider(name: &str, api_key: &str) -> Option<Box<dyn Provider>> {
    let preset = find_preset(name)?;
    let key = if api_key.is_empty() {
        "public".to_string()
    } else {
        api_key.to_string()
    };

    if preset.name == "gemini" {
        return Some(Box::new(GeminiProvider::new(key)));
    }

    let mut provider = OpenAiCompatProvider::new(preset.name, preset.base_url, key);
    if preset.name.starts_with("openrouter") {
        provider = provider.with_headers(vec![
            (
                "HTTP-Referer".to_string(),
                "https://sorycode.local".to_string(),
            ),
            ("X-Title".to_string(), "SoryCode".to_string()),
        ]);
    }
    if preset.name == "opencode-zen" {
        provider = provider.with_headers(vec![
            (
                "x-opencode-session".to_string(),
                "sorycode-session".to_string(),
            ),
            ("User-Agent".to_string(), "opencode/1.0.0".to_string()),
        ]);
    }
    Some(Box::new(provider))
}

const OPENAI_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4.1",
    "gpt-4.1-mini",
    "o3-mini",
];
const GEMINI_MODELS: &[&str] = &[
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.0-flash",
    "gemini-1.5-pro",
];
const MISTRAL_MODELS: &[&str] = &[
    "mistral-large-latest",
    "mistral-small-latest",
    "codestral-latest",
    "pixtral-large-latest",
];
const GROQ_MODELS: &[&str] = &[
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
    "qwen/qwen3-32b",
];
const NVIDIA_MODELS: &[&str] = &[
    "nvidia/nemotron-3-super-120b-a12b",
    "nvidia/nemotron-3-ultra-550b-a55b",
    "meta/llama-3.3-70b-instruct",
    "deepseek-ai/deepseek-v4-flash",
    "openai/gpt-oss-120b",
    "moonshotai/kimi-k2-instruct",
    "qwen/qwen3-next-80b-a3b-instruct",
];
const OPENROUTER_MODELS: &[&str] = &[
    "openai/gpt-4o-mini",
    "anthropic/claude-3.5-sonnet",
    "google/gemini-2.0-flash-001",
    "meta-llama/llama-3.3-70b-instruct",
];
const OPENROUTER_FREE_MODELS: &[&str] = &[
    "openrouter/free",
    "meta-llama/llama-3.3-70b-instruct:free",
    "google/gemma-4-26b-a4b-it:free",
    "nvidia/nemotron-3-super-120b-a12b:free",
    "qwen/qwen3.8-27b:free",
];
const OPENCODE_ZEN_MODELS: &[&str] = &[
    "big-pickle",
    "mimo-v2.6-flash-free",
    "deepseek-v4.1-flash-free",
    "glm-5.3-flash-free",
    "nemotron-3.5-lightning-free",
    "longcat-2.5-preview-free",
    "step-5-preview-free",
    "ling-3.1-flash-free",
];

const GITHUB_MODELS: &[&str] = &[
    "openai/gpt-4o",
    "openai/gpt-4o-mini",
    "openai/o4-mini",
    "meta/Llama-3.3-70B-Instruct",
    "meta/Llama-4-Maverick-17B-128E-Instruct-FP8",
    "mistral-ai/Mistral-Large-2411",
    "mistral-ai/Magistral-Small-2509",
    "deepseek/DeepSeek-R1",
    "deepseek/DeepSeek-V3-0324",
    "xai/grok-3",
    "xai/grok-3-mini",
    "microsoft/Phi-4",
    "ai21/jamba-1.5-large",
    "cohere/Cohere-command-a",
];

/// Static fallback catalogue. Clients may refresh it from the provider API.
pub fn model_catalog(name: &str) -> &'static [&'static str] {
    match name {
        "openai" => OPENAI_MODELS,
        "gemini" => GEMINI_MODELS,
        "mistral" => MISTRAL_MODELS,
        "groq" => GROQ_MODELS,
        "nvidia" => NVIDIA_MODELS,
        "openrouter" => OPENROUTER_MODELS,
        "openrouter-free" => OPENROUTER_FREE_MODELS,
        "opencode-zen" => OPENCODE_ZEN_MODELS,
        "github" => GITHUB_MODELS,
        _ => &[],
    }
}
