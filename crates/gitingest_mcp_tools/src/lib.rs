mod github;
mod gitlab;
mod ignore_patterns;
mod provider;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use context_server::{Tool, ToolContent, ToolExecutor};
use futures::future::join_all;
use github::GitHubProvider;
use gitlab::GitLabProvider;
use http_client::HttpClient;
use provider::{GitProvider, GitRef};
use serde_json::{Value, json};

pub struct RepositoryRead {
    providers: Vec<Box<dyn GitProvider>>,
}

impl RepositoryRead {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        let providers: Vec<Box<dyn GitProvider>> = vec![
            Box::new(GitHubProvider::new(http_client.clone())),
            Box::new(GitLabProvider::new(http_client.clone())),
        ];

        Self { providers }
    }

    fn get_provider(&self, provider_name: &str) -> Option<&dyn GitProvider> {
        self.providers
            .iter()
            .find(|p| p.name() == provider_name)
            .map(|p| p.as_ref())
    }

    fn get_supported_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }

    fn parse_git_ref(&self, ref_str: &str) -> GitRef {
        if ref_str.is_empty() {
            return GitRef::Default;
        }

        let parts: Vec<&str> = ref_str.split(':').collect();
        if parts.len() != 2 {
            return GitRef::Branch(ref_str.to_string());
        }

        match parts[0] {
            "tag" => GitRef::Tag(parts[1].to_string()),
            "commit" => GitRef::Commit(parts[1].to_string()),
            "branch" => GitRef::Branch(parts[1].to_string()),
            _ => GitRef::Branch(ref_str.to_string()),
        }
    }
}

#[async_trait]
impl ToolExecutor for RepositoryRead {
    async fn execute(&self, arguments: Option<Value>) -> Result<Vec<ToolContent>> {
        let args = arguments.ok_or_else(|| anyhow!("Missing arguments"))?;

        // Extract the repository identifier (e.g., "github:owner/repo")
        let repo_identifier = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid repository identifier"))?;

        // Parse the "gitprovider:username/reponame" format
        let parts: Vec<&str> = repo_identifier.split(':').collect();
        if parts.len() != 2 || !parts[1].contains('/') {
            return Err(anyhow!(
                "Invalid repository format. Expected 'gitprovider:username/reponame'"
            ));
        }

        let git_provider = parts[0];
        let repo_path = parts[1];

        // Get the file path
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid file path"))?;

        // Get the provider implementation
        let provider = self.get_provider(git_provider).ok_or_else(|| {
            let supported = self.get_supported_providers().join(", ");
            anyhow!(
                "Git provider '{}' is not supported. Supported providers: {}",
                git_provider,
                supported
            )
        })?;

        // Parse git reference (branch, tag, commit)
        let git_ref = args
            .get("git_ref")
            .and_then(|v| v.as_str())
            .map(|s| self.parse_git_ref(s));

        // Get file content
        match provider
            .get_file_content(repo_path, file_path, git_ref)
            .await
        {
            Ok(content) => {
                // Determine if we need to wrap the content in a code block
                let is_code = file_path.ends_with(".rs")
                    || file_path.ends_with(".js")
                    || file_path.ends_with(".py")
                    || file_path.ends_with(".go")
                    || file_path.ends_with(".java")
                    || file_path.ends_with(".c")
                    || file_path.ends_with(".cpp")
                    || file_path.ends_with(".h")
                    || file_path.ends_with(".ts")
                    || file_path.ends_with(".sh")
                    || file_path.ends_with(".json")
                    || file_path.ends_with(".yaml")
                    || file_path.ends_with(".yml")
                    || file_path.ends_with(".toml")
                    || file_path.ends_with(".md");

                let formatted_content = if is_code {
                    // Get file extension for syntax highlighting
                    let extension = file_path.split('.').last().unwrap_or("");
                    format!("```{}\n{}\n```", extension, content)
                } else {
                    content
                };

                Ok(vec![ToolContent::Text {
                    text: formatted_content,
                }])
            }
            Err(e) => Err(anyhow!("Error getting file content: {}", e)),
        }
    }

    fn to_tool(&self) -> Tool {
        let providers = self.get_supported_providers().join(", ");

        Tool {
            name: "repository_read".into(),
            description: Some(format!(
                "Read file content from a Git repository. Supported providers: {}",
                providers
            )),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Repository identifier in format 'gitprovider:username/reponame' (e.g., 'github:rust-lang/rust')"
                    },
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file within the repository to read"
                    },
                    "git_ref": {
                        "type": "string",
                        "description": "Optional git reference: branch name, 'tag:name', or 'commit:sha'. Default: main branch"
                    }
                },
                "required": ["repo", "file_path"]
            }),
        }
    }
}

pub struct RepositoryTreeView {
    providers: Vec<Box<dyn GitProvider>>,
}

impl RepositoryTreeView {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        let providers: Vec<Box<dyn GitProvider>> = vec![
            Box::new(GitHubProvider::new(http_client.clone())),
            Box::new(GitLabProvider::new(http_client.clone())),
        ];

        Self { providers }
    }

    fn get_provider(&self, provider_name: &str) -> Option<&dyn GitProvider> {
        self.providers
            .iter()
            .find(|p| p.name() == provider_name)
            .map(|p| p.as_ref())
    }

    fn get_supported_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }

    fn parse_git_ref(&self, ref_str: &str) -> GitRef {
        if ref_str.is_empty() {
            return GitRef::Default;
        }

        let parts: Vec<&str> = ref_str.split(':').collect();
        if parts.len() != 2 {
            return GitRef::Branch(ref_str.to_string());
        }

        match parts[0] {
            "tag" => GitRef::Tag(parts[1].to_string()),
            "commit" => GitRef::Commit(parts[1].to_string()),
            "branch" => GitRef::Branch(parts[1].to_string()),
            _ => GitRef::Branch(ref_str.to_string()),
        }
    }
}

