use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq)]
pub enum GitRef {
    /// Default branch (usually main or master)
    Default,
    /// A specific branch name
    Branch(String),
    /// A specific tag name
    Tag(String),
    /// A specific commit SHA
    Commit(String),
}

impl Default for GitRef {
    fn default() -> Self {
        Self::Default
    }
}

#[async_trait]
pub trait GitProvider: Send + Sync {
    /// Returns the name of the provider (e.g., "github", "gitlab")
    fn name(&self) -> &str;

    /// Process a repository and return the tree structure
    async fn get_tree_structure(
        &self,
        repo_path: &str,
        git_ref: Option<GitRef>,
        exclude_patterns: Vec<String>,
        include_patterns: Vec<String>,
    ) -> Result<String>;
    
    /// Retrieve file content from a repository
    async fn get_file_content(
        &self,
        repo_path: &str,
        file_path: &str,
        git_ref: Option<GitRef>,
    ) -> Result<String>;
}

/// Represents a file or directory in a repository
#[derive(Debug, Clone)]
pub struct RepoItem {
    pub name: String,
    pub path: String,
    pub item_type: RepoItemType, // file or directory
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum RepoItemType {
    File,
    Directory,
}

/// Represents a node in the repository tree structure
#[derive(Debug, Clone)]
pub struct RepoNode {
    pub name: String,
    pub node_type: RepoItemType,
    pub size: u64,
    pub children: Vec<RepoNode>,
    pub file_count: usize,
    pub dir_count: usize,
}

/// Helper function to create a formatted tree structure
pub fn create_tree_structure(node: &RepoNode, prefix: &str, is_last: bool) -> String {
    let mut result = String::new();
    let marker = if is_last { "└── " } else { "├── " };

    // Add the current node with appropriate prefix
    result.push_str(&format!("{}{}{}\n", prefix, marker, node.name));

    // Calculate the prefix for children
    let child_prefix = if is_last { "    " } else { "│   " };

    // Add all children recursively
    for (i, child) in node.children.iter().enumerate() {
        result.push_str(&create_tree_structure(
            child,
            &format!("{}{}", prefix, child_prefix),
            i == node.children.len() - 1,
        ));
    }

    result
}
