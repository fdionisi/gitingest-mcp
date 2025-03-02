use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use context_server::{Tool, ToolContent, ToolDelegate, ToolExecutor};
use parking_lot::RwLock;
use serde_json::Value;

#[derive(Default)]
pub struct ToolRegistry(RwLock<HashMap<String, Arc<dyn ToolExecutor>>>);

impl ToolRegistry {
    pub fn register(&self, tool: Arc<dyn ToolExecutor>) {
        self.0.write().insert(tool.to_tool().name.clone(), tool);
    }

    pub fn list(&self) -> Vec<Tool> {
        self.0.read().values().map(|t| t.to_tool()).collect()
    }

    pub async fn execute(&self, tool: &str, arguments: Option<Value>) -> Result<Vec<ToolContent>> {
        let tool = self
            .0
            .read()
            .get(tool)
            .ok_or_else(|| anyhow!("Tool not found: {}", tool))?
            .clone();

        tool.execute(arguments).await
    }
}

#[async_trait]
impl ToolDelegate for ToolRegistry {
    async fn list(&self) -> Result<Vec<Tool>> {
        Ok(self.list())
    }

    async fn execute(&self, tool: &str, arguments: Option<Value>) -> Result<Vec<ToolContent>> {
        self.execute(tool, arguments).await
    }
}
