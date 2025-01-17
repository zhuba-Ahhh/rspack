use rspack_error::Result;

use super::{build::BuildTask, MakeTaskContext};
use crate::{
  module_graph::{ModuleGraph, ModuleGraphModule},
  utils::task_loop::{Task, TaskResult, TaskType},
  DependencyId, Module, ModuleIdentifier, ModuleProfile,
};

#[derive(Debug)]
pub struct AddTask {
  pub original_module_identifier: Option<ModuleIdentifier>,
  pub module: Box<dyn Module>,
  pub module_graph_module: Box<ModuleGraphModule>,
  pub dependencies: Vec<DependencyId>,
  pub is_entry: bool,
  pub current_profile: Option<Box<ModuleProfile>>,
}

impl Task<MakeTaskContext> for AddTask {
  fn get_task_type(&self) -> TaskType {
    TaskType::Sync
  }
  fn sync_run(self: Box<Self>, context: &mut MakeTaskContext) -> TaskResult<MakeTaskContext> {
    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_integration_start();
    }

    let module_identifier = self.module.identifier();
    let is_new_treeshaking = context.compiler_options.is_new_tree_shaking();
    let module_graph = &mut MakeTaskContext::get_module_graph(&mut context.module_graph_partial);

    if self.module.as_self_module().is_some() {
      let issuer = self
        .module_graph_module
        .get_issuer()
        .identifier()
        .expect("self module should have issuer");

      set_resolved_module(
        module_graph,
        self.original_module_identifier,
        self.dependencies,
        *issuer,
        is_new_treeshaking,
      )?;

      // reused module
      return Ok(vec![]);
    }

    if module_graph
      .module_graph_module_by_identifier(&module_identifier)
      .is_some()
    {
      set_resolved_module(
        module_graph,
        self.original_module_identifier,
        self.dependencies,
        module_identifier,
        is_new_treeshaking,
      )?;

      // reused module
      return Ok(vec![]);
    }

    module_graph.add_module_graph_module(*self.module_graph_module);

    set_resolved_module(
      module_graph,
      self.original_module_identifier,
      self.dependencies,
      module_identifier,
      is_new_treeshaking,
    )?;

    if self.is_entry {
      context.entry_module_identifiers.insert(module_identifier);
    }

    if let Some(current_profile) = &self.current_profile {
      current_profile.mark_integration_end();
    }

    tracing::trace!("Module added: {}", self.module.identifier());

    Ok(vec![Box::new(BuildTask {
      module: self.module,
      current_profile: self.current_profile,
      resolver_factory: context.resolver_factory.clone(),
      compiler_options: context.compiler_options.clone(),
      plugin_driver: context.plugin_driver.clone(),
      cache: context.cache.clone(),
    })])
  }
}

fn set_resolved_module(
  module_graph: &mut ModuleGraph,
  original_module_identifier: Option<ModuleIdentifier>,
  dependencies: Vec<DependencyId>,
  module_identifier: ModuleIdentifier,
  // TODO: removed when new treeshaking is stable
  is_new_treeshaking: bool,
) -> Result<()> {
  for dependency in dependencies {
    module_graph.set_resolved_module(
      original_module_identifier,
      dependency,
      module_identifier,
      is_new_treeshaking,
    )?;
  }
  Ok(())
}
