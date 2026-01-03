use std::collections::{HashMap, HashSet};
use tracing::{warn, info, debug};
use crate::mods::{Meta, Ordering};

/// Result of dependency resolution
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub order: Vec<String>,
    pub warnings: Vec<ResolutionWarning>,
}

/// Warnings generated during dependency resolution
#[derive(Debug, Clone)]
pub enum ResolutionWarning {
    CircularDependency { cycle: Vec<String> },
    MissingOptionalDependency { mod_id: String, missing: String },
    MissingRequiredDependency { mod_id: String, missing: String },
    ConflictingConstraints { mod_id: String, details: String },
}

/// Dependency graph for mod load order resolution
#[derive(Debug)]
struct DependencyGraph {
    // mod_id → mods that must load before this one
    before_deps: HashMap<String, Vec<String>>,

    // mod_id → mods that must load after this one
    after_deps: HashMap<String, Vec<String>>,

    // Track which dependencies are optional
    optional: HashMap<String, HashSet<String>>,
}

/// Manages dependency resolution for mod loading
pub struct DependencyResolver {
    mods: HashMap<String, Meta>,
}

impl DependencyResolver {
    /// Create a new resolver with available mods
    pub fn new(mods: HashMap<String, Meta>) -> Self {
        Self { mods }
    }

    /// Resolve mod load order based on dependencies and existing configuration
    ///
    /// # Arguments
    /// * `existing_order` - Current mod order from openzt.toml (user-controlled)
    /// * `disabled_mods` - Mods that should not be loaded (but kept in order)
    ///
    /// # Returns
    /// Resolution result with final order and any warnings
    ///
    /// Note: Disabled mods are kept in the order list but not processed for dependencies.
    /// They will be filtered out during actual mod loading.
    pub fn resolve_order(
        &self,
        existing_order: &[String],
        disabled_mods: &[String],
    ) -> ResolutionResult {
        let mut warnings = Vec::new();

        // Disabled mods should be kept in order but not processed
        let disabled_set: HashSet<_> = disabled_mods.iter().cloned().collect();

        // Identify new mods (not in existing order and not disabled)
        let existing_set: HashSet<_> = existing_order.iter().cloned().collect();
        let mut new_mods: Vec<_> = self.mods.keys()
            .filter(|id| !existing_set.contains(*id) && !disabled_set.contains(*id))
            .cloned()
            .collect();

        // Sort new mods alphabetically for deterministic processing
        new_mods.sort();

        // Keep existing order, only remove mods that no longer exist in discovered mods
        let valid_existing_order: Vec<_> = existing_order.iter()
            .filter(|id| self.mods.contains_key(*id))
            .cloned()
            .collect();

        if new_mods.is_empty() {
            // No new mods, return validated existing order (including disabled ones)
            return ResolutionResult {
                order: valid_existing_order,
                warnings,
            };
        }

        info!("Discovered {} new mod(s): {:?}", new_mods.len(), new_mods);

        // Build dependency graph only for enabled mods
        let enabled_mods: HashMap<_, _> = self.mods.iter()
            .filter(|(id, _)| !disabled_set.contains(*id))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let graph = self.build_dependency_graph(&enabled_mods);

        // Detect cycles in new mods
        let cycles = self.detect_cycles_in_subgraph(&graph, &new_mods);
        for cycle in &cycles {
            warn!("Circular dependency detected: {:?}", cycle);
            warnings.push(ResolutionWarning::CircularDependency {
                cycle: cycle.clone(),
            });
        }

        // Insert new mods into existing order
        let (final_order, insert_warnings) = self.insert_new_mods(
            &valid_existing_order,
            &new_mods,
            &graph,
            &enabled_mods,
            &cycles,
        );

        warnings.extend(insert_warnings);

        ResolutionResult {
            order: final_order,
            warnings,
        }
    }

