use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use context_server::{ComputedPrompt, Prompt, PromptDelegate, PromptExecutor};
use parking_lot::RwLock;
use serde_json::Value;

#[derive(Default)]
pub struct PromptRegistry(RwLock<HashMap<String, Arc<dyn PromptExecutor>>>);

impl PromptRegistry {
    #[allow(unused)]
    pub fn register(&self, prompt: Arc<dyn PromptExecutor>) {
        self.0.write().insert(prompt.name().to_string(), prompt);
    }

    pub fn list_prompts(&self) -> Vec<Prompt> {
        self.0.read().values().map(|p| p.to_prompt()).collect()
    }

    pub async fn compute_prompt(
        &self,
        prompt: &str,
        arguments: Option<Value>,
    ) -> Result<ComputedPrompt> {
        let prompt = self
            .0
            .read()
            .get(prompt)
            .ok_or_else(|| anyhow!("Prompt not found: {}", prompt))?
            .clone();

        prompt.compute(arguments).await
    }
}

#[async_trait]
impl PromptDelegate for PromptRegistry {
    async fn list(&self) -> Result<Vec<Prompt>> {
        Ok(self.list_prompts())
    }

    async fn compute(&self, prompt: &str, arguments: Option<Value>) -> Result<ComputedPrompt> {
        self.compute_prompt(prompt, arguments).await
    }
}