pub struct FindRepositories {
    providers: Vec<Box<dyn GitProvider>>,
}

impl FindRepositories {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        let providers: Vec<Box<dyn GitProvider>> = vec![
            Box::new(GitHubProvider::new(http_client.clone())),
            Box::new(GitLabProvider::new(http_client.clone())),
        ];

        Self { providers }
    }

    fn get_supported_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }
}

#[async_trait]
impl ToolExecutor for FindRepositories {
    async fn execute(&self, arguments: Option<Value>) -> Result<Vec<ToolContent>> {
        let args = arguments.ok_or_else(|| anyhow!("Missing arguments"))?;

        // Extract the query for repository search
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid search query"))?;

        // Get limit (optional)
        let limit = args.get("limit").and_then(|v| {
            // Handle limit as either string or number
            if let Some(str_val) = v.as_str() {
                str_val.parse::<usize>().ok()
            } else {
                None
            }
        });

        let mut results = join_all(
            self.providers
                .iter()
                .map(|p| p.find_repositories(query, limit)),
        )
        .await
        .into_iter()
        .filter_map(|result| result.ok())
        .flatten()
        .collect::<Vec<_>>();

        // If no results were found
        if results.is_empty() {
            return Ok(vec![ToolContent::Text {
                text: format!("No repositories found matching query: \"{}\"", query),
            }]);
        }

        // Sort results by star count (most popular first)
        results.sort_by(|a, b| b.stargazers_count.cmp(&a.stargazers_count));

        // Format results in a simpler format
        let mut formatted_output = String::new();
        formatted_output.push_str(&format!("Search results for: \"{}\"\n\n", query));

        for repo in results.iter() {
            let description = repo.description.as_deref().unwrap_or("").trim();

            formatted_output.push_str(&format!(
                "- {}:{} ⭐️{}\n  {}\n\n",
                repo.provider, repo.full_name, repo.stargazers_count, description
            ));
        }

        Ok(vec![ToolContent::Text {
            text: formatted_output,
        }])
    }

    fn to_tool(&self) -> Tool {
        let providers = self.get_supported_providers().join(", ");

        Tool {
            name: "find_repositories".into(),
            description: Some(format!(
                "Find code repositories matching a search query. Supported providers: {}",
                providers
            )),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query to find repositories (e.g., 'lang:rust web framework', 'machine learning lang:python')"
                    },
                    "limit": {
                        "type": "string",
                        "description": "Optional maximum number of results to return per each provider"
                    }
                },
                "required": ["query"]
            }),
        }
    }
}

#[async_trait]
impl ToolExecutor for RepositoryTreeView {
    async fn execute(&self, arguments: Option<Value>) -> Result<Vec<ToolContent>> {
        let args = arguments.ok_or_else(|| anyhow!("Missing arguments"))?;

        let repo_identifier = args
            .get("repo")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid repository identifier"))?;

        // Parse the "gitprovider:username/reponame" format
        let parts: Vec<&str> = repo_identifier.split(':').collect();
        if parts.len() != 2 || !parts[1].contains('/') {
            return Err(anyhow!(
                "Invalid repository format. Expected 'gitprovider:username/reponame'"
            ));
        }

        let git_provider = parts[0];
        let repo_path = parts[1];

        // Get the provider implementation
        let provider = self.get_provider(git_provider).ok_or_else(|| {
            let supported = self.get_supported_providers().join(", ");
            anyhow!(
                "Git provider '{}' is not supported. Supported providers: {}",
                git_provider,
                supported
            )
        })?;

        // Parse git reference (branch, tag, commit)
        let git_ref = args
            .get("git_ref")
            .and_then(|v| v.as_str())
            .map(|s| self.parse_git_ref(s));

        let exclude_patterns = args
            .get("exclude_patterns")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let include_patterns = args
            .get("include_patterns")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Get tree structure directly from the provider
        match provider
            .get_tree_structure(repo_path, git_ref, exclude_patterns, include_patterns)
            .await
        {
            Ok(tree_structure) => {
                // Return the tree structure as text wrapped in code block for better formatting
                Ok(vec![ToolContent::Text {
                    text: format!("```\n{}\n```", tree_structure),
                }])
            }
            Err(e) => Err(anyhow!("Error getting repository tree structure: {}", e)),
        }
    }

    fn to_tool(&self) -> Tool {
        let providers = self.get_supported_providers().join(", ");

        Tool {
            name: "repository_tree_view".into(),
            description: Some(format!(
                "View the file structure of a Git repository recursively. Supported providers: {}",
                providers
            )),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Repository identifier in format 'gitprovider:username/reponame' (e.g., 'github:rust-lang/rust')"
                    },
                    "git_ref": {
                        "type": "string",
                        "description": "Optional git reference: branch name, 'tag:name', or 'commit:sha'. Default: main branch"
                    },
                    "exclude_patterns": {
                        "type": "string",
                        "description": "Optional comma-separated list of patterns to exclude"
                    },
                    "include_patterns": {
                        "type": "string",
                        "description": "Optional comma-separated list of patterns to include"
                    }
                },
                "required": ["repo"]
            }),
        }
    }
}