    /// Build dependency graph from mod metadata
    fn build_dependency_graph(&self, mods: &HashMap<String, Meta>) -> DependencyGraph {
        let mut before_deps: HashMap<String, Vec<String>> = HashMap::new();
        let mut after_deps: HashMap<String, Vec<String>> = HashMap::new();
        let mut optional: HashMap<String, HashSet<String>> = HashMap::new();

        for (mod_id, meta) in mods {
            for dep in meta.dependencies() {
                let dep_mod_id = dep.mod_id();

                match dep.ordering() {
                    Ordering::Before => {
                        // This mod must load BEFORE dep_mod_id
                        after_deps.entry(mod_id.clone())
                            .or_default()
                            .push(dep_mod_id.clone());
                        before_deps.entry(dep_mod_id.clone())
                            .or_default()
                            .push(mod_id.clone());
                    }
                    Ordering::After => {
                        // This mod must load AFTER dep_mod_id
                        before_deps.entry(mod_id.clone())
                            .or_default()
                            .push(dep_mod_id.clone());
                        after_deps.entry(dep_mod_id.clone())
                            .or_default()
                            .push(mod_id.clone());
                    }
                    Ordering::None => {
                        // No ordering constraint, just track dependency existence
                    }
                }

                if *dep.optional() {
                    optional.entry(mod_id.clone())
                        .or_default()
                        .insert(dep_mod_id.clone());
                }
            }
        }

        DependencyGraph {
            before_deps,
            after_deps,
            optional,
        }
    }

    /// Detect cycles using Tarjan's strongly connected components algorithm
    /// Only considers the specified subset of mods
    fn detect_cycles_in_subgraph(
        &self,
        graph: &DependencyGraph,
        mods_to_check: &[String],
    ) -> Vec<Vec<String>> {
        let mod_set: HashSet<_> = mods_to_check.iter().cloned().collect();

        let mut state = TarjanState::new();
        let mut sccs = Vec::new();

        for mod_id in mods_to_check {
            if !state.indices.contains_key(mod_id) {
                self.tarjan_strongconnect(mod_id, graph, &mod_set, &mut state, &mut sccs);
            }
        }

        // Filter to only cycles (SCC size > 1)
        sccs.into_iter()
            .filter(|component| component.len() > 1)
            .collect()
    }

    /// Tarjan's algorithm recursive step
    fn tarjan_strongconnect(
        &self,
        mod_id: &str,
        graph: &DependencyGraph,
        mod_set: &HashSet<String>,
        state: &mut TarjanState,
        sccs: &mut Vec<Vec<String>>,
    ) {
        let index = state.index;
        state.indices.insert(mod_id.to_string(), index);
        state.lowlinks.insert(mod_id.to_string(), index);
        state.index += 1;
        state.stack.push(mod_id.to_string());
        state.on_stack.insert(mod_id.to_string());

        // Consider dependencies (nodes this mod depends on)
        if let Some(deps) = graph.before_deps.get(mod_id) {
            for dep in deps {
                // Only follow edges within the subgraph
                if !mod_set.contains(dep) {
                    continue;
                }

                if !state.indices.contains_key(dep) {
                    self.tarjan_strongconnect(dep, graph, mod_set, state, sccs);
                    let dep_lowlink = state.lowlinks[dep];
                    let current_lowlink = state.lowlinks.get_mut(mod_id).unwrap();
                    *current_lowlink = (*current_lowlink).min(dep_lowlink);
                } else if state.on_stack.contains(dep) {
                    let dep_index = state.indices[dep];
                    let current_lowlink = state.lowlinks.get_mut(mod_id).unwrap();
                    *current_lowlink = (*current_lowlink).min(dep_index);
                }
            }
        }

        // If mod_id is a root node, pop the stack and create an SCC
        if state.lowlinks[mod_id] == state.indices[mod_id] {
            let mut component = Vec::new();
            loop {
                let node = state.stack.pop().unwrap();
                state.on_stack.remove(&node);
                component.push(node.clone());
                if node == mod_id {
                    break;
                }
            }
            sccs.push(component);
        }
    }

