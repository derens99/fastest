//! Smart Test Dependency Tracking
//!
//! Fast dependency analysis using petgraph for optimal execution order

use anyhow::Result;
use petgraph::{algo::toposort, Graph, Direction};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, Query, QueryCursor};

use super::AdvancedConfig;

/// Smart dependency tracker using graph algorithms
pub struct DependencyTracker {
    #[allow(dead_code)]
    config: AdvancedConfig,
    dependency_graph: Graph<String, DependencyType>,
    node_indices: HashMap<String, petgraph::graph::NodeIndex>,
    cache_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    ImportDependency,
    FixtureDependency,
    TestOrderDependency,
    ClassInheritance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDependency {
    pub test_id: String,
    pub depends_on: Vec<String>,
    pub dependency_type: DependencyType,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
}

impl DependencyTracker {
    pub fn new(config: &AdvancedConfig) -> Result<Self> {
        let cache_file = config.cache_dir.join("dependencies.json");
        
        Ok(Self {
            config: config.clone(),
            dependency_graph: Graph::new(),
            node_indices: HashMap::new(),
            cache_file,
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Load cached dependencies
        self.load_cache().await?;
        
        tracing::info!("Dependency tracker initialized with {} nodes", 
                      self.dependency_graph.node_count());
        Ok(())
    }

    /// Analyze test dependencies using tree-sitter for fast parsing
    pub async fn analyze_dependencies(&mut self, test_files: &[String]) -> Result<()> {
        tracing::info!("Analyzing dependencies for {} files", test_files.len());

        // Parse all files in parallel for speed
        let parse_futures: Vec<_> = test_files
            .iter()
            .map(|file| self.analyze_file_dependencies(file))
            .collect();

        let results = futures::future::join_all(parse_futures).await;
        
        for result in results {
            if let Ok(dependencies) = result {
                for dep in dependencies {
                    self.add_dependency(dep).await?;
                }
            }
        }

        // Save cache
        self.save_cache().await?;
        
        Ok(())
    }

    /// Fast file dependency analysis using tree-sitter
    async fn analyze_file_dependencies(&self, file_path: &str) -> Result<Vec<TestDependency>> {
        let mut dependencies = Vec::new();
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Ok(dependencies);
        }

        let content = std::fs::read_to_string(path)?;
        
        // Use tree-sitter for fast Python parsing
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser.set_language(&language).map_err(|e| anyhow::anyhow!("Parser error: {:?}", e))?;
        
        let tree = parser.parse(&content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python file"))?;

        // Analyze imports
        dependencies.extend(self.analyze_imports(&content, &tree, file_path).await?);
        
        // Analyze fixtures
        dependencies.extend(self.analyze_fixtures(&content, &tree, file_path).await?);
        
        // Analyze class inheritance
        dependencies.extend(self.analyze_inheritance(&content, &tree, file_path).await?);

        Ok(dependencies)
    }

    /// Analyze import dependencies
    async fn analyze_imports(
        &self,
        content: &str,
        tree: &tree_sitter::Tree,
        file_path: &str,
    ) -> Result<Vec<TestDependency>> {
        let mut dependencies = Vec::new();
        
        // Query for import statements - simplified pattern
        let import_query = "(import_statement) @import";
        let query = Query::new(&tree_sitter_python::language(), import_query)
            .map_err(|e| anyhow::anyhow!("Query error: {:?}", e))?;

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        for m in matches {
            for capture in m.captures {
                let import_name = &content[capture.node.byte_range()];
                
                dependencies.push(TestDependency {
                    test_id: file_path.to_string(),
                    depends_on: vec![import_name.to_string()],
                    dependency_type: DependencyType::ImportDependency,
                    discovered_at: chrono::Utc::now(),
                });
            }
        }

        Ok(dependencies)
    }

    /// Analyze fixture dependencies
    async fn analyze_fixtures(
        &self,
        content: &str,
        tree: &tree_sitter::Tree,
        file_path: &str,
    ) -> Result<Vec<TestDependency>> {
        let mut dependencies = Vec::new();
        
        // Query for function definitions - simplified
        let func_query = "(function_definition) @func";
        let query = Query::new(&tree_sitter_python::language(), func_query)
            .map_err(|e| anyhow::anyhow!("Query error: {:?}", e))?;

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        for m in matches {
            let mut test_name = String::new();
            let mut fixtures = Vec::new();

            for capture in m.captures {
                match query.capture_names()[capture.index as usize] {
                    "test_name" => {
                        test_name = content[capture.node.byte_range()].to_string();
                    }
                    "param" => {
                        let param_name = &content[capture.node.byte_range()];
                        // Common pytest fixtures
                        if ["tmp_path", "tmpdir", "monkeypatch", "capfd", "capsys"]
                            .contains(&param_name)
                        {
                            fixtures.push(param_name.to_string());
                        }
                    }
                    _ => {}
                }
            }

            if !test_name.is_empty() && !fixtures.is_empty() {
                dependencies.push(TestDependency {
                    test_id: format!("{}::{}", file_path, test_name),
                    depends_on: fixtures,
                    dependency_type: DependencyType::FixtureDependency,
                    discovered_at: chrono::Utc::now(),
                });
            }
        }

        Ok(dependencies)
    }

    /// Analyze class inheritance dependencies
    async fn analyze_inheritance(
        &self,
        content: &str,
        tree: &tree_sitter::Tree,
        file_path: &str,
    ) -> Result<Vec<TestDependency>> {
        let mut dependencies = Vec::new();
        
        // Query for class definitions with inheritance
        let query = Query::new(
            &tree_sitter_python::language(),
            r#"
            (class_definition
              name: (identifier) @class_name
              superclasses: (argument_list
                (identifier) @parent
              )
            )
            "#,
        )?;

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        for m in matches {
            let mut class_name = String::new();
            let mut parents = Vec::new();

            for capture in m.captures {
                match query.capture_names()[capture.index as usize] {
                    "class_name" => {
                        class_name = content[capture.node.byte_range()].to_string();
                    }
                    "parent" => {
                        parents.push(content[capture.node.byte_range()].to_string());
                    }
                    _ => {}
                }
            }

            if !class_name.is_empty() && !parents.is_empty() {
                dependencies.push(TestDependency {
                    test_id: format!("{}::{}", file_path, class_name),
                    depends_on: parents,
                    dependency_type: DependencyType::ClassInheritance,
                    discovered_at: chrono::Utc::now(),
                });
            }
        }

        Ok(dependencies)
    }

    /// Add dependency to graph
    async fn add_dependency(&mut self, dependency: TestDependency) -> Result<()> {
        // Get or create node for test
        let test_node = self.get_or_create_node(&dependency.test_id);
        
        // Add dependencies
        for dep in &dependency.depends_on {
            let dep_node = self.get_or_create_node(dep);
            
            // Add edge from dependency to test
            self.dependency_graph.add_edge(dep_node, test_node, dependency.dependency_type.clone());
        }

        Ok(())
    }

    /// Get or create node in graph
    fn get_or_create_node(&mut self, test_id: &str) -> petgraph::graph::NodeIndex {
        if let Some(&index) = self.node_indices.get(test_id) {
            index
        } else {
            let index = self.dependency_graph.add_node(test_id.to_string());
            self.node_indices.insert(test_id.to_string(), index);
            index
        }
    }

    /// Get optimal execution order using topological sort
    pub async fn get_execution_order(&self, test_ids: &[String]) -> Result<Vec<String>> {
        // Filter graph to only include requested tests
        let mut subgraph = Graph::new();
        let mut sub_nodes = HashMap::new();
        
        // Add nodes for requested tests
        for test_id in test_ids {
            if self.node_indices.contains_key(test_id) {
                let index = subgraph.add_node(test_id.clone());
                sub_nodes.insert(test_id.clone(), index);
            }
        }

        // Add edges between requested tests
        for test_id in test_ids {
            if let Some(&original_index) = self.node_indices.get(test_id) {
                if let Some(&sub_index) = sub_nodes.get(test_id) {
                    // Add dependencies that exist in our subgraph
                    for edge in self.dependency_graph.edges_directed(original_index, Direction::Incoming) {
                        let dep_node = edge.source();
                        let dep_test = &self.dependency_graph[dep_node];
                        if let Some(&dep_sub_index) = sub_nodes.get(dep_test) {
                            subgraph.add_edge(dep_sub_index, sub_index, ());
                        }
                    }
                }
            }
        }

        // Perform topological sort for optimal order
        match toposort(&subgraph, None) {
            Ok(sorted_indices) => {
                let ordered_tests: Vec<String> = sorted_indices
                    .into_iter()
                    .map(|index| subgraph[index].clone())
                    .collect();
                
                tracing::debug!("Computed execution order for {} tests", ordered_tests.len());
                Ok(ordered_tests)
            }
            Err(_) => {
                // Cycle detected, return original order
                tracing::warn!("Dependency cycle detected, using original order");
                Ok(test_ids.to_vec())
            }
        }
    }

    /// Check if test should be skipped due to failed dependencies
    pub async fn should_skip_test(&self, test_id: &str, failed_tests: &HashSet<String>) -> bool {
        if let Some(&node_index) = self.node_indices.get(test_id) {
            // Check if any dependency failed
            for edge in self.dependency_graph.edges_directed(node_index, Direction::Incoming) {
                let dep_node = edge.source();
                let dep_test = &self.dependency_graph[dep_node];
                if failed_tests.contains(dep_test) {
                    tracing::info!("Skipping {} due to failed dependency: {}", test_id, dep_test);
                    return true;
                }
            }
        }
        false
    }

    /// Load cached dependencies
    async fn load_cache(&mut self) -> Result<()> {
        if !self.cache_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.cache_file)?;
        let dependencies: Vec<TestDependency> = serde_json::from_str(&content)?;

        for dep in dependencies {
            self.add_dependency(dep).await?;
        }

        tracing::debug!("Loaded {} dependencies from cache", self.dependency_graph.edge_count());
        Ok(())
    }

    /// Save dependencies to cache
    async fn save_cache(&self) -> Result<()> {
        let mut dependencies = Vec::new();

        // Extract dependencies from graph
        for edge in self.dependency_graph.edge_references() {
            let test_id = &self.dependency_graph[edge.target()];
            let dep_id = &self.dependency_graph[edge.source()];
            
            dependencies.push(TestDependency {
                test_id: test_id.clone(),
                depends_on: vec![dep_id.clone()],
                dependency_type: edge.weight().clone(),
                discovered_at: chrono::Utc::now(),
            });
        }

        let content = serde_json::to_string_pretty(&dependencies)?;
        std::fs::write(&self.cache_file, content)?;

        tracing::debug!("Saved {} dependencies to cache", dependencies.len());
        Ok(())
    }

    /// Get dependency statistics
    pub fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("total_nodes".to_string(), 
                    serde_json::Value::Number(self.dependency_graph.node_count().into()));
        stats.insert("total_edges".to_string(), 
                    serde_json::Value::Number(self.dependency_graph.edge_count().into()));
        
        // Count dependency types
        let mut type_counts = HashMap::new();
        for edge in self.dependency_graph.edge_references() {
            let type_name = format!("{:?}", edge.weight());
            *type_counts.entry(type_name).or_insert(0) += 1;
        }
        
        stats.insert("dependency_types".to_string(), 
                    serde_json::to_value(type_counts).unwrap());
        
        stats
    }
}