    /// Insert new mods into existing order respecting dependencies
    fn insert_new_mods(
        &self,
        existing_order: &[String],
        new_mods: &[String],
        graph: &DependencyGraph,
        all_mods: &HashMap<String, Meta>,
        cycles: &[Vec<String>],
    ) -> (Vec<String>, Vec<ResolutionWarning>) {
        let mut order = existing_order.to_vec();
        let mut warnings = Vec::new();

        // Identify mods in cycles
        let cyclic_mods: HashSet<String> = cycles.iter()
            .flat_map(|cycle| cycle.iter().cloned())
            .collect();

        // Separate cyclic from acyclic new mods
        let mut acyclic_new_mods: Vec<_> = new_mods.iter()
            .filter(|id| !cyclic_mods.contains(*id))
            .cloned()
            .collect();

        let mut cyclic_new_mods: Vec<_> = new_mods.iter()
            .filter(|id| cyclic_mods.contains(*id))
            .cloned()
            .collect();

        // Sort both lists for deterministic behavior
        acyclic_new_mods.sort();
        cyclic_new_mods.sort();

        // Insert acyclic mods using topological constraints
        // Track the number of mods inserted at the beginning to maintain alphabetical order
        let mut insert_offset = 0;

        for mod_id in acyclic_new_mods {
            let (position, insert_warnings) = self.find_insert_position(
                &mod_id,
                &order,
                graph,
                all_mods,
            );
            warnings.extend(insert_warnings);

            // If inserting at the beginning with no constraints, use insert_offset
            // to maintain alphabetical order among independent mods
            let actual_position = if position == 0 && insert_offset > 0 {
                position + insert_offset
            } else {
                position
            };

            order.insert(actual_position, mod_id);

            // Track insertions at position 0 for alphabetical ordering
            if position == 0 {
                insert_offset += 1;
            }
        }

        // Append cyclic mods at the end (already sorted alphabetically)
        for mod_id in cyclic_new_mods {
            info!("Inserting cyclic mod '{}' at end of load order", mod_id);
            order.push(mod_id);
        }

        (order, warnings)
    }

    /// Find the best insertion position for a new mod
    fn find_insert_position(
        &self,
        mod_id: &str,
        current_order: &[String],
        graph: &DependencyGraph,
        all_mods: &HashMap<String, Meta>,
    ) -> (usize, Vec<ResolutionWarning>) {
        let mut warnings = Vec::new();
        let mut min_position = 0;
        let mut max_position = current_order.len();

        // Create position map for efficient lookup
        let position_map: HashMap<&String, usize> = current_order.iter()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();

        // Check "before" dependencies (mods this one depends on)
        if let Some(before_deps) = graph.before_deps.get(mod_id) {
            for dep in before_deps {
                if let Some(&pos) = position_map.get(dep) {
                    // This mod must load AFTER dep, so min_position is after dep
                    min_position = min_position.max(pos + 1);
                } else if !all_mods.contains_key(dep) {
                    // Dependency doesn't exist
                    let meta = &all_mods[mod_id];
                    let is_optional = meta.dependencies().iter()
                        .any(|d| d.mod_id() == dep && *d.optional());

                    if is_optional {
                        debug!("Optional dependency '{}' for mod '{}' not found", dep, mod_id);
                        warnings.push(ResolutionWarning::MissingOptionalDependency {
                            mod_id: mod_id.to_string(),
                            missing: dep.to_string(),
                        });
                    } else {
                        warn!("Required dependency '{}' for mod '{}' not found", dep, mod_id);
                        warnings.push(ResolutionWarning::MissingRequiredDependency {
                            mod_id: mod_id.to_string(),
                            missing: dep.to_string(),
                        });
                    }
                }
            }
        }

        // Check "after" dependencies (mods that depend on this one)
        if let Some(after_deps) = graph.after_deps.get(mod_id) {
            for dep in after_deps {
                if let Some(&pos) = position_map.get(dep) {
                    // This mod must load BEFORE dep, so max_position is before dep
                    max_position = max_position.min(pos);
                }
            }
        }

        // Determine final position
        let position = if min_position > max_position {
            // Conflicting constraints
            warn!(
                "Conflicting dependency constraints for mod '{}': must be in range [{}, {}]",
                mod_id, min_position, max_position
            );
            warnings.push(ResolutionWarning::ConflictingConstraints {
                mod_id: mod_id.to_string(),
                details: format!("Required position range [{}, {}] is invalid", min_position, max_position),
            });
            // Insert at end as fallback
            current_order.len()
        } else {
            // Insert at earliest valid position
            min_position
        };

        debug!("Inserting mod '{}' at position {}", mod_id, position);
        (position, warnings)
    }
}

/// State for Tarjan's algorithm
struct TarjanState {
    index: usize,
    stack: Vec<String>,
    indices: HashMap<String, usize>,
    lowlinks: HashMap<String, usize>,
    on_stack: HashSet<String>,
}

impl TarjanState {
    fn new() -> Self {
        Self {
            index: 0,
            stack: Vec::new(),
            indices: HashMap::new(),
            lowlinks: HashMap::new(),
            on_stack: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::{Meta, Dependencies, Version};
    use std::str::FromStr;

    /// Helper to create test metadata from TOML string
    fn create_test_meta(toml_str: &str) -> Meta {
        toml::from_str(toml_str).expect("Failed to parse test TOML")
    }

    #[test]
    fn test_empty_resolver() {
        let resolver = DependencyResolver::new(HashMap::new());
        let result = resolver.resolve_order(&[], &[]);
        assert!(result.order.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_no_new_mods() {
        let mut mods = HashMap::new();

        // Create two mods
        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
        "#);

        let meta_b = create_test_meta(r#"
            name = "Mod B"
            description = "Test mod B"
            authors = ["Test"]
            mod_id = "test.mod_b"
            version = "1.0.0"
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);
        mods.insert("test.mod_b".to_string(), meta_b);

        let resolver = DependencyResolver::new(mods);
        let existing = vec!["test.mod_a".to_string(), "test.mod_b".to_string()];
        let result = resolver.resolve_order(&existing, &[]);

        // Should return existing order unchanged
        assert_eq!(result.order, existing);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_simple_after_dependency() {
        let mut mods = HashMap::new();

        // Mod A has no dependencies
        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
        "#);

        // Mod B depends on A (must load AFTER A)
        let meta_b = create_test_meta(r#"
            name = "Mod B"
            description = "Test mod B"
            authors = ["Test"]
            mod_id = "test.mod_b"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.mod_a", name = "Mod A", ordering = "after" }
            ]
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);
        mods.insert("test.mod_b".to_string(), meta_b);

        let resolver = DependencyResolver::new(mods);
        let result = resolver.resolve_order(&[], &[]);

        // mod_a should load before mod_b
        assert_eq!(result.order, vec!["test.mod_a", "test.mod_b"]);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_simple_before_dependency() {
        let mut mods = HashMap::new();

        // Mod A must load BEFORE B
        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.mod_b", name = "Mod B", ordering = "before" }
            ]
        "#);

        // Mod B has no dependencies
        let meta_b = create_test_meta(r#"
            name = "Mod B"
            description = "Test mod B"
            authors = ["Test"]
            mod_id = "test.mod_b"
            version = "1.0.0"
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);
        mods.insert("test.mod_b".to_string(), meta_b);

        let resolver = DependencyResolver::new(mods);
        let result = resolver.resolve_order(&[], &[]);

        // mod_a should load before mod_b
        assert_eq!(result.order, vec!["test.mod_a", "test.mod_b"]);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut mods = HashMap::new();

        // A depends on B (after)
        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.mod_b", name = "Mod B", ordering = "after" }
            ]
        "#);

        // B depends on C (after)
        let meta_b = create_test_meta(r#"
            name = "Mod B"
            description = "Test mod B"
            authors = ["Test"]
            mod_id = "test.mod_b"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.mod_c", name = "Mod C", ordering = "after" }
            ]
        "#);

        // C depends on A (after) - creates cycle!
        let meta_c = create_test_meta(r#"
            name = "Mod C"
            description = "Test mod C"
            authors = ["Test"]
            mod_id = "test.mod_c"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.mod_a", name = "Mod A", ordering = "after" }
            ]
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);
        mods.insert("test.mod_b".to_string(), meta_b);
        mods.insert("test.mod_c".to_string(), meta_c);

        let resolver = DependencyResolver::new(mods);
        let result = resolver.resolve_order(&[], &[]);

        // Should detect cycle and place mods at end in alphabetical order
        assert_eq!(result.order.len(), 3);
        assert!(result.order.contains(&"test.mod_a".to_string()));
        assert!(result.order.contains(&"test.mod_b".to_string()));
        assert!(result.order.contains(&"test.mod_c".to_string()));

        // Should have circular dependency warning
        assert_eq!(result.warnings.len(), 1);
        match &result.warnings[0] {
            ResolutionWarning::CircularDependency { cycle } => {
                assert_eq!(cycle.len(), 3);
            }
            _ => panic!("Expected CircularDependency warning"),
        }
    }

    #[test]
    fn test_missing_optional_dependency() {
        let mut mods = HashMap::new();

        // Mod A has optional dependency on non-existent mod
        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.nonexistent", name = "Nonexistent", optional = true, ordering = "after" }
            ]
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);

        let resolver = DependencyResolver::new(mods);
        let result = resolver.resolve_order(&[], &[]);

        assert_eq!(result.order, vec!["test.mod_a"]);
        assert_eq!(result.warnings.len(), 1);
        match &result.warnings[0] {
            ResolutionWarning::MissingOptionalDependency { mod_id, missing } => {
                assert_eq!(mod_id, "test.mod_a");
                assert_eq!(missing, "test.nonexistent");
            }
            _ => panic!("Expected MissingOptionalDependency warning"),
        }
    }

    #[test]
    fn test_missing_required_dependency() {
        let mut mods = HashMap::new();

        // Mod A has required dependency on non-existent mod
        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.nonexistent", name = "Nonexistent", ordering = "after" }
            ]
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);

        let resolver = DependencyResolver::new(mods);
        let result = resolver.resolve_order(&[], &[]);

        assert_eq!(result.order, vec!["test.mod_a"]);
        assert_eq!(result.warnings.len(), 1);
        match &result.warnings[0] {
            ResolutionWarning::MissingRequiredDependency { mod_id, missing } => {
                assert_eq!(mod_id, "test.mod_a");
                assert_eq!(missing, "test.nonexistent");
            }
            _ => panic!("Expected MissingRequiredDependency warning"),
        }
    }

    #[test]
    fn test_insert_new_mod_into_existing_order() {
        let mut mods = HashMap::new();

        // Existing mods
        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
        "#);

        let meta_c = create_test_meta(r#"
            name = "Mod C"
            description = "Test mod C"
            authors = ["Test"]
            mod_id = "test.mod_c"
            version = "1.0.0"
        "#);

        // New mod B depends on A and comes before C
        let meta_b = create_test_meta(r#"
            name = "Mod B"
            description = "Test mod B"
            authors = ["Test"]
            mod_id = "test.mod_b"
            version = "1.0.0"
            dependencies = [
                { mod_id = "test.mod_a", name = "Mod A", ordering = "after" },
                { mod_id = "test.mod_c", name = "Mod C", ordering = "before" }
            ]
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);
        mods.insert("test.mod_b".to_string(), meta_b);
        mods.insert("test.mod_c".to_string(), meta_c);

        let resolver = DependencyResolver::new(mods);
        let existing = vec!["test.mod_a".to_string(), "test.mod_c".to_string()];
        let result = resolver.resolve_order(&existing, &[]);

        // B should be inserted between A and C
        assert_eq!(result.order, vec!["test.mod_a", "test.mod_b", "test.mod_c"]);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_disabled_mods_stay_in_order() {
        let mut mods = HashMap::new();

        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
        "#);

        let meta_b = create_test_meta(r#"
            name = "Mod B"
            description = "Test mod B"
            authors = ["Test"]
            mod_id = "test.mod_b"
            version = "1.0.0"
        "#);

        let meta_c = create_test_meta(r#"
            name = "Mod C"
            description = "Test mod C"
            authors = ["Test"]
            mod_id = "test.mod_c"
            version = "1.0.0"
        "#);

        mods.insert("test.mod_a".to_string(), meta_a);
        mods.insert("test.mod_b".to_string(), meta_b);
        mods.insert("test.mod_c".to_string(), meta_c);

        let resolver = DependencyResolver::new(mods);

        // B is in existing order but disabled - should stay in order
        // C is new and disabled - should NOT be added
        let existing = vec!["test.mod_a".to_string(), "test.mod_b".to_string()];
        let disabled = vec!["test.mod_b".to_string(), "test.mod_c".to_string()];
        let result = resolver.resolve_order(&existing, &disabled);

        // Both A and B should be in order (B is disabled but stays in order)
        // C is new and disabled, so not added
        assert_eq!(result.order, vec!["test.mod_a", "test.mod_b"]);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_alphabetical_ordering_for_independent_mods() {
        let mut mods = HashMap::new();

        // Three independent mods with no dependencies
        let meta_c = create_test_meta(r#"
            name = "Mod C"
            description = "Test mod C"
            authors = ["Test"]
            mod_id = "test.mod_c"
            version = "1.0.0"
        "#);

        let meta_a = create_test_meta(r#"
            name = "Mod A"
            description = "Test mod A"
            authors = ["Test"]
            mod_id = "test.mod_a"
            version = "1.0.0"
        "#);

        let meta_b = create_test_meta(r#"
            name = "Mod B"
            description = "Test mod B"
            authors = ["Test"]
            mod_id = "test.mod_b"
            version = "1.0.0"
        "#);

        mods.insert("test.mod_c".to_string(), meta_c);
        mods.insert("test.mod_a".to_string(), meta_a);
        mods.insert("test.mod_b".to_string(), meta_b);

        let resolver = DependencyResolver::new(mods);
        let result = resolver.resolve_order(&[], &[]);

        // Should be sorted alphabetically
        assert_eq!(result.order, vec!["test.mod_a", "test.mod_b", "test.mod_c"]);
        assert!(result.warnings.is_empty());
    }
}